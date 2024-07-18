use iso8601_timestamp::Timestamp;
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::{create_error, Result};

use crate::{
    events::client::EventV1, util::permissions::DatabasePermissionQuery, Channel, Database, File,
    Server, SystemMessage, User,
};

auto_derived_partial!(
    /// Server Member
    pub struct Member {
        /// Unique member id
        #[serde(rename = "_id")]
        pub id: MemberCompositeKey,

        /// Time at which this user joined the server
        pub joined_at: Timestamp,

        /// Invite used by member to join server
        #[serde(skip_serializing_if = "Option::is_none")]
        pub used_invite: Option<String>,

        /// Member's nickname
        #[serde(skip_serializing_if = "Option::is_none")]
        pub nickname: Option<String>,
        /// Avatar attachment
        #[serde(skip_serializing_if = "Option::is_none")]
        pub avatar: Option<File>,

        /// Member's roles
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        pub roles: Vec<String>,
        /// Timestamp this member is timed out until
        #[serde(skip_serializing_if = "Option::is_none")]
        pub timeout: Option<Timestamp>,
    },
    "PartialMember"
);

auto_derived!(
    /// Composite primary key consisting of server and user id
    #[derive(Hash, Default)]
    pub struct MemberCompositeKey {
        /// Server Id
        pub server: String,
        /// User Id
        pub user: String,
    }

    /// Optional fields on server member object
    pub enum FieldsMember {
        Nickname,
        Avatar,
        Roles,
        Timeout,
    }

    /// Member removal intention
    pub enum RemovalIntention {
        Leave,
        Kick,
        Ban,
    }
);

impl Default for Member {
    fn default() -> Self {
        Self {
            id: Default::default(),
            joined_at: Timestamp::now_utc(),
            nickname: None,
            avatar: None,
            roles: vec![],
            timeout: None,
            used_invite: None,
        }
    }
}

#[allow(clippy::disallowed_methods)]
impl Member {
    /// Create a new member in a server
    pub async fn create(
        db: &Database,
        server: &Server,
        user: &User,
        channels: Option<Vec<Channel>>,
        used_invite: Option<String>,
    ) -> Result<Vec<Channel>> {
        if db.fetch_ban(&server.id, &user.id).await.is_ok() {
            return Err(create_error!(Banned));
        }

        if db.fetch_member(&server.id, &user.id).await.is_ok() {
            return Err(create_error!(AlreadyInServer));
        }

        let member = Member {
            id: MemberCompositeKey {
                server: server.id.to_string(),
                user: user.id.to_string(),
            },
            used_invite,
            ..Default::default()
        };

        db.insert_member(&member).await?;

        let should_fetch = channels.is_none();
        let mut channels = channels.unwrap_or_default();

        if should_fetch {
            let query = DatabasePermissionQuery::new(db, user).server(server);
            let existing_channels = db.fetch_channels(&server.channels).await?;

            for channel in existing_channels {
                let mut channel_query = query.clone().channel(&channel);

                if calculate_channel_permissions(&mut channel_query)
                    .await
                    .has_channel_permission(ChannelPermission::ViewChannel)
                {
                    channels.push(channel);
                }
            }
        }

        let emojis = db.fetch_emoji_by_parent_id(&server.id).await?;

        EventV1::ServerMemberJoin {
            id: server.id.clone(),
            user: user.id.clone(),
        }
        .p(server.id.clone())
        .await;

        EventV1::ServerCreate {
            id: server.id.clone(),
            server: server.clone().into(),
            channels: channels
                .clone()
                .into_iter()
                .map(|channel| channel.into())
                .collect(),
            emojis: emojis.into_iter().map(|emoji| emoji.into()).collect(),
        }
        .private(user.id.clone())
        .await;

        if let Some(id) = server
            .system_messages
            .as_ref()
            .and_then(|x| x.user_joined.as_ref())
        {
            SystemMessage::UserJoined {
                id: user.id.clone(),
            }
            .into_message(id.to_string())
            .send_without_notifications(db, None, None, false, false)
            .await
            .ok();
        }

        Ok(channels)
    }

    /// Update member data
    pub async fn update<'a>(
        &mut self,
        db: &Database,
        partial: PartialMember,
        remove: Vec<FieldsMember>,
    ) -> Result<()> {
        for field in &remove {
            self.remove_field(field);
        }

        self.apply_options(partial.clone());

        db.update_member(&self.id, &partial, remove.clone()).await?;

        EventV1::ServerMemberUpdate {
            id: self.id.clone().into(),
            data: partial.into(),
            clear: remove.into_iter().map(|field| field.into()).collect(),
        }
        .p(self.id.server.clone())
        .await;

        Ok(())
    }

    pub fn remove_field(&mut self, field: &FieldsMember) {
        match field {
            FieldsMember::Avatar => self.avatar = None,
            FieldsMember::Nickname => self.nickname = None,
            FieldsMember::Roles => self.roles.clear(),
            FieldsMember::Timeout => self.timeout = None,
        }
    }

    /// Get this user's current ranking
    pub fn get_ranking(&self, server: &Server) -> i64 {
        let mut value = i64::MAX;
        for role in &self.roles {
            if let Some(role) = server.roles.get(role) {
                if role.rank < value {
                    value = role.rank;
                }
            }
        }

        value
    }

    /// Check whether this member is in timeout
    pub fn in_timeout(&self) -> bool {
        if let Some(timeout) = self.timeout {
            *timeout > *Timestamp::now_utc()
        } else {
            false
        }
    }

    /// Remove member from server
    pub async fn remove(
        self,
        db: &Database,
        server: &Server,
        intention: RemovalIntention,
        silent: bool,
    ) -> Result<()> {
        db.delete_member(&self.id).await?;

        EventV1::ServerMemberLeave {
            id: self.id.server.to_string(),
            user: self.id.user.to_string(),
            reason: intention.clone().into(),
        }
        .p(self.id.server.to_string())
        .await;

        if !silent {
            if let Some(id) = server
                .system_messages
                .as_ref()
                .and_then(|x| match intention {
                    RemovalIntention::Leave => x.user_left.as_ref(),
                    RemovalIntention::Kick => x.user_kicked.as_ref(),
                    RemovalIntention::Ban => x.user_banned.as_ref(),
                })
            {
                match intention {
                    RemovalIntention::Leave => SystemMessage::UserLeft { id: self.id.user },
                    RemovalIntention::Kick => SystemMessage::UserKicked { id: self.id.user },
                    RemovalIntention::Ban => SystemMessage::UserBanned { id: self.id.user },
                }
                .into_message(id.to_string())
                // TODO: support notifications here in the future?
                .send_without_notifications(db, None, None, false, false)
                .await
                .ok();
            }
        }

        Ok(())
    }
}
