use revolt_models::v0::*;
use revolt_permissions::{
    calculate_user_permissions, ChannelPermission, PermissionValue, UserPermission,
};

use crate::{util::permissions::DatabasePermissionQuery, Database, FileUsedFor};

impl crate::Bot {
    pub fn into_public_bot(self, user: crate::User) -> PublicBot {
        #[cfg(debug_assertions)]
        assert_eq!(self.id, user.id);

        PublicBot {
            id: self.id,
            username: user.username,
            avatar: user.avatar.map(|x| x.id).unwrap_or_default(),
            description: user
                .profile
                .map(|profile| profile.content)
                .flatten()
                .unwrap_or_default(),
        }
    }
}

impl From<crate::Bot> for Bot {
    fn from(value: crate::Bot) -> Self {
        Bot {
            id: value.id,
            owner_id: value.owner,
            token: value.token,
            public: value.public,
            analytics: value.analytics,
            discoverable: value.discoverable,
            interactions_url: value.interactions_url,
            terms_of_service_url: value.terms_of_service_url,
            privacy_policy_url: value.privacy_policy_url,
            flags: value.flags.unwrap_or_default() as u32,
        }
    }
}

impl From<FieldsBot> for crate::FieldsBot {
    fn from(value: FieldsBot) -> Self {
        match value {
            FieldsBot::InteractionsURL => crate::FieldsBot::InteractionsURL,
            FieldsBot::Token => crate::FieldsBot::Token,
        }
    }
}

impl From<crate::FieldsBot> for FieldsBot {
    fn from(value: crate::FieldsBot) -> Self {
        match value {
            crate::FieldsBot::InteractionsURL => FieldsBot::InteractionsURL,
            crate::FieldsBot::Token => FieldsBot::Token,
        }
    }
}

impl From<crate::Invite> for Invite {
    fn from(value: crate::Invite) -> Self {
        match value {
            crate::Invite::Group {
                code,
                creator,
                channel,
            } => Invite::Group {
                code,
                creator,
                channel,
            },
            crate::Invite::Server {
                code,
                server,
                creator,
                channel,
            } => Invite::Server {
                code,
                server,
                creator,
                channel,
            },
        }
    }
}

impl From<crate::ChannelUnread> for ChannelUnread {
    fn from(value: crate::ChannelUnread) -> Self {
        ChannelUnread {
            id: value.id.into(),
            last_id: value.last_id,
            mentions: value.mentions.unwrap_or_default(),
        }
    }
}

impl From<crate::ChannelCompositeKey> for ChannelCompositeKey {
    fn from(value: crate::ChannelCompositeKey) -> Self {
        ChannelCompositeKey {
            channel: value.channel,
            user: value.user,
        }
    }
}

impl From<crate::Webhook> for Webhook {
    fn from(value: crate::Webhook) -> Self {
        Webhook {
            id: value.id,
            name: value.name,
            avatar: value.avatar.map(|file| file.into()),
            creator_id: value.creator_id,
            channel_id: value.channel_id,
            token: value.token,
            permissions: value.permissions,
        }
    }
}

impl From<crate::PartialWebhook> for PartialWebhook {
    fn from(value: crate::PartialWebhook) -> Self {
        PartialWebhook {
            id: value.id,
            name: value.name,
            avatar: value.avatar.map(|file| file.into()),
            creator_id: value.creator_id,
            channel_id: value.channel_id,
            token: value.token,
            permissions: value.permissions,
        }
    }
}

impl From<FieldsWebhook> for crate::FieldsWebhook {
    fn from(_value: FieldsWebhook) -> Self {
        Self::Avatar
    }
}

impl From<crate::FieldsWebhook> for FieldsWebhook {
    fn from(_value: crate::FieldsWebhook) -> Self {
        Self::Avatar
    }
}

impl From<crate::Channel> for Channel {
    fn from(value: crate::Channel) -> Self {
        match value {
            crate::Channel::SavedMessages { id, user } => Channel::SavedMessages { id, user },
            crate::Channel::DirectMessage {
                id,
                active,
                recipients,
                last_message_id,
            } => Channel::DirectMessage {
                id,
                active,
                recipients,
                last_message_id,
            },
            crate::Channel::Group {
                id,
                name,
                owner,
                description,
                recipients,
                icon,
                last_message_id,
                permissions,
                nsfw,
            } => Channel::Group {
                id,
                name,
                owner,
                description,
                recipients,
                icon: icon.map(|file| file.into()),
                last_message_id,
                permissions,
                nsfw,
            },
            crate::Channel::TextChannel {
                id,
                server,
                name,
                description,
                icon,
                last_message_id,
                default_permissions,
                role_permissions,
                nsfw,
            } => Channel::TextChannel {
                id,
                server,
                name,
                description,
                icon: icon.map(|file| file.into()),
                last_message_id,
                default_permissions,
                role_permissions,
                nsfw,
            },
            crate::Channel::VoiceChannel {
                id,
                server,
                name,
                description,
                icon,
                default_permissions,
                role_permissions,
                nsfw,
            } => Channel::VoiceChannel {
                id,
                server,
                name,
                description,
                icon: icon.map(|file| file.into()),
                default_permissions,
                role_permissions,
                nsfw,
            },
        }
    }
}

