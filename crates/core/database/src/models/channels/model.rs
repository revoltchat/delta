use std::{borrow::Cow, collections::HashMap};

use revolt_config::config;
use revolt_models::v0::{self, MessageAuthor, VoiceInformation};
use revolt_permissions::OverrideField;
use revolt_result::Result;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{
    events::client::EventV1, tasks::ack::AckEvent, Database, File, IntoDocumentPath, PartialServer,
    Server, SystemMessage, User,
};

auto_derived!(
    #[serde(tag = "channel_type")]
    pub enum Channel {
        /// Personal "Saved Notes" channel which allows users to save messages
        SavedMessages {
            /// Unique Id
            #[serde(rename = "_id")]
            id: String,
            /// Id of the user this channel belongs to
            user: String,
        },
        /// Direct message channel between two users
        DirectMessage {
            /// Unique Id
            #[serde(rename = "_id")]
            id: String,

            /// Whether this direct message channel is currently open on both sides
            active: bool,
            /// 2-tuple of user ids participating in direct message
            recipients: Vec<String>,
            /// Id of the last message sent in this channel
            #[serde(skip_serializing_if = "Option::is_none")]
            last_message_id: Option<String>,
        },
        /// Group channel between 1 or more participants
        Group {
            /// Unique Id
            #[serde(rename = "_id")]
            id: String,

            /// Display name of the channel
            name: String,
            /// User id of the owner of the group
            owner: String,
            /// Channel description
            #[serde(skip_serializing_if = "Option::is_none")]
            description: Option<String>,
            /// Array of user ids participating in channel
            recipients: Vec<String>,

            /// Custom icon attachment
            #[serde(skip_serializing_if = "Option::is_none")]
            icon: Option<File>,
            /// Id of the last message sent in this channel
            #[serde(skip_serializing_if = "Option::is_none")]
            last_message_id: Option<String>,

            /// Permissions assigned to members of this group
            /// (does not apply to the owner of the group)
            #[serde(skip_serializing_if = "Option::is_none")]
            permissions: Option<i64>,

            /// Whether this group is marked as not safe for work
            #[serde(skip_serializing_if = "crate::if_false", default)]
            nsfw: bool,
        },
        /// Text channel belonging to a server
        TextChannel {
            /// Unique Id
            #[serde(rename = "_id")]
            id: String,
            /// Id of the server this channel belongs to
            server: String,

            /// Display name of the channel
            name: String,
            /// Channel description
            #[serde(skip_serializing_if = "Option::is_none")]
            description: Option<String>,

            /// Custom icon attachment
            #[serde(skip_serializing_if = "Option::is_none")]
            icon: Option<File>,
            /// Id of the last message sent in this channel
            #[serde(skip_serializing_if = "Option::is_none")]
            last_message_id: Option<String>,

            /// Default permissions assigned to users in this channel
            #[serde(skip_serializing_if = "Option::is_none")]
            default_permissions: Option<OverrideField>,
            /// Permissions assigned based on role to this channel
            #[serde(
                default = "HashMap::<String, OverrideField>::new",
                skip_serializing_if = "HashMap::<String, OverrideField>::is_empty"
            )]
            role_permissions: HashMap<String, OverrideField>,

            /// Whether this channel is marked as not safe for work
            #[serde(skip_serializing_if = "crate::if_false", default)]
            nsfw: bool,

            /// Voice Information for when this channel is also a voice channel
            #[serde(skip_serializing_if = "Option::is_none")]
            voice: Option<VoiceInformation>
        },
        /// Voice channel belonging to a server
        VoiceChannel {
            /// Unique Id
            #[serde(rename = "_id")]
            id: String,
            /// Id of the server this channel belongs to
            server: String,

            /// Display name of the channel
            name: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            /// Channel description
            description: Option<String>,
            /// Custom icon attachment
            #[serde(skip_serializing_if = "Option::is_none")]
            icon: Option<File>,

            /// Default permissions assigned to users in this channel
            #[serde(skip_serializing_if = "Option::is_none")]
            default_permissions: Option<OverrideField>,
            /// Permissions assigned based on role to this channel
            #[serde(
                default = "HashMap::<String, OverrideField>::new",
                skip_serializing_if = "HashMap::<String, OverrideField>::is_empty"
            )]
            role_permissions: HashMap<String, OverrideField>,

            /// Whether this channel is marked as not safe for work
            #[serde(skip_serializing_if = "crate::if_false", default)]
            nsfw: bool,
        },
    }
);

