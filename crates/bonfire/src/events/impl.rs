use std::collections::HashSet;

use revolt_database::{
    events::client::EventV1, util::permissions::DatabasePermissionQuery, Channel, Database, Member,
    MemberCompositeKey, Presence, RelationshipStatus,
};
use revolt_models::v0;
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_presence::filter_online;
use revolt_result::Result;
use revolt_voice::{delete_voice_state, get_voice_channel_members, get_voice_state};


use super::state::{Cache, State};

/// Cache Manager
impl Cache {
    /// Check whether the current user can view a channel
    pub async fn can_view_channel(&self, db: &Database, channel: &Channel) -> bool {
        match &channel {
            Channel::TextChannel { server, .. } | Channel::VoiceChannel { server, .. } => {
                let member = self.members.get(server);
                let server = self.servers.get(server);
                let mut query =
                    DatabasePermissionQuery::new(db, self.users.get(&self.user_id).unwrap())
                        .channel(&channel);
                // let mut perms = perms(self.users.get(&self.user_id).unwrap()).channel(channel);

                if let Some(member) = member {
                    query = query.member(&member);
                }

                if let Some(server) = server {
                    query = query.server(server);
                }

                calculate_channel_permissions(&mut query)
                    .await
                    .has_channel_permission(ChannelPermission::ViewChannel)
            }
            _ => true,
        }
    }

    /// Filter a given vector of channels to only include the ones we can access
    pub async fn filter_accessible_channels(
        &self,
        db: &Database,
        channels: Vec<Channel>,
    ) -> Vec<Channel> {
        let mut viewable_channels = vec![];
        for channel in channels {
            if self.can_view_channel(db, &channel).await {
                viewable_channels.push(channel);
            }
        }

        viewable_channels
    }

    /// Check whether we can subscribe to another user
    pub fn can_subscribe_to_user(&self, user_id: &str) -> bool {
        if let Some(user) = self.users.get(&self.user_id) {
            match user.relationship_with(user_id) {
                RelationshipStatus::Friend
                | RelationshipStatus::Incoming
                | RelationshipStatus::Outgoing
                | RelationshipStatus::User => true,
                _ => {
                    let user_id = &user_id.to_string();
                    for channel in self.channels.values() {
                        match channel {
                            Channel::DirectMessage { recipients, .. }
                            | Channel::Group { recipients, .. } => {
                                if recipients.contains(user_id) {
                                    return true;
                                }
                            }
                            _ => {}
                        }
                    }

                    false
                }
            }
        } else {
            false
        }
    }
}