impl From<Channel> for crate::Channel {
    fn from(value: Channel) -> crate::Channel {
        match value {
            Channel::SavedMessages { id, user } => crate::Channel::SavedMessages { id, user },
            Channel::DirectMessage {
                id,
                active,
                recipients,
                last_message_id,
            } => crate::Channel::DirectMessage {
                id,
                active,
                recipients,
                last_message_id,
            },
            Channel::Group {
                id,
                name,
                owner,
                description,
                recipients,
                icon,
                last_message_id,
                permissions,
                nsfw,
            } => crate::Channel::Group {
                id,
                name,
                owner,
                description,
                recipients,
                icon: icon.map(|file| file.into()),
                last_message_id,
                permissions,
                nsfw,
            },
            Channel::TextChannel {
                id,
                server,
                name,
                description,
                icon,
                last_message_id,
                default_permissions,
                role_permissions,
                nsfw,
            } => crate::Channel::TextChannel {
                id,
                server,
                name,
                description,
                icon: icon.map(|file| file.into()),
                last_message_id,
                default_permissions,
                role_permissions,
                nsfw,
            },
            Channel::VoiceChannel {
                id,
                server,
                name,
                description,
                icon,
                default_permissions,
                role_permissions,
                nsfw,
            } => crate::Channel::VoiceChannel {
                id,
                server,
                name,
                description,
                icon: icon.map(|file| file.into()),
                default_permissions,
                role_permissions,
                nsfw,
            },
        }
    }
}

impl From<crate::PartialChannel> for PartialChannel {
    fn from(value: crate::PartialChannel) -> Self {
        PartialChannel {
            name: value.name,
            owner: value.owner,
            description: value.description,
            icon: value.icon.map(|file| file.into()),
            nsfw: value.nsfw,
            active: value.active,
            permissions: value.permissions,
            role_permissions: value.role_permissions,
            default_permissions: value.default_permissions,
            last_message_id: value.last_message_id,
        }
    }
}

impl From<PartialChannel> for crate::PartialChannel {
    fn from(value: PartialChannel) -> crate::PartialChannel {
        crate::PartialChannel {
            name: value.name,
            owner: value.owner,
            description: value.description,
            icon: value.icon.map(|file| file.into()),
            nsfw: value.nsfw,
            active: value.active,
            permissions: value.permissions,
            role_permissions: value.role_permissions,
            default_permissions: value.default_permissions,
            last_message_id: value.last_message_id,
        }
    }
}

impl From<FieldsChannel> for crate::FieldsChannel {
    fn from(value: FieldsChannel) -> Self {
        match value {
            FieldsChannel::Description => crate::FieldsChannel::Description,
            FieldsChannel::Icon => crate::FieldsChannel::Icon,
            FieldsChannel::DefaultPermissions => crate::FieldsChannel::DefaultPermissions,
        }
    }
}

impl From<crate::FieldsChannel> for FieldsChannel {
    fn from(value: crate::FieldsChannel) -> Self {
        match value {
            crate::FieldsChannel::Description => FieldsChannel::Description,
            crate::FieldsChannel::Icon => FieldsChannel::Icon,
            crate::FieldsChannel::DefaultPermissions => FieldsChannel::DefaultPermissions,
        }
    }
}

impl From<crate::Emoji> for Emoji {
    fn from(value: crate::Emoji) -> Self {
        Emoji {
            id: value.id,
            parent: value.parent.into(),
            creator_id: value.creator_id,
            name: value.name,
            animated: value.animated,
            nsfw: value.nsfw,
        }
    }
}

impl From<crate::EmojiParent> for EmojiParent {
    fn from(value: crate::EmojiParent) -> Self {
        match value {
            crate::EmojiParent::Detached => EmojiParent::Detached,
            crate::EmojiParent::Server { id } => EmojiParent::Server { id },
        }
    }
}

impl From<EmojiParent> for crate::EmojiParent {
    fn from(value: EmojiParent) -> Self {
        match value {
            EmojiParent::Detached => crate::EmojiParent::Detached,
            EmojiParent::Server { id } => crate::EmojiParent::Server { id },
        }
    }
}