auto_derived!(
    #[derive(Default)]
    pub struct PartialChannel {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub owner: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub icon: Option<File>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub nsfw: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub active: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub permissions: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub role_permissions: Option<HashMap<String, OverrideField>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub default_permissions: Option<OverrideField>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub last_message_id: Option<String>,
    }

    /// Optional fields on channel object
    pub enum FieldsChannel {
        Description,
        Icon,
        DefaultPermissions,
    }
);

#[allow(clippy::disallowed_methods)]
impl Channel {
    /* /// Create a channel
    pub async fn create(&self, db: &Database) -> Result<()> {
        db.insert_channel(self).await?;

        let event = EventV1::ChannelCreate(self.clone().into());
        match self {
            Self::SavedMessages { user, .. } => event.private(user.clone()).await,
            Self::DirectMessage { recipients, .. } | Self::Group { recipients, .. } => {
                for recipient in recipients {
                    event.clone().private(recipient.clone()).await;
                }
            }
            Self::TextChannel { server, .. } | Self::VoiceChannel { server, .. } => {
                event.p(server.clone()).await;
            }
        }

        Ok(())
    }*/

    /// Create a new server channel
    pub async fn create_server_channel(
        db: &Database,
        server: &mut Server,
        data: v0::DataCreateServerChannel,
        update_server: bool,
    ) -> Result<Channel> {
        let config = config().await;
        if server.channels.len() > config.features.limits.default.server_channels {
            return Err(create_error!(TooManyChannels {
                max: config.features.limits.default.server_channels,
            }));
        };

        let id = ulid::Ulid::new().to_string();
        let channel = match data.channel_type {
            v0::LegacyServerChannelType::Text => Channel::TextChannel {
                id: id.clone(),
                server: server.id.to_owned(),
                name: data.name,
                description: data.description,
                icon: None,
                last_message_id: None,
                default_permissions: None,
                role_permissions: HashMap::new(),
                nsfw: data.nsfw.unwrap_or(false),
                voice: data.voice
            },
            v0::LegacyServerChannelType::Voice => Channel::VoiceChannel {
                id: id.clone(),
                server: server.id.to_owned(),
                name: data.name,
                description: data.description,
                icon: None,
                default_permissions: None,
                role_permissions: HashMap::new(),
                nsfw: data.nsfw.unwrap_or(false),
            },
        };

        db.insert_channel(&channel).await?;

        if update_server {
            server
                .update(
                    db,
                    PartialServer {
                        channels: Some([server.channels.clone(), [id].into()].concat()),
                        ..Default::default()
                    },
                    vec![],
                )
                .await?;

            EventV1::ChannelCreate(channel.clone().into())
                .p(server.id.clone())
                .await;
        }

        Ok(channel)
    }

    /// Create a group
    pub async fn create_group(
        db: &Database,
        mut data: v0::DataCreateGroup,
        owner_id: String,
    ) -> Result<Channel> {
        data.users.insert(owner_id.to_string());

        let config = config().await;
        if data.users.len() > config.features.limits.default.group_size {
            return Err(create_error!(GroupTooLarge {
                max: config.features.limits.default.group_size,
            }));
        }

        let recipients = data.users.into_iter().collect::<Vec<String>>();
        let channel = Channel::Group {
            id: ulid::Ulid::new().to_string(),

            name: data.name,
            owner: owner_id,
            description: data.description,
            recipients: recipients.clone(),

            icon: None,
            last_message_id: None,

            permissions: None,

            nsfw: data.nsfw.unwrap_or(false),
        };

        db.insert_channel(&channel).await?;

        let event = EventV1::ChannelCreate(channel.clone().into());
        for recipient in recipients {
            event.clone().private(recipient).await;
        }

        Ok(channel)
    }

