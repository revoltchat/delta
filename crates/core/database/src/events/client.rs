use authifier::AuthifierEvent;
use serde::{Deserialize, Serialize};

use revolt_models::v0::{
    AppendMessage, Channel, ChannelVoiceState, Emoji, FieldsChannel, FieldsMember, FieldsRole, FieldsServer, FieldsUser, FieldsWebhook, Member, MemberCompositeKey, Message, PartialChannel, PartialMember, PartialMessage, PartialRole, PartialServer, PartialUser, PartialUserVoiceState, PartialWebhook, Report, Server, User, UserSettings, UserVoiceState, Webhook
};
use revolt_result::Error;

use crate::Database;

/// WebSocket Client Errors
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "error")]
pub enum WebSocketError {
    LabelMe,
    InternalError { at: String },
    InvalidSession,
    OnboardingNotFinished,
    AlreadyAuthenticated,
    MalformedData { msg: String },
}

/// Ping Packet
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Ping {
    Binary(Vec<u8>),
    Number(usize),
}

/// Untagged Error
#[derive(Serialize)]
#[serde(untagged)]
pub enum ErrorEvent {
    Error(WebSocketError),
    APIError(Error),
}

/// Protocol Events
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum EventV1 {
    /// Multiple events
    Bulk { v: Vec<EventV1> },

    /// Successfully authenticated
    Authenticated,
    /// Basic data to cache
    Ready {
        users: Vec<User>,
        servers: Vec<Server>,
        channels: Vec<Channel>,
        members: Vec<Member>,
        emojis: Vec<Emoji>,
        voice_states: Vec<ChannelVoiceState>
    },

    /// Ping response
    Pong { data: Ping },
    /// New message
    Message(Message),

    /// Update existing message
    MessageUpdate {
        id: String,
        channel: String,
        data: PartialMessage,
    },

    /// Append information to existing message
    MessageAppend {
        id: String,
        channel: String,
        append: AppendMessage,
    },

    /// Delete message
    MessageDelete { id: String, channel: String },

    /// New reaction to a message
    MessageReact {
        id: String,
        channel_id: String,
        user_id: String,
        emoji_id: String,
    },

    /// Remove user's reaction from message
    MessageUnreact {
        id: String,
        channel_id: String,
        user_id: String,
        emoji_id: String,
    },

    /// Remove a reaction from message
    MessageRemoveReaction {
        id: String,
        channel_id: String,
        emoji_id: String,
    },

    /// Bulk delete messages
    BulkMessageDelete { channel: String, ids: Vec<String> },

    /// New server
    ServerCreate {
        id: String,
        server: Server,
        channels: Vec<Channel>,
        emojis: Vec<Emoji>,
    },

    /// Update existing server
    ServerUpdate {
        id: String,
        data: PartialServer,
        clear: Vec<FieldsServer>,
    },

    /// Delete server
    ServerDelete { id: String },

    /// Update existing server member
    ServerMemberUpdate {
        id: MemberCompositeKey,
        data: PartialMember,
        clear: Vec<FieldsMember>,
    },

    /// User joins server
    ServerMemberJoin { id: String, user: String },

    /// User left server
    ServerMemberLeave { id: String, user: String },

    /// Server role created or updated
    ServerRoleUpdate {
        id: String,
        role_id: String,
        data: PartialRole,
        clear: Vec<FieldsRole>,
    },

    /// Server role deleted
    ServerRoleDelete { id: String, role_id: String },

    /// Update existing user
    UserUpdate {
        id: String,
        data: PartialUser,
        clear: Vec<FieldsUser>,
        event_id: Option<String>,
    },

    /// Relationship with another user changed
    UserRelationship { id: String, user: User },
    /// Settings updated remotely
    UserSettingsUpdate { id: String, update: UserSettings },

    /// User has been platform banned or deleted their account
    ///
    /// Clients should remove the following associated data:
    /// - Messages
    /// - DM Channels
    /// - Relationships
    /// - Server Memberships
    ///
    /// User flags are specified to explain why a wipe is occurring though not all reasons will necessarily ever appear.
    UserPlatformWipe { user_id: String, flags: i32 },
    /// New emoji
    EmojiCreate(Emoji),

    /// Delete emoji
    EmojiDelete { id: String },

    /// New report
    ReportCreate(Report),
    /// New channel
    ChannelCreate(Channel),

    /// Update existing channel
    ChannelUpdate {
        id: String,
        data: PartialChannel,
        clear: Vec<FieldsChannel>,
    },

    /// Delete channel
    ChannelDelete { id: String },

    /// User joins a group
    ChannelGroupJoin { id: String, user: String },

    /// User leaves a group
    ChannelGroupLeave { id: String, user: String },

    /// User started typing in a channel
    ChannelStartTyping { id: String, user: String },

    /// User stopped typing in a channel
    ChannelStopTyping { id: String, user: String },

    /// User acknowledged message in channel
    ChannelAck {
        id: String,
        user: String,
        message_id: String,
    },

    /// New webhook
    WebhookCreate(Webhook),

    /// Update existing webhook
    WebhookUpdate {
        id: String,
        data: PartialWebhook,
        remove: Vec<FieldsWebhook>,
    },

    /// Delete webhook
    WebhookDelete { id: String },

    /// Auth events
    Auth(AuthifierEvent),

    /// Voice events
    VoiceChannelJoin {
        id: String,
        state: UserVoiceState,
    },
    VoiceChannelLeave {
        id: String,
        user: String,
    },
    UserVoiceStateUpdate {
        id: String,
        channel_id: String,
        data: PartialUserVoiceState
    }
}

impl EventV1 {
    /// Publish helper wrapper
    pub async fn p(self, channel: String) {
        #[cfg(not(debug_assertions))]
        redis_kiss::p(channel, self).await;

        #[cfg(debug_assertions)]
        info!("Publishing event to {channel}: {self:?}");

        #[cfg(debug_assertions)]
        redis_kiss::publish(channel, self).await.unwrap();
    }

    /// Publish user event
    pub async fn p_user(self, id: String, db: &Database) {
        self.clone().p(id.clone()).await;

        // TODO: this should be captured by member list in the future and not immediately fanned out to users
        if let Ok(members) = db.fetch_all_memberships(&id).await {
            for member in members {
                self.clone().p(member.id.server).await;
            }
        }
    }

    /// Publish private event
    pub async fn private(self, id: String) {
        self.p(format!("{id}!")).await;
    }

    /// Publish internal global event
    pub async fn global(self) {
        self.p("global".to_string()).await;
    }
}