impl From<crate::File> for File {
    fn from(value: crate::File) -> Self {
        File {
            id: value.id,
            tag: value.tag,
            filename: value.filename,
            metadata: value.metadata.into(),
            content_type: value.content_type,
            size: value.size,
            deleted: value.deleted,
            reported: value.reported,
            message_id: value.message_id,
            user_id: value.user_id,
            server_id: value.server_id,
            object_id: value.object_id,
        }
    }
}

impl From<File> for crate::File {
    fn from(value: File) -> crate::File {
        crate::File {
            id: value.id,
            tag: value.tag,
            filename: value.filename,
            metadata: value.metadata.into(),
            content_type: value.content_type,
            size: value.size,
            deleted: value.deleted,
            reported: value.reported,
            message_id: value.message_id,
            user_id: value.user_id,
            server_id: value.server_id,
            object_id: value.object_id,
            hash: None,
            uploaded_at: None,
            uploader_id: None,
            used_for: None,
        }
    }
}

impl From<crate::Metadata> for Metadata {
    fn from(value: crate::Metadata) -> Self {
        match value {
            crate::Metadata::File => Metadata::File,
            crate::Metadata::Text => Metadata::Text,
            crate::Metadata::Image { width, height } => Metadata::Image {
                width: width as usize,
                height: height as usize,
            },
            crate::Metadata::Video { width, height } => Metadata::Video {
                width: width as usize,
                height: height as usize,
            },
            crate::Metadata::Audio => Metadata::Audio,
        }
    }
}

impl From<Metadata> for crate::Metadata {
    fn from(value: Metadata) -> crate::Metadata {
        match value {
            Metadata::File => crate::Metadata::File,
            Metadata::Text => crate::Metadata::Text,
            Metadata::Image { width, height } => crate::Metadata::Image {
                width: width as isize,
                height: height as isize,
            },
            Metadata::Video { width, height } => crate::Metadata::Video {
                width: width as isize,
                height: height as isize,
            },
            Metadata::Audio => crate::Metadata::Audio,
        }
    }
}

impl crate::Message {
    pub fn into_model(self, user: Option<User>, member: Option<Member>) -> Message {
        Message {
            id: self.id,
            nonce: self.nonce,
            channel: self.channel,
            author: self.author,
            user,
            member,
            webhook: self.webhook,
            content: self.content,
            system: self.system.map(Into::into),
            attachments: self
                .attachments
                .map(|v| v.into_iter().map(|f| f.into()).collect()),
            edited: self.edited,
            embeds: self.embeds,
            mentions: self.mentions,
            replies: self.replies,
            reactions: self.reactions,
            interactions: self.interactions.into(),
            masquerade: self.masquerade.map(Into::into),
            flags: self.flags.map(|flags| flags as u32).unwrap_or_default(),
            pinned: self.pinned,
        }
    }
}

impl From<crate::PartialMessage> for PartialMessage {
    fn from(value: crate::PartialMessage) -> Self {
        PartialMessage {
            id: value.id,
            nonce: value.nonce,
            channel: value.channel,
            author: value.author,
            user: None,
            member: None,
            webhook: value.webhook,
            content: value.content,
            system: value.system.map(Into::into),
            attachments: value
                .attachments
                .map(|v| v.into_iter().map(|f| f.into()).collect()),
            edited: value.edited,
            embeds: value.embeds,
            mentions: value.mentions,
            replies: value.replies,
            reactions: value.reactions,
            interactions: value.interactions.map(Into::into),
            masquerade: value.masquerade.map(Into::into),
            flags: value.flags.map(|flags| flags as u32),
            pinned: value.pinned,
        }
    }
}

impl From<crate::SystemMessage> for SystemMessage {
    fn from(value: crate::SystemMessage) -> Self {
        match value {
            crate::SystemMessage::ChannelDescriptionChanged { by } => {
                Self::ChannelDescriptionChanged { by }
            }
            crate::SystemMessage::ChannelIconChanged { by } => Self::ChannelIconChanged { by },
            crate::SystemMessage::ChannelOwnershipChanged { from, to } => {
                Self::ChannelOwnershipChanged { from, to }
            }
            crate::SystemMessage::ChannelRenamed { name, by } => Self::ChannelRenamed { name, by },
            crate::SystemMessage::Text { content } => Self::Text { content },
            crate::SystemMessage::UserAdded { id, by } => Self::UserAdded { id, by },
            crate::SystemMessage::UserBanned { id } => Self::UserBanned { id },
            crate::SystemMessage::UserJoined { id } => Self::UserJoined { id },
            crate::SystemMessage::UserKicked { id } => Self::UserKicked { id },
            crate::SystemMessage::UserLeft { id } => Self::UserLeft { id },
            crate::SystemMessage::UserRemove { id, by } => Self::UserRemove { id, by },
            crate::SystemMessage::MessagePinned { id, by } => Self::MessagePinned { id, by },
            crate::SystemMessage::MessageUnpinned { id, by } => Self::MessageUnpinned { id, by },
        }
    }
}