    /// Create a DM (or return the existing one / saved messages)
    pub async fn create_dm(db: &Database, user_a: &User, user_b: &User) -> Result<Channel> {
        // Try to find existing channel
        if let Ok(channel) = db.find_direct_message_channel(&user_a.id, &user_b.id).await {
            Ok(channel)
        } else {
            let channel = if user_a.id == user_b.id {
                // Create a new saved messages channel
                Channel::SavedMessages {
                    id: Ulid::new().to_string(),
                    user: user_a.id.to_string(),
                }
            } else {
                // Create a new DM channel
                Channel::DirectMessage {
                    id: Ulid::new().to_string(),
                    active: true, // show by default
                    recipients: vec![user_a.id.clone(), user_b.id.clone()],
                    last_message_id: None,
                }
            };

            db.insert_channel(&channel).await?;

            match &channel {
                Channel::DirectMessage { .. } => {
                    let event = EventV1::ChannelCreate(channel.clone().into());
                    event.clone().private(user_a.id.clone()).await;
                    event.private(user_b.id.clone()).await;
                }
                _ => {}
            };

            Ok(channel)
        }
    }

    /// Add user to a group
    pub async fn add_user_to_group(
        &mut self,
        db: &Database,
        user: &User,
        by_id: &str,
    ) -> Result<()> {
        if let Channel::Group { recipients, .. } = self {
            if recipients.contains(&String::from(&user.id)) {
                return Err(create_error!(AlreadyInGroup));
            }

            let config = config().await;
            if recipients.len() >= config.features.limits.default.group_size {
                return Err(create_error!(GroupTooLarge {
                    max: config.features.limits.default.group_size
                }));
            }

            recipients.push(String::from(&user.id));
        }

        match &self {
            Channel::Group { id, .. } => {
                db.add_user_to_group(id, &user.id).await?;

                EventV1::ChannelGroupJoin {
                    id: id.to_string(),
                    user: user.id.to_string(),
                }
                .p(id.to_string())
                .await;

                SystemMessage::UserAdded {
                    id: user.id.to_string(),
                    by: by_id.to_string(),
                }
                .into_message(id.to_string())
                .send(
                    db,
                    MessageAuthor::System {
                        username: &user.username,
                        avatar: user.avatar.as_ref().map(|file| file.id.as_ref()),
                    },
                    self,
                    false,
                )
                .await
                .ok();

                EventV1::ChannelCreate(self.clone().into())
                    .private(user.id.to_string())
                    .await;

                Ok(())
            }
            _ => Err(create_error!(InvalidOperation)),
        }
    }

    /// Map out whether it is a direct DM
    pub fn is_direct_dm(&self) -> bool {
        matches!(self, Channel::DirectMessage { .. })
    }

    /// Check whether has a user as a recipient
    pub fn contains_user(&self, user_id: &str) -> bool {
        match self {
            Channel::Group { recipients, .. } => recipients.contains(&String::from(user_id)),
            _ => false,
        }
    }

    /// Get list of recipients
    pub fn users(&self) -> Result<Vec<String>> {
        match self {
            Channel::Group { recipients, .. } => Ok(recipients.to_owned()),
            _ => Err(create_error!(NotFound)),
        }
    }

    /// Clone this channel's id
    pub fn id(&self) -> String {
        match self {
            Channel::DirectMessage { id, .. }
            | Channel::Group { id, .. }
            | Channel::SavedMessages { id, .. }
            | Channel::TextChannel { id, .. }
            | Channel::VoiceChannel { id, .. } => id.clone(),
        }
    }

    /// Clone this channel's server id
    pub fn server(&self) -> Option<String> {
        match self {
            Channel::TextChannel { server, .. } | Channel::VoiceChannel { server, .. } => Some(server.clone()),
            _ => None
        }
    }

    /// Gets this channel's voice information
    pub fn voice(&self) -> Option<Cow<VoiceInformation>> {
        match self {
            Self::DirectMessage { .. } | Channel::VoiceChannel { .. } => Some(Cow::Owned(v0::VoiceInformation::default())),
            Self::TextChannel { voice: Some(voice), .. } => Some(Cow::Borrowed(voice)),
            _ => None
        }
    }

    /// Set role permission on a channel
    pub async fn set_role_permission(
        &mut self,
        db: &Database,
        role_id: &str,
        permissions: OverrideField,
    ) -> Result<()> {
        match self {
            Channel::TextChannel {
                id,
                server,
                role_permissions,
                ..
            }
            | Channel::VoiceChannel {
                id,
                server,
                role_permissions,
                ..
            } => {
                db.set_channel_role_permission(id, role_id, permissions)
                    .await?;

                role_permissions.insert(role_id.to_string(), permissions);

                EventV1::ChannelUpdate {
                    id: id.clone(),
                    data: PartialChannel {
                        role_permissions: Some(role_permissions.clone()),
                        ..Default::default()
                    }
                    .into(),
                    clear: vec![],
                }
                .p(server.clone())
                .await;

                Ok(())
            }
            _ => Err(create_error!(InvalidOperation)),
        }
    }