/// State Manager
impl State {
    /// Generate a Ready packet for the current user
    pub async fn generate_ready_payload(&mut self, db: &Database) -> Result<EventV1> {
        let user = self.clone_user();

        // Find all relationships to the user.
        let mut user_ids: HashSet<String> = user
            .relations
            .as_ref()
            .map(|arr| arr.iter().map(|x| x.id.to_string()).collect())
            .unwrap_or_default();

        // Fetch all memberships with their corresponding servers.
        let members: Vec<Member> = db.fetch_all_memberships(&user.id).await?;
        self.cache.members = members
            .iter()
            .cloned()
            .map(|x| (x.id.server.clone(), x))
            .collect();

        let server_ids: Vec<String> = members.iter().map(|x| x.id.server.clone()).collect();
        let servers = db.fetch_servers(&server_ids).await?;
        self.cache.servers = servers.iter().cloned().map(|x| (x.id.clone(), x)).collect();

        // Collect channel ids from servers.
        let mut channel_ids = vec![];
        for server in &servers {
            channel_ids.append(&mut server.channels.clone());
        }

        // Fetch DMs and server channels.
        let mut channels = db.find_direct_messages(&user.id).await?;
        channels.append(&mut db.fetch_channels(&channel_ids).await?);

        // Filter server channels by permission.
        let channels = self.cache.filter_accessible_channels(db, channels).await;

        // Append known user IDs from DMs.
        for channel in &channels {
            match channel {
                Channel::DirectMessage { recipients, .. } | Channel::Group { recipients, .. } => {
                    user_ids.extend(&mut recipients.clone().into_iter());
                }
                _ => {}
            }
        }

        // Fetch presence data for known users.
        let online_ids = filter_online(&user_ids.iter().cloned().collect::<Vec<String>>()).await;

        // Fetch user data.
        let users = db
            .fetch_users(
                &user_ids
                    .into_iter()
                    .filter(|x| x != &user.id)
                    .collect::<Vec<String>>(),
            )
            .await?;

        // Fetch customisations.
        let emojis = db
            .fetch_emoji_by_parent_ids(
                &servers
                    .iter()
                    .map(|x| x.id.to_string())
                    .collect::<Vec<String>>(),
            )
            .await?;

        // Copy data into local state cache.
        self.cache.users = users.iter().cloned().map(|x| (x.id.clone(), x)).collect();
        self.cache
            .users
            .insert(self.cache.user_id.clone(), user.clone());
        self.cache.channels = channels
            .iter()
            .cloned()
            .map(|x| (x.id().to_string(), x))
            .collect();

        // Make all users appear from our perspective.
        let mut users: Vec<v0::User> = users
            .into_iter()
            .map(|other_user| {
                let is_online = online_ids.contains(&other_user.id);
                other_user.into_known(&user, is_online)
            })
            .collect();

        // Make sure we see our own user correctly.
        users.push(user.into_self().await);

        // Set subscription state internally.
        self.reset_state();
        self.insert_subscription(self.private_topic.clone());

        for user in &users {
            self.insert_subscription(user.id.clone());
        }

        for server in &servers {
            self.insert_subscription(server.id.clone());
        }

        for channel in &channels {
            self.insert_subscription(channel.id().to_string());
        }

        // fetch voice states for all the channels we can see
        let mut voice_states = Vec::new();

        for channel in &channels {
            if let Ok(Some(voice_state)) = self.fetch_voice_state(channel).await {
                voice_states.push(voice_state)
            }
        }

        Ok(EventV1::Ready {
            users,
            servers: servers.into_iter().map(Into::into).collect(),
            channels: channels.into_iter().map(Into::into).collect(),
            members: members.into_iter().map(Into::into).collect(),
            emojis: emojis.into_iter().map(Into::into).collect(),
            voice_states,
        })
    }