impl From<crate::Interactions> for Interactions {
    fn from(value: crate::Interactions) -> Self {
        Interactions {
            reactions: value
                .reactions
                .map(|reactions| reactions.into_iter().collect()),
            restrict_reactions: value.restrict_reactions,
        }
    }
}

impl From<Interactions> for crate::Interactions {
    fn from(value: Interactions) -> Self {
        crate::Interactions {
            reactions: value
                .reactions
                .map(|reactions| reactions.into_iter().collect()),
            restrict_reactions: value.restrict_reactions,
        }
    }
}

impl From<crate::AppendMessage> for AppendMessage {
    fn from(value: crate::AppendMessage) -> Self {
        AppendMessage {
            embeds: value.embeds,
        }
    }
}

impl From<crate::Masquerade> for Masquerade {
    fn from(value: crate::Masquerade) -> Self {
        Masquerade {
            name: value.name,
            avatar: value.avatar,
            colour: value.colour,
        }
    }
}

impl From<Masquerade> for crate::Masquerade {
    fn from(value: Masquerade) -> Self {
        crate::Masquerade {
            name: value.name,
            avatar: value.avatar,
            colour: value.colour,
        }
    }
}

impl From<crate::Report> for Report {
    fn from(value: crate::Report) -> Self {
        Report {
            id: value.id,
            author_id: value.author_id,
            content: value.content,
            additional_context: value.additional_context,
            status: value.status,
            notes: value.notes,
        }
    }
}

impl From<crate::ServerBan> for ServerBan {
    fn from(value: crate::ServerBan) -> Self {
        ServerBan {
            id: value.id.into(),
            reason: value.reason,
        }
    }
}

impl crate::Member {
    pub fn into(self, perspective_permissions: Option<PermissionValue>) -> Member {
        let mut m = Member {
            id: self.id.into(),
            joined_at: self.joined_at,
            nickname: self.nickname,
            avatar: self.avatar.map(|f| f.into()),
            roles: self.roles,
            timeout: self.timeout,
            used_invite: self.used_invite,
        };

        match perspective_permissions {
            Some(permissions) => {
                if !permissions.has_channel_permission(ChannelPermission::ViewInvitation) {
                    m.used_invite = None;
                }
            }
            None => {}
        }

        m
    }
}

impl From<Member> for crate::Member {
    fn from(value: Member) -> crate::Member {
        crate::Member {
            id: value.id.into(),
            joined_at: value.joined_at,
            nickname: value.nickname,
            avatar: value.avatar.map(|f| f.into()),
            roles: value.roles,
            timeout: value.timeout,
            used_invite: value.used_invite,
        }
    }
}

impl crate::PartialMember {
    pub fn into(self, perspective_permissions: Option<PermissionValue>) -> PartialMember {
        let mut pm = PartialMember {
            id: self.id.map(|id| id.into()),
            joined_at: self.joined_at,
            nickname: self.nickname,
            avatar: self.avatar.map(|f| f.into()),
            roles: self.roles,
            timeout: self.timeout,
            used_invite: self.used_invite,
        };

        match perspective_permissions {
            Some(permissions) => {
                if !permissions.has_channel_permission(ChannelPermission::ViewInvitation) {
                    pm.used_invite = None;
                }
            }
            None => {}
        }

        pm
    }
}

impl From<PartialMember> for crate::PartialMember {
    fn from(value: PartialMember) -> crate::PartialMember {
        crate::PartialMember {
            id: value.id.map(|id| id.into()),
            joined_at: value.joined_at,
            nickname: value.nickname,
            avatar: value.avatar.map(|f| f.into()),
            roles: value.roles,
            timeout: value.timeout,
            used_invite: value.used_invite,
        }
    }
}

impl From<crate::MemberCompositeKey> for MemberCompositeKey {
    fn from(value: crate::MemberCompositeKey) -> Self {
        MemberCompositeKey {
            server: value.server,
            user: value.user,
        }
    }
}

impl From<MemberCompositeKey> for crate::MemberCompositeKey {
    fn from(value: MemberCompositeKey) -> crate::MemberCompositeKey {
        crate::MemberCompositeKey {
            server: value.server,
            user: value.user,
        }
    }
}

impl From<crate::FieldsMember> for FieldsMember {
    fn from(value: crate::FieldsMember) -> Self {
        match value {
            crate::FieldsMember::Avatar => FieldsMember::Avatar,
            crate::FieldsMember::Nickname => FieldsMember::Nickname,
            crate::FieldsMember::Roles => FieldsMember::Roles,
            crate::FieldsMember::Timeout => FieldsMember::Timeout,
        }
    }
}