    /// Update channel data
    pub async fn update(
        &mut self,
        db: &Database,
        partial: PartialChannel,
        remove: Vec<FieldsChannel>,
    ) -> Result<()> {
        for field in &remove {
            self.remove_field(field);
        }

        self.apply_options(partial.clone());

        let id = self.id().to_string();
        db.update_channel(&id, &partial, remove.clone()).await?;

        EventV1::ChannelUpdate {
            id: id.clone(),
            data: partial.into(),
            clear: remove.into_iter().map(|v| v.into()).collect(),
        }
        .p(match self {
            Self::TextChannel { server, .. } | Self::VoiceChannel { server, .. } => server.clone(),
            _ => id,
        })
        .await;

        Ok(())
    }

    /// Remove a field from Channel object
    pub fn remove_field(&mut self, field: &FieldsChannel) {
        match field {
            FieldsChannel::Description => match self {
                Self::Group { description, .. }
                | Self::TextChannel { description, .. }
                | Self::VoiceChannel { description, .. } => {
                    description.take();
                }
                _ => {}
            },
            FieldsChannel::Icon => match self {
                Self::Group { icon, .. }
                | Self::TextChannel { icon, .. }
                | Self::VoiceChannel { icon, .. } => {
                    icon.take();
                }
                _ => {}
            },
            FieldsChannel::DefaultPermissions => match self {
                Self::TextChannel {
                    default_permissions,
                    ..
                }
                | Self::VoiceChannel {
                    default_permissions,
                    ..
                } => {
                    default_permissions.take();
                }
                _ => {}
            },
        }
    }

    /// Remove multiple fields from Channel object
    pub fn remove_fields(&mut self, partial: Vec<FieldsChannel>) {
        for field in partial {
            self.remove_field(&field)
        }
    }

    /// Apply partial channel to channel
    pub fn apply_options(&mut self, partial: PartialChannel) {
        match self {
            Self::SavedMessages { .. } => {}
            Self::DirectMessage { active, .. } => {
                if let Some(v) = partial.active {
                    *active = v;
                }
            }
            Self::Group {
                name,
                owner,
                description,
                icon,
                nsfw,
                permissions,
                ..
            } => {
                if let Some(v) = partial.name {
                    *name = v;
                }

                if let Some(v) = partial.owner {
                    *owner = v;
                }

                if let Some(v) = partial.description {
                    description.replace(v);
                }

                if let Some(v) = partial.icon {
                    icon.replace(v);
                }

                if let Some(v) = partial.nsfw {
                    *nsfw = v;
                }

                if let Some(v) = partial.permissions {
                    permissions.replace(v);
                }
            }
            Self::TextChannel {
                name,
                description,
                icon,
                nsfw,
                default_permissions,
                role_permissions,
                ..
            }
            | Self::VoiceChannel {
                name,
                description,
                icon,
                nsfw,
                default_permissions,
                role_permissions,
                ..
            } => {
                if let Some(v) = partial.name {
                    *name = v;
                }

                if let Some(v) = partial.description {
                    description.replace(v);
                }

                if let Some(v) = partial.icon {
                    icon.replace(v);
                }

                if let Some(v) = partial.nsfw {
                    *nsfw = v;
                }

                if let Some(v) = partial.role_permissions {
                    *role_permissions = v;
                }

                if let Some(v) = partial.default_permissions {
                    default_permissions.replace(v);
                }
            }
        }
    }

    /// Acknowledge a message
    pub async fn ack(&self, user: &str, message: &str) -> Result<()> {
        EventV1::ChannelAck {
            id: self.id().to_string(),
            user: user.to_string(),
            message_id: message.to_string(),
        }
        .private(user.to_string())
        .await;

        crate::tasks::ack::queue(
            self.id().to_string(),
            user.to_string(),
            AckEvent::AckMessage {
                id: message.to_string(),
            },
        )
        .await;

        Ok(())
    }

