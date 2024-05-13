use super::{File, UserVoiceState};

use revolt_permissions::{Override, OverrideField};
use std::collections::{HashMap, HashSet};

#[cfg(feature = "rocket")]
use rocket::FromForm;

auto_derived!(
    /// Channel
    #[serde(tag = "channel_type")]
    pub enum Channel {
        /// Personal "Saved Notes" channel which allows users to save messages
        SavedMessages {
            /// Unique Id
            #[cfg_attr(feature = "serde", serde(rename = "_id"))]
            id: String,
            /// Id of the user this channel belongs to
            user: String,
        },
        /// Direct message channel between two users
        DirectMessage {
            /// Unique Id
            #[cfg_attr(feature = "serde", serde(rename = "_id"))]
            id: String,

            /// Whether this direct message channel is currently open on both sides
            active: bool,
            /// 2-tuple of user ids participating in direct message
            recipients: Vec<String>,
            /// Id of the last message sent in this channel
            #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
            last_message_id: Option<String>,
        },
        /// Group channel between 1 or more participants
        Group {
            /// Unique Id
            #[cfg_attr(feature = "serde", serde(rename = "_id"))]
            id: String,

            /// Display name of the channel
            name: String,
            /// User id of the owner of the group
            owner: String,
            /// Channel description
            #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
            description: Option<String>,
            /// Array of user ids participating in channel
            recipients: Vec<String>,

            /// Custom icon attachment
            #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
            icon: Option<File>,
            /// Id of the last message sent in this channel
            #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
            last_message_id: Option<String>,

            /// Permissions assigned to members of this group
            /// (does not apply to the owner of the group)
            #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
            permissions: Option<i64>,

            /// Whether this group is marked as not safe for work
            #[cfg_attr(
                feature = "serde",
                serde(skip_serializing_if = "crate::if_false", default)
            )]
            nsfw: bool,
        },
        /// Text channel belonging to a server
        TextChannel {
            /// Unique Id
            #[cfg_attr(feature = "serde", serde(rename = "_id"))]
            id: String,
            /// Id of the server this channel belongs to
            server: String,

            /// Display name of the channel
            name: String,
            /// Channel description
            #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
            description: Option<String>,

            /// Custom icon attachment
            #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
            icon: Option<File>,
            /// Id of the last message sent in this channel
            #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
            last_message_id: Option<String>,

            /// Default permissions assigned to users in this channel
            #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
            default_permissions: Option<OverrideField>,
            /// Permissions assigned based on role to this channel
            #[cfg_attr(
                feature = "serde",
                serde(
                    default = "HashMap::<String, OverrideField>::new",
                    skip_serializing_if = "HashMap::<String, OverrideField>::is_empty"
                )
            )]
            role_permissions: HashMap<String, OverrideField>,

            /// Whether this channel is marked as not safe for work
            #[cfg_attr(
                feature = "serde",
                serde(skip_serializing_if = "crate::if_false", default)
            )]
            nsfw: bool,

            /// Voice Information for when this channel is also a voice channel
            #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
            voice: Option<VoiceInformation>
        },
        /// DEPRECATED (use TextChannel { voice }): Voice channel belonging to a server
        VoiceChannel {
            /// Unique Id
            #[cfg_attr(feature = "serde", serde(rename = "_id"))]
            id: String,
            /// Id of the server this channel belongs to
            server: String,

            /// Display name of the channel
            name: String,
            #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
            /// Channel description
            description: Option<String>,
            /// Custom icon attachment
            #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
            icon: Option<File>,

            /// Default permissions assigned to users in this channel
            #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
            default_permissions: Option<OverrideField>,
            /// Permissions assigned based on role to this channel
            #[cfg_attr(
                feature = "serde",
                serde(
                    default = "HashMap::<String, OverrideField>::new",
                    skip_serializing_if = "HashMap::<String, OverrideField>::is_empty"
                )
            )]
            role_permissions: HashMap<String, OverrideField>,

            /// Whether this channel is marked as not safe for work
            #[cfg_attr(
                feature = "serde",
                serde(skip_serializing_if = "crate::if_false", default)
            )]
            nsfw: bool,
        },
    }

    /// Voice information for a channel
    #[derive(Default)]
    #[cfg_attr(feature = "validator", derive(validator::Validate))]
    pub struct VoiceInformation {
        /// Maximium amount of users allowed in the voice channel at once
        #[cfg_attr(feature = "validator", validate(range(min = 1)))]
        pub max_users: Option<u32>
    }

    /// Partial representation of a channel
    #[derive(Default)]
    pub struct PartialChannel {
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub name: Option<String>,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub owner: Option<String>,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub description: Option<String>,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub icon: Option<File>,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub nsfw: Option<bool>,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub active: Option<bool>,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub permissions: Option<i64>,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub role_permissions: Option<HashMap<String, OverrideField>>,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub default_permissions: Option<OverrideField>,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub last_message_id: Option<String>,
    }

    /// Optional fields on channel object
    pub enum FieldsChannel {
        Description,
        Icon,
        DefaultPermissions,
    }

    /// New webhook information
    #[cfg_attr(feature = "validator", derive(validator::Validate))]
    pub struct DataEditChannel {
        /// Channel name
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32)))]
        pub name: Option<String>,

        /// Channel description
        #[cfg_attr(feature = "validator", validate(length(min = 0, max = 1024)))]
        pub description: Option<String>,

        /// Group owner
        pub owner: Option<String>,

        /// Icon
        ///
        /// Provide an Autumn attachment Id.
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 128)))]
        pub icon: Option<String>,

        /// Whether this channel is age-restricted
        pub nsfw: Option<bool>,

        /// Whether this channel is archived
        pub archived: Option<bool>,

        /// Fields to remove from channel
        #[cfg_attr(feature = "serde", serde(default))]
        pub remove: Option<Vec<FieldsChannel>>,
    }

    /// Create new group
    #[derive(Default)]
    #[cfg_attr(feature = "validator", derive(validator::Validate))]
    pub struct DataCreateGroup {
        /// Group name
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32)))]
        pub name: String,
        /// Group description
        #[cfg_attr(feature = "validator", validate(length(min = 0, max = 1024)))]
        pub description: Option<String>,
        /// Group icon
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 128)))]
        pub icon: Option<String>,
        /// Array of user IDs to add to the group
        ///
        /// Must be friends with these users.
        #[cfg_attr(feature = "validator", validate(length(min = 0, max = 49)))]
        #[serde(default)]
        pub users: HashSet<String>,
        /// Whether this group is age-restricted
        #[serde(skip_serializing_if = "Option::is_none")]
        pub nsfw: Option<bool>,
    }

    /// Server Channel Type
    #[derive(Default)]
    pub enum LegacyServerChannelType {
        /// Text Channel
        #[default]
        Text,
        /// Voice Channel
        Voice,
    }

    /// Create new server channel
    #[derive(Default)]
    #[cfg_attr(feature = "validator", derive(validator::Validate))]
    pub struct DataCreateServerChannel {
        /// Channel type
        #[serde(rename = "type", default = "LegacyServerChannelType::default")]
        pub channel_type: LegacyServerChannelType,
        /// Channel name
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 32)))]
        pub name: String,
        /// Channel description
        #[cfg_attr(feature = "validator", validate(length(min = 0, max = 1024)))]
        pub description: Option<String>,
        /// Whether this channel is age restricted
        #[serde(skip_serializing_if = "Option::is_none")]
        pub nsfw: Option<bool>,

        /// Voice Information for when this channel is also a voice channel
        #[serde(skip_serializing_if = "Option::is_none")]
        pub voice: Option<VoiceInformation>
    }

    /// New default permissions
    #[serde(untagged)]
    pub enum DataDefaultChannelPermissions {
        Value {
            /// Permission values to set for members in a `Group`
            permissions: u64,
        },
        Field {
            /// Allow / deny values to set for members in this `TextChannel` or `VoiceChannel`
            permissions: Override,
        },
    }

    /// New role permissions
    pub struct DataSetRolePermissions {
        /// Allow / deny values to set for this role
        pub permissions: Override,
    }

    /// Options when deleting a channel
    #[cfg_attr(feature = "rocket", derive(FromForm))]
    pub struct OptionsChannelDelete {
        /// Whether to not send a leave message
        pub leave_silently: Option<bool>,
    }

    /// Voice server token response
    pub struct CreateVoiceUserResponse {
        /// Token for authenticating with the voice server
        pub token: String,
    }

    /// Voice state for a channel
    pub struct ChannelVoiceState {
        pub id: String,
        /// The states of the users who are connected to the channel
        pub participants: Vec<UserVoiceState>
    }

);

impl Channel {
    /// Get a reference to this channel's id
    pub fn id(&self) -> &str {
        match self {
            Channel::DirectMessage { id, .. }
            | Channel::Group { id, .. }
            | Channel::SavedMessages { id, .. }
            | Channel::TextChannel { id, .. }
            | Channel::VoiceChannel { id, .. } => id,
        }
    }
}