impl From<FieldsMember> for crate::FieldsMember {
    fn from(value: FieldsMember) -> crate::FieldsMember {
        match value {
            FieldsMember::Avatar => crate::FieldsMember::Avatar,
            FieldsMember::Nickname => crate::FieldsMember::Nickname,
            FieldsMember::Roles => crate::FieldsMember::Roles,
            FieldsMember::Timeout => crate::FieldsMember::Timeout,
        }
    }
}

impl From<crate::RemovalIntention> for RemovalIntention {
    fn from(value: crate::RemovalIntention) -> Self {
        match value {
            crate::RemovalIntention::Ban => RemovalIntention::Ban,
            crate::RemovalIntention::Kick => RemovalIntention::Kick,
            crate::RemovalIntention::Leave => RemovalIntention::Leave,
        }
    }
}

impl From<crate::Server> for Server {
    fn from(value: crate::Server) -> Self {
        Server {
            id: value.id,
            owner: value.owner,
            name: value.name,
            description: value.description,
            channels: value.channels,
            categories: value
                .categories
                .map(|categories| categories.into_iter().map(|v| v.into()).collect()),
            system_messages: value.system_messages.map(|v| v.into()),
            roles: value
                .roles
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            default_permissions: value.default_permissions,
            icon: value.icon.map(|f| f.into()),
            banner: value.banner.map(|f| f.into()),
            flags: value.flags.unwrap_or_default() as u32,
            nsfw: value.nsfw,
            analytics: value.analytics,
            discoverable: value.discoverable,
        }
    }
}

impl From<Server> for crate::Server {
    fn from(value: Server) -> crate::Server {
        crate::Server {
            id: value.id,
            owner: value.owner,
            name: value.name,
            description: value.description,
            channels: value.channels,
            categories: value
                .categories
                .map(|categories| categories.into_iter().map(|v| v.into()).collect()),
            system_messages: value.system_messages.map(|v| v.into()),
            roles: value
                .roles
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            default_permissions: value.default_permissions,
            icon: value.icon.map(|f| f.into()),
            banner: value.banner.map(|f| f.into()),
            flags: Some(value.flags as i32),
            nsfw: value.nsfw,
            analytics: value.analytics,
            discoverable: value.discoverable,
        }
    }
}

impl From<crate::PartialServer> for PartialServer {
    fn from(value: crate::PartialServer) -> Self {
        PartialServer {
            id: value.id,
            owner: value.owner,
            name: value.name,
            description: value.description,
            channels: value.channels,
            categories: value
                .categories
                .map(|categories| categories.into_iter().map(|v| v.into()).collect()),
            system_messages: value.system_messages.map(|v| v.into()),
            roles: value
                .roles
                .map(|roles| roles.into_iter().map(|(k, v)| (k, v.into())).collect()),
            default_permissions: value.default_permissions,
            icon: value.icon.map(|f| f.into()),
            banner: value.banner.map(|f| f.into()),
            flags: value.flags.map(|v| v as u32),
            nsfw: value.nsfw,
            analytics: value.analytics,
            discoverable: value.discoverable,
        }
    }
}

impl From<PartialServer> for crate::PartialServer {
    fn from(value: PartialServer) -> crate::PartialServer {
        crate::PartialServer {
            id: value.id,
            owner: value.owner,
            name: value.name,
            description: value.description,
            channels: value.channels,
            categories: value
                .categories
                .map(|categories| categories.into_iter().map(|v| v.into()).collect()),
            system_messages: value.system_messages.map(|v| v.into()),
            roles: value
                .roles
                .map(|roles| roles.into_iter().map(|(k, v)| (k, v.into())).collect()),
            default_permissions: value.default_permissions,
            icon: value.icon.map(|f| f.into()),
            banner: value.banner.map(|f| f.into()),
            flags: value.flags.map(|v| v as i32),
            nsfw: value.nsfw,
            analytics: value.analytics,
            discoverable: value.discoverable,
        }
    }
}

impl From<crate::FieldsServer> for FieldsServer {
    fn from(value: crate::FieldsServer) -> Self {
        match value {
            crate::FieldsServer::Banner => FieldsServer::Banner,
            crate::FieldsServer::Categories => FieldsServer::Categories,
            crate::FieldsServer::Description => FieldsServer::Description,
            crate::FieldsServer::Icon => FieldsServer::Icon,
            crate::FieldsServer::SystemMessages => FieldsServer::SystemMessages,
        }
    }
}

impl From<FieldsServer> for crate::FieldsServer {
    fn from(value: FieldsServer) -> crate::FieldsServer {
        match value {
            FieldsServer::Banner => crate::FieldsServer::Banner,
            FieldsServer::Categories => crate::FieldsServer::Categories,
            FieldsServer::Description => crate::FieldsServer::Description,
            FieldsServer::Icon => crate::FieldsServer::Icon,
            FieldsServer::SystemMessages => crate::FieldsServer::SystemMessages,
        }
    }
}