    /// Remove user from a group
    pub async fn remove_user_from_group(
        &self,
        db: &Database,
        user: &User,
        by_id: Option<&str>,
        silent: bool,
    ) -> Result<()> {
        match &self {
            Channel::Group {
                id,
                name,
                owner,
                recipients,
                ..
            } => {
                if &user.id == owner {
                    if let Some(new_owner) = recipients.iter().find(|x| *x != &user.id) {
                        db.update_channel(
                            id,
                            &PartialChannel {
                                owner: Some(new_owner.into()),
                                ..Default::default()
                            },
                            vec![],
                        )
                        .await?;

                        SystemMessage::ChannelOwnershipChanged {
                            from: owner.to_string(),
                            to: new_owner.to_string(),
                        }
                        .into_message(id.to_string())
                        .send(
                            db,
                            MessageAuthor::System {
                                username: name,
                                avatar: None,
                            },
                            self,
                            false,
                        )
                        .await
                        .ok();
                    } else {
                        return self.delete(db).await;
                    }
                }

                EventV1::ChannelGroupLeave {
                    id: id.to_string(),
                    user: user.id.to_string(),
                }
                .p(id.to_string())
                .await;

                if !silent {
                    if let Some(by) = by_id {
                        SystemMessage::UserRemove {
                            id: user.id.to_string(),
                            by: by.to_string(),
                        }
                    } else {
                        SystemMessage::UserLeft {
                            id: user.id.to_string(),
                        }
                    }
                    .into_message(id.to_string())
                    .send(
                        db,
                        MessageAuthor::System {
                            username: &user.username,
                            avatar: user.avatar.as_ref().map(|file| file.id.as_ref()),
                        },
                        self,
                        false,
                    )
                    .await
                    .ok();
                }

                Ok(())
            }

            _ => Err(create_error!(InvalidOperation)),
        }
    }

    /// Delete a channel
    pub async fn delete(&self, db: &Database) -> Result<()> {
        let id = self.id().to_string();
        EventV1::ChannelDelete { id: id.clone() }.p(id).await;
        // TODO: missing functionality:
        // - group invites
        // - channels list / categories list on server
        db.delete_channel(self).await
    }
}

impl IntoDocumentPath for FieldsChannel {
    fn as_path(&self) -> Option<&'static str> {
        Some(match self {
            FieldsChannel::Description => "description",
            FieldsChannel::Icon => "icon",
            FieldsChannel::DefaultPermissions => "default_permissions",
        })
    }
}

#[cfg(test)]
mod tests {
    use revolt_permissions::{calculate_channel_permissions, ChannelPermission};

    use crate::{fixture, util::permissions::DatabasePermissionQuery};

    #[async_std::test]
    async fn permissions_group_channel() {
        database_test!(|db| async move {
            fixture!(db, "group_with_members",
                owner user 0
                member1 user 1
                member2 user 2
                channel channel 3);

            let mut query = DatabasePermissionQuery::new(&db, &owner).channel(&channel);
            assert!(calculate_channel_permissions(&mut query)
                .await
                .has_channel_permission(ChannelPermission::SendMessage));

            let mut query = DatabasePermissionQuery::new(&db, &member1).channel(&channel);
            assert!(calculate_channel_permissions(&mut query)
                .await
                .has_channel_permission(ChannelPermission::SendMessage));

            let mut query = DatabasePermissionQuery::new(&db, &member2).channel(&channel);
            assert!(!calculate_channel_permissions(&mut query)
                .await
                .has_channel_permission(ChannelPermission::SendMessage));
        });
    }

    #[async_std::test]
    async fn permissions_text_channel() {
        database_test!(|db| async move {
            fixture!(db, "server_with_roles",
                owner user 0
                moderator user 1
                user user 2
                channel channel 3);

            let mut query = DatabasePermissionQuery::new(&db, &owner).channel(&channel);
            assert!(calculate_channel_permissions(&mut query)
                .await
                .has_channel_permission(ChannelPermission::SendMessage));

            let mut query = DatabasePermissionQuery::new(&db, &moderator).channel(&channel);
            assert!(calculate_channel_permissions(&mut query)
                .await
                .has_channel_permission(ChannelPermission::SendMessage));

            let mut query = DatabasePermissionQuery::new(&db, &user).channel(&channel);
            assert!(!calculate_channel_permissions(&mut query)
                .await
                .has_channel_permission(ChannelPermission::SendMessage));
        });
    }
}