    /// Re-determine the currently accessible server channels
    pub async fn recalculate_server(&mut self, db: &Database, id: &str, event: &mut EventV1) {
        if let Some(server) = self.cache.servers.get(id) {
            let mut channel_ids = HashSet::new();
            let mut added_channels = vec![];
            let mut removed_channels = vec![];

            let id = &id.to_string();
            for (channel_id, channel) in &self.cache.channels {
                match channel {
                    Channel::TextChannel { server, .. } | Channel::VoiceChannel { server, .. } => {
                        if server == id {
                            channel_ids.insert(channel_id.clone());

                            if self.cache.can_view_channel(db, channel).await {
                                added_channels.push(channel_id.clone());
                            } else {
                                removed_channels.push(channel_id.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }

            let known_ids = server.channels.iter().cloned().collect::<HashSet<String>>();

            let mut bulk_events = vec![];

            for id in added_channels {
                self.insert_subscription(id);
            }

            for id in removed_channels {
                self.remove_subscription(&id);
                self.cache.channels.remove(&id);

                bulk_events.push(EventV1::ChannelDelete { id });
            }

            // * NOTE: currently all channels should be cached
            // * provided that a server was loaded from payload
            let unknowns = known_ids
                .difference(&channel_ids)
                .cloned()
                .collect::<Vec<String>>();

            if !unknowns.is_empty() {
                if let Ok(channels) = db.fetch_channels(&unknowns).await {
                    let viewable_channels =
                        self.cache.filter_accessible_channels(db, channels).await;

                    for channel in viewable_channels {
                        self.cache
                            .channels
                            .insert(channel.id().to_string(), channel.clone());

                        self.insert_subscription(channel.id().to_string());
                        bulk_events.push(EventV1::ChannelCreate(channel.into()));
                    }
                }
            }

            if !bulk_events.is_empty() {
                let mut new_event = EventV1::Bulk { v: bulk_events };
                std::mem::swap(&mut new_event, event);

                if let EventV1::Bulk { v } = event {
                    v.push(new_event);
                }
            }
        }
    }

    /// Push presence change to the user and all associated server topics
    pub async fn broadcast_presence_change(&self, target: bool) {
        if if let Some(status) = &self.cache.users.get(&self.cache.user_id).unwrap().status {
            status.presence != Some(Presence::Invisible)
        } else {
            true
        } {
            let event = EventV1::UserUpdate {
                id: self.cache.user_id.clone(),
                data: v0::PartialUser {
                    online: Some(target),
                    ..Default::default()
                },
                clear: vec![],
                event_id: Some(ulid::Ulid::new().to_string()),
            };

            for server in self.cache.servers.keys() {
                event.clone().p(server.clone()).await;
            }

            event.p(self.cache.user_id.clone()).await;
        }
    }

    /// Handle an incoming event for protocol version 1
    pub async fn handle_incoming_event_v1(&mut self, db: &Database, event: &mut EventV1) -> bool {
        /* Superseded by private topics.
          if match event {
            EventV1::UserRelationship { id, .. }
            | EventV1::UserSettingsUpdate { id, .. }
            | EventV1::ChannelAck { id, .. } => id != &self.cache.user_id,
            EventV1::ServerCreate { server, .. } => server.owner != self.cache.user_id,
            EventV1::ChannelCreate(channel) => match channel {
                Channel::SavedMessages { user, .. } => user != &self.cache.user_id,
                Channel::DirectMessage { recipients, .. } | Channel::Group { recipients, .. } => {
                    !recipients.contains(&self.cache.user_id)
                }
                _ => false,
            },
            _ => false,
        } {
            return false;
        }*/

        // An event may trigger recalculation of an entire server's permission.
        // Keep track of whether we need to do anything.
        let mut queue_server = None;

        // It may also need to sub or unsub a single value.
        let mut queue_add = None;
        let mut queue_remove = None;

        match event {
            EventV1::ChannelCreate(channel) => {
                let id = channel.id().to_string();
                self.insert_subscription(id.clone());
                self.cache.channels.insert(id, channel.clone().into());
            }
            EventV1::ChannelUpdate {
                id, data, clear, ..
            } => {
                let could_view: bool = if let Some(channel) = self.cache.channels.get(id) {
                    self.cache.can_view_channel(db, channel).await
                } else {
                    false
                };

                if let Some(channel) = self.cache.channels.get_mut(id) {
                    for field in clear {
                        channel.remove_field(&field.clone().into());
                    }

                    channel.apply_options(data.clone().into());
                }

                if !self.cache.channels.contains_key(id) {
                    if let Ok(channel) = db.fetch_channel(id).await {
                        self.cache.channels.insert(id.clone(), channel);
                    }
                }

                if let Some(channel) = self.cache.channels.get(id) {
                    let can_view = self.cache.can_view_channel(db, channel).await;
                    if could_view != can_view {
                        if can_view {
                            queue_add = Some(id.clone());
                            *event = EventV1::ChannelCreate(channel.clone().into());
                        } else {
                            queue_remove = Some(id.clone());
                            *event = EventV1::ChannelDelete { id: id.clone() };
                        }
                    }
                }
            }
            EventV1::ChannelDelete { id } => {
                self.remove_subscription(id);
                self.cache.channels.remove(id);
            }
            EventV1::ChannelGroupJoin { user, .. } => {
                self.insert_subscription(user.clone());
            }
            EventV1::ChannelGroupLeave { id, user, .. } => {
                if user == &self.cache.user_id {
                    self.remove_subscription(id);
                } else if !self.cache.can_subscribe_to_user(user) {
                    self.remove_subscription(user);
                }
            }

            EventV1::ServerCreate {
                id,
                server,
                channels,
                emojis: _,
            } => {
                self.insert_subscription(id.clone());
                self.cache.servers.insert(id.clone(), server.clone().into());
                let member = Member {
                    id: MemberCompositeKey {
                        server: server.id.clone(),
                        user: self.cache.user_id.clone(),
                    },
                    ..Default::default()
                };
                self.cache.members.insert(id.clone(), member);

                for channel in channels {
                    self.cache
                        .channels
                        .insert(channel.id().to_string(), channel.clone().into());
                }

                queue_server = Some(id.clone());
            }
            EventV1::ServerUpdate {
                id, data, clear, ..
            } => {
                if let Some(server) = self.cache.servers.get_mut(id) {
                    for field in clear {
                        server.remove_field(&field.clone().into());
                    }

                    server.apply_options(data.clone().into());
                }

                if data.default_permissions.is_some() {
                    queue_server = Some(id.clone());
                }
            }
            EventV1::ServerMemberJoin { .. } => {
                // We will always receive ServerCreate when joining a new server.
            }
            EventV1::ServerMemberLeave { id, user } => {
                if user == &self.cache.user_id {
                    self.remove_subscription(id);

                    if let Some(server) = self.cache.servers.remove(id) {
                        for channel in &server.channels {
                            self.remove_subscription(channel);
                            self.cache.channels.remove(channel);
                        }
                    }
                    self.cache.members.remove(id);
                }
            }
            EventV1::ServerDelete { id } => {
                self.remove_subscription(id);

                if let Some(server) = self.cache.servers.remove(id) {
                    for channel in &server.channels {
                        self.remove_subscription(channel);
                        self.cache.channels.remove(channel);
                    }
                }
                self.cache.members.remove(id);
            }
            EventV1::ServerMemberUpdate { id, data, clear } => {
                if id.user == self.cache.user_id {
                    if let Some(member) = self.cache.members.get_mut(&id.server) {
                        for field in &clear.clone() {
                            member.remove_field(&field.clone().into());
                        }

                        member.apply_options(data.clone().into());
                    }

                    if data.roles.is_some() || clear.contains(&v0::FieldsMember::Roles) {
                        queue_server = Some(id.server.clone());
                    }
                }
            }
            EventV1::ServerRoleUpdate {
                id,
                role_id,
                data,
                clear,
                ..
            } => {
                if let Some(server) = self.cache.servers.get_mut(id) {
                    if let Some(role) = server.roles.get_mut(role_id) {
                        for field in &clear.clone() {
                            role.remove_field(&field.clone().into());
                        }

                        role.apply_options(data.clone().into());
                    }
                }

                if data.rank.is_some() || data.permissions.is_some() {
                    if let Some(member) = self.cache.members.get(id) {
                        if member.roles.contains(role_id) {
                            queue_server = Some(id.clone());
                        }
                    }
                }
            }
            EventV1::ServerRoleDelete { id, role_id } => {
                if let Some(server) = self.cache.servers.get_mut(id) {
                    server.roles.remove(role_id);
                }

                if let Some(member) = self.cache.members.get(id) {
                    if member.roles.contains(role_id) {
                        queue_server = Some(id.clone());
                    }
                }
            }

            EventV1::UserUpdate { event_id, .. } => {
                if let Some(id) = event_id {
                    if self.cache.seen_events.contains(id) {
                        return false;
                    }

                    self.cache.seen_events.put(id.to_string(), ());
                }

                *event_id = None;
            }
            EventV1::UserRelationship { id, user, .. } => {
                self.cache.users.insert(id.clone(), user.clone().into());

                if self.cache.can_subscribe_to_user(id) {
                    self.insert_subscription(id.clone());
                } else {
                    self.remove_subscription(id);
                }
            }

            _ => {}
        }

        // Calculate server permissions if requested.
        if let Some(server_id) = queue_server {
            self.recalculate_server(db, &server_id, event).await;
        }

        // Sub / unsub accordingly.
        if let Some(id) = queue_add {
            self.insert_subscription(id);
        }

        if let Some(id) = queue_remove {
            self.remove_subscription(&id);
        }

        true
    }

    async fn fetch_voice_state(
        &self,
        channel: &Channel,
    ) -> Result<Option<v0::ChannelVoiceState>> {
        let members = get_voice_channel_members(&channel.id()).await?;

        if !members.is_empty() {
            let mut participants = Vec::with_capacity(members.len());

            for user_id in members {
                if let Some(voice_state) = get_voice_state(&channel.id(), channel.server().as_deref(), &user_id).await? {
                    participants.push(voice_state);
                } else {
                    log::info!("Voice state not found but member in voice channel members, removing.");

                    delete_voice_state(&channel.id(), channel.server().as_deref(),  &user_id).await?;
                }
            }

            Ok(Some(v0::ChannelVoiceState {
                id: channel.id(),
                participants,
            }))
        } else {
            Ok(None)
        }
    }
}