impl From<crate::Category> for Category {
    fn from(value: crate::Category) -> Self {
        Category {
            id: value.id,
            title: value.title,
            channels: value.channels,
        }
    }
}

impl From<Category> for crate::Category {
    fn from(value: Category) -> Self {
        crate::Category {
            id: value.id,
            title: value.title,
            channels: value.channels,
        }
    }
}

impl From<crate::SystemMessageChannels> for SystemMessageChannels {
    fn from(value: crate::SystemMessageChannels) -> Self {
        SystemMessageChannels {
            user_joined: value.user_joined,
            user_left: value.user_left,
            user_kicked: value.user_kicked,
            user_banned: value.user_banned,
        }
    }
}

impl From<SystemMessageChannels> for crate::SystemMessageChannels {
    fn from(value: SystemMessageChannels) -> Self {
        crate::SystemMessageChannels {
            user_joined: value.user_joined,
            user_left: value.user_left,
            user_kicked: value.user_kicked,
            user_banned: value.user_banned,
        }
    }
}

impl From<crate::Role> for Role {
    fn from(value: crate::Role) -> Self {
        Role {
            name: value.name,
            permissions: value.permissions,
            colour: value.colour,
            hoist: value.hoist,
            rank: value.rank,
        }
    }
}

impl From<Role> for crate::Role {
    fn from(value: Role) -> crate::Role {
        crate::Role {
            name: value.name,
            permissions: value.permissions,
            colour: value.colour,
            hoist: value.hoist,
            rank: value.rank,
        }
    }
}

impl From<crate::PartialRole> for PartialRole {
    fn from(value: crate::PartialRole) -> Self {
        PartialRole {
            name: value.name,
            permissions: value.permissions,
            colour: value.colour,
            hoist: value.hoist,
            rank: value.rank,
        }
    }
}

impl From<PartialRole> for crate::PartialRole {
    fn from(value: PartialRole) -> crate::PartialRole {
        crate::PartialRole {
            name: value.name,
            permissions: value.permissions,
            colour: value.colour,
            hoist: value.hoist,
            rank: value.rank,
        }
    }
}

impl From<crate::FieldsRole> for FieldsRole {
    fn from(value: crate::FieldsRole) -> Self {
        match value {
            crate::FieldsRole::Colour => FieldsRole::Colour,
        }
    }
}

impl From<FieldsRole> for crate::FieldsRole {
    fn from(value: FieldsRole) -> Self {
        match value {
            FieldsRole::Colour => crate::FieldsRole::Colour,
        }
    }
}

impl crate::User {
    pub async fn into<'a, P>(self, db: &Database, perspective: P) -> User
    where
        P: Into<Option<&'a crate::User>>,
    {
        let perspective = perspective.into();
        let (relationship, can_see_profile) = if self.bot.is_some() {
            (RelationshipStatus::None, true)
        } else if let Some(perspective) = perspective {
            let mut query = DatabasePermissionQuery::new(db, perspective).user(&self);

            if perspective.id == self.id {
                (RelationshipStatus::User, true)
            } else {
                (
                    perspective
                        .relations
                        .as_ref()
                        .map(|relations| {
                            relations
                                .iter()
                                .find(|relationship| relationship.id == self.id)
                                .map(|relationship| relationship.status.clone().into())
                                .unwrap_or_default()
                        })
                        .unwrap_or_default(),
                    calculate_user_permissions(&mut query)
                        .await
                        .has_user_permission(UserPermission::ViewProfile),
                )
            }
        } else {
            (RelationshipStatus::None, false)
        };

        User {
            username: self.username,
            discriminator: self.discriminator,
            display_name: self.display_name,
            avatar: self.avatar.map(|file| file.into()),
            relations: if let Some(crate::User { id, .. }) = perspective {
                if id == &self.id {
                    self.relations
                        .unwrap_or_default()
                        .into_iter()
                        .map(|relation| relation.into())
                        .collect()
                } else {
                    vec![]
                }
            } else {
                vec![]
            },
            badges: self.badges.unwrap_or_default() as u32,
            online: can_see_profile
                && revolt_presence::is_online(&self.id).await
                && !matches!(
                    self.status,
                    Some(crate::UserStatus {
                        presence: Some(crate::Presence::Invisible),
                        ..
                    })
                ),
            status: if can_see_profile {
                self.status.map(|status| status.into())
            } else {
                None
            },
            flags: self.flags.unwrap_or_default() as u32,
            privileged: self.privileged,
            bot: self.bot.map(|bot| bot.into()),
            relationship,
            id: self.id,
        }
    }

