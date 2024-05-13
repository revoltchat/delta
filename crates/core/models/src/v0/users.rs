use once_cell::sync::Lazy;
use regex::Regex;

use super::File;

#[cfg(feature = "validator")]
use validator::Validate;

/// Regex for valid usernames
///
/// Block zero width space
/// Block lookalike characters
pub static RE_USERNAME: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(\p{L}|[\d_.-])+$").unwrap());

/// Regex for valid display names
///
/// Block zero width space
/// Block newline and carriage return
pub static RE_DISPLAY_NAME: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[^\u200B\n\r]+$").unwrap());

auto_derived_partial!(
    /// User
    pub struct User {
        /// Unique Id
        #[cfg_attr(feature = "serde", serde(rename = "_id"))]
        pub id: String,
        /// Username
        pub username: String,
        /// Discriminator
        pub discriminator: String,
        /// Display name
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub display_name: Option<String>,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        /// Avatar attachment
        pub avatar: Option<File>,
        /// Relationships with other users
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "Vec::is_empty", default)
        )]
        pub relations: Vec<Relationship>,

        /// Bitfield of user badges
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "crate::if_zero_u32", default)
        )]
        pub badges: u32,
        /// User's current status
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub status: Option<UserStatus>,
        /// User's profile page
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub profile: Option<UserProfile>,

        /// Enum of user flags
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "crate::if_zero_u32", default)
        )]
        pub flags: u32,
        /// Whether this user is privileged
        #[cfg_attr(
            feature = "serde",
            serde(skip_serializing_if = "crate::if_false", default)
        )]
        pub privileged: bool,
        /// Bot information
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub bot: Option<BotInformation>,

        /// Current session user's relationship with this user
        pub relationship: RelationshipStatus,
        /// Whether this user is currently online
        pub online: bool,
    },
    "PartialUser"
);

auto_derived!(
    /// Optional fields on user object
    pub enum FieldsUser {
        Avatar,
        StatusText,
        StatusPresence,
        ProfileContent,
        ProfileBackground,
    }

    /// User's relationship with another user (or themselves)
    #[derive(Default)]
    pub enum RelationshipStatus {
        /// No relationship with other user
        #[default]
        None,
        /// Other user is us
        User,
        /// Friends with the other user
        Friend,
        /// Pending friend request to user
        Outgoing,
        /// Incoming friend request from user
        Incoming,
        /// Blocked this user
        Blocked,
        /// Blocked by this user
        BlockedOther,
    }

    /// Relationship entry indicating current status with other user
    pub struct Relationship {
        /// Other user's Id
        #[cfg_attr(feature = "serde", serde(rename = "_id"))]
        pub user_id: String,
        /// Relationship status with them
        pub status: RelationshipStatus,
    }

    /// Presence status
    pub enum Presence {
        /// User is online
        Online,
        /// User is not currently available
        Idle,
        /// User is focusing / will only receive mentions
        Focus,
        /// User is busy / will not receive any notifications
        Busy,
        /// User appears to be offline
        Invisible,
    }

    /// User's active status
    #[derive(Default)]
    #[cfg_attr(feature = "validator", derive(Validate))]
    pub struct UserStatus {
        /// Custom status text
        #[validate(length(min = 0, max = 128))]
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub text: Option<String>,
        /// Current presence option
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub presence: Option<Presence>,
    }

    /// User's profile
    #[derive(Default)]
    #[cfg_attr(feature = "validator", derive(Validate))]
    pub struct UserProfile {
        /// Text content on user's profile
        #[validate(length(min = 0, max = 2000))]
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub content: Option<String>,
        /// Background visible on user's profile
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub background: Option<File>,
    }

    /// User badge bitfield
    #[repr(u32)]
    pub enum UserBadges {
        /// Revolt Developer
        Developer = 1,
        /// Helped translate Revolt
        Translator = 2,
        /// Monetarily supported Revolt
        Supporter = 4,
        /// Responsibly disclosed a security issue
        ResponsibleDisclosure = 8,
        /// Revolt Founder
        Founder = 16,
        /// Platform moderator
        PlatformModeration = 32,
        /// Active monetary supporter
        ActiveSupporter = 64,
        /// 🦊🦝
        Paw = 128,
        /// Joined as one of the first 1000 users in 2021
        EarlyAdopter = 256,
        /// Amogus
        ReservedRelevantJokeBadge1 = 512,
        /// Low resolution troll face
        ReservedRelevantJokeBadge2 = 1024,
    }

    /// User flag enum
    #[repr(u32)]
    pub enum UserFlags {
        /// User has been suspended from the platform
        Suspended = 1,
        /// User has deleted their account
        Deleted = 2,
        /// User was banned off the platform
        Banned = 4,
        /// User was marked as spam and removed from platform
        Spam = 8,
    }

    /// New user profile data
    #[cfg_attr(feature = "validator", derive(Validate))]
    pub struct DataUserProfile {
        /// Text to set as user profile description
        #[cfg_attr(feature = "validator", validate(length(min = 0, max = 2000)))]
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub content: Option<String>,
        /// Attachment Id for background
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 128)))]
        pub background: Option<String>,
    }

    /// New user information
    #[cfg_attr(feature = "validator", derive(Validate))]
    pub struct DataEditUser {
        /// New display name
        #[cfg_attr(
            feature = "validator",
            validate(length(min = 2, max = 32), regex = "RE_DISPLAY_NAME")
        )]
        pub display_name: Option<String>,
        /// Attachment Id for avatar
        #[cfg_attr(feature = "validator", validate(length(min = 1, max = 128)))]
        pub avatar: Option<String>,

        /// New user status
        #[cfg_attr(feature = "validator", validate)]
        pub status: Option<UserStatus>,
        /// New user profile data
        ///
        /// This is applied as a partial.
        #[cfg_attr(feature = "validator", validate)]
        pub profile: Option<DataUserProfile>,

        /// Bitfield of user badges
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub badges: Option<i32>,
        /// Enum of user flags
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        pub flags: Option<i32>,

        /// Fields to remove from user object
        #[cfg_attr(feature = "validator", validate(length(min = 1)))]
        pub remove: Option<Vec<FieldsUser>>,
    }

    /// User flag reponse
    pub struct FlagResponse {
        /// Flags
        pub flags: i32,
    }

    /// Mutual friends and servers response
    pub struct MutualResponse {
        /// Array of mutual user IDs that both users are friends with
        pub users: Vec<String>,
        /// Array of mutual server IDs that both users are in
        pub servers: Vec<String>,
    }

    /// Bot information for if the user is a bot
    pub struct BotInformation {
        /// Id of the owner of this bot
        #[cfg_attr(feature = "serde", serde(rename = "owner"))]
        pub owner_id: String,
    }

    /// User lookup information
    pub struct DataSendFriendRequest {
        /// Username and discriminator combo separated by #
        pub username: String,
    }
);

auto_derived_partial!(
    /// Voice State information for a user
    pub struct UserVoiceState {
        pub id: String,
        pub can_receive: bool,
        pub can_publish: bool,
        pub screensharing: bool,
        pub camera: bool,
    },
    "PartialUserVoiceState"
);

pub trait CheckRelationship {
    fn with(&self, user: &str) -> RelationshipStatus;
}

impl CheckRelationship for Vec<Relationship> {
    fn with(&self, user: &str) -> RelationshipStatus {
        for entry in self {
            if entry.user_id == user {
                return entry.status.clone();
            }
        }

        RelationshipStatus::None
    }
}