    /// Convert user object into user model assuming mutual connection
    ///
    /// Relations will never be included, i.e. when we process ourselves
    pub fn into_known<'a, P>(self, perspective: P, is_online: bool) -> User
    where
        P: Into<Option<&'a crate::User>>,
    {
        let perspective = perspective.into();
        let (relationship, can_see_profile) = if self.bot.is_some() {
            (RelationshipStatus::None, true)
        } else if let Some(perspective) = perspective {
            if perspective.id == self.id {
                (RelationshipStatus::User, true)
            } else {
                let relationship = perspective
                    .relations
                    .as_ref()
                    .map(|relations| {
                        relations
                            .iter()
                            .find(|relationship| relationship.id == self.id)
                            .map(|relationship| relationship.status.clone().into())
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();

                let can_see_profile = relationship != RelationshipStatus::BlockedOther;
                (relationship, can_see_profile)
            }
        } else {
            (RelationshipStatus::None, false)
        };

        User {
            username: self.username,
            discriminator: self.discriminator,
            display_name: self.display_name,
            avatar: self.avatar.map(|file| file.into()),
            relations: vec![],
            badges: self.badges.unwrap_or_default() as u32,
            online: can_see_profile
                && is_online
                && !matches!(
                    self.status,
                    Some(crate::UserStatus {
                        presence: Some(crate::Presence::Invisible),
                        ..
                    })
                ),
            status: if can_see_profile {
                self.status.map(|status| status.into())
            } else {
                None
            },
            flags: self.flags.unwrap_or_default() as u32,
            privileged: self.privileged,
            bot: self.bot.map(|bot| bot.into()),
            relationship,
            id: self.id,
        }
    }

    /// Convert user object into user model without presence information
    pub fn into_known_static<'a>(self, is_online: bool) -> User {
        User {
            username: self.username,
            discriminator: self.discriminator,
            display_name: self.display_name,
            avatar: self.avatar.map(|file| file.into()),
            relations: vec![],
            badges: self.badges.unwrap_or_default() as u32,
            online: is_online
                && !matches!(
                    self.status,
                    Some(crate::UserStatus {
                        presence: Some(crate::Presence::Invisible),
                        ..
                    })
                ),
            status: self.status.map(|status| status.into()),
            flags: self.flags.unwrap_or_default() as u32,
            privileged: self.privileged,
            bot: self.bot.map(|bot| bot.into()),
            relationship: RelationshipStatus::None, // events client will populate this from cache
            id: self.id,
        }
    }

    pub async fn into_self(self, force_online: bool) -> User {
        User {
            username: self.username,
            discriminator: self.discriminator,
            display_name: self.display_name,
            avatar: self.avatar.map(|file| file.into()),
            relations: self
                .relations
                .map(|relationships| {
                    relationships
                        .into_iter()
                        .map(|relationship| relationship.into())
                        .collect()
                })
                .unwrap_or_default(),
            badges: self.badges.unwrap_or_default() as u32,
            online: (force_online || revolt_presence::is_online(&self.id).await)
                && !matches!(
                    self.status,
                    Some(crate::UserStatus {
                        presence: Some(crate::Presence::Invisible),
                        ..
                    })
                ),
            status: self.status.map(|status| status.into()),
            flags: self.flags.unwrap_or_default() as u32,
            privileged: self.privileged,
            bot: self.bot.map(|bot| bot.into()),
            relationship: RelationshipStatus::User,
            id: self.id,
        }
    }

    pub fn as_author_for_system(&self) -> MessageAuthor {
        MessageAuthor::System {
            username: &self.username,
            avatar: self.avatar.as_ref().map(|file| file.id.as_ref()),
        }
    }
}

impl From<User> for crate::User {
    fn from(value: User) -> crate::User {
        crate::User {
            id: value.id,
            username: value.username,
            discriminator: value.discriminator,
            display_name: value.display_name,
            avatar: value.avatar.map(Into::into),
            relations: None,
            badges: Some(value.badges as i32),
            status: value.status.map(Into::into),
            profile: None,
            flags: Some(value.flags as i32),
            privileged: value.privileged,
            bot: value.bot.map(Into::into),
            suspended_until: None,
        }
    }
}

impl From<crate::PartialUser> for PartialUser {
    fn from(value: crate::PartialUser) -> Self {
        PartialUser {
            username: value.username,
            discriminator: value.discriminator,
            display_name: value.display_name,
            avatar: value.avatar.map(|file| file.into()),
            relations: value.relations.map(|relationships| {
                relationships
                    .into_iter()
                    .map(|relationship| relationship.into())
                    .collect()
            }),
            badges: value.badges.map(|badges| badges as u32),
            status: value.status.map(|status| status.into()),
            flags: value.flags.map(|flags| flags as u32),
            privileged: value.privileged,
            bot: value.bot.map(|bot| bot.into()),
            relationship: None,
            online: None,
            id: value.id,
        }
    }
}

impl From<FieldsUser> for crate::FieldsUser {
    fn from(value: FieldsUser) -> Self {
        match value {
            FieldsUser::Avatar => crate::FieldsUser::Avatar,
            FieldsUser::ProfileBackground => crate::FieldsUser::ProfileBackground,
            FieldsUser::ProfileContent => crate::FieldsUser::ProfileContent,
            FieldsUser::StatusPresence => crate::FieldsUser::StatusPresence,
            FieldsUser::StatusText => crate::FieldsUser::StatusText,
            FieldsUser::DisplayName => crate::FieldsUser::DisplayName,

            FieldsUser::Internal => crate::FieldsUser::None,
        }
    }
}

impl From<crate::FieldsUser> for FieldsUser {
    fn from(value: crate::FieldsUser) -> Self {
        match value {
            crate::FieldsUser::Avatar => FieldsUser::Avatar,
            crate::FieldsUser::ProfileBackground => FieldsUser::ProfileBackground,
            crate::FieldsUser::ProfileContent => FieldsUser::ProfileContent,
            crate::FieldsUser::StatusPresence => FieldsUser::StatusPresence,
            crate::FieldsUser::StatusText => FieldsUser::StatusText,
            crate::FieldsUser::DisplayName => FieldsUser::DisplayName,

            crate::FieldsUser::Suspension => FieldsUser::Internal,
            crate::FieldsUser::None => FieldsUser::Internal,
        }
    }
}

impl From<crate::RelationshipStatus> for RelationshipStatus {
    fn from(value: crate::RelationshipStatus) -> Self {
        match value {
            crate::RelationshipStatus::None => RelationshipStatus::None,
            crate::RelationshipStatus::User => RelationshipStatus::User,
            crate::RelationshipStatus::Friend => RelationshipStatus::Friend,
            crate::RelationshipStatus::Outgoing => RelationshipStatus::Outgoing,
            crate::RelationshipStatus::Incoming => RelationshipStatus::Incoming,
            crate::RelationshipStatus::Blocked => RelationshipStatus::Blocked,
            crate::RelationshipStatus::BlockedOther => RelationshipStatus::BlockedOther,
        }
    }
}

impl From<crate::Relationship> for Relationship {
    fn from(value: crate::Relationship) -> Self {
        Self {
            user_id: value.id,
            status: value.status.into(),
        }
    }
}

impl From<crate::Presence> for Presence {
    fn from(value: crate::Presence) -> Self {
        match value {
            crate::Presence::Online => Presence::Online,
            crate::Presence::Idle => Presence::Idle,
            crate::Presence::Focus => Presence::Focus,
            crate::Presence::Busy => Presence::Busy,
            crate::Presence::Invisible => Presence::Invisible,
        }
    }
}

impl From<Presence> for crate::Presence {
    fn from(value: Presence) -> crate::Presence {
        match value {
            Presence::Online => crate::Presence::Online,
            Presence::Idle => crate::Presence::Idle,
            Presence::Focus => crate::Presence::Focus,
            Presence::Busy => crate::Presence::Busy,
            Presence::Invisible => crate::Presence::Invisible,
        }
    }
}

impl From<crate::UserStatus> for UserStatus {
    fn from(value: crate::UserStatus) -> Self {
        UserStatus {
            text: value.text,
            presence: value.presence.map(|presence| presence.into()),
        }
    }
}

impl From<UserStatus> for crate::UserStatus {
    fn from(value: UserStatus) -> crate::UserStatus {
        crate::UserStatus {
            text: value.text,
            presence: value.presence.map(|presence| presence.into()),
        }
    }
}

impl From<crate::UserProfile> for UserProfile {
    fn from(value: crate::UserProfile) -> Self {
        UserProfile {
            content: value.content,
            background: value.background.map(|file| file.into()),
        }
    }
}

impl From<UserProfile> for crate::UserProfile {
    fn from(value: UserProfile) -> crate::UserProfile {
        crate::UserProfile {
            content: value.content,
            background: value.background.map(|file| file.into()),
        }
    }
}

impl From<crate::BotInformation> for BotInformation {
    fn from(value: crate::BotInformation) -> Self {
        BotInformation {
            owner_id: value.owner,
        }
    }
}

impl From<BotInformation> for crate::BotInformation {
    fn from(value: BotInformation) -> crate::BotInformation {
        crate::BotInformation {
            owner: value.owner_id,
        }
    }
}

impl From<crate::FieldsMessage> for FieldsMessage {
    fn from(value: crate::FieldsMessage) -> Self {
        match value {
            crate::FieldsMessage::Pinned => FieldsMessage::Pinned,
        }
    }
}
impl From<FieldsMessage> for crate::FieldsMessage {
    fn from(value: FieldsMessage) -> Self {
        match value {
            FieldsMessage::Pinned => crate::FieldsMessage::Pinned,
        }
    }
}
