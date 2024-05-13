use std::panic::Location;

#[cfg(feature = "serde")]
#[macro_use]
extern crate serde;

#[cfg(feature = "schemas")]
#[macro_use]
extern crate schemars;

#[cfg(feature = "rocket")]
pub mod rocket;

#[cfg(feature = "okapi")]
pub mod okapi;

/// Result type with custom Error
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Error information
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[derive(Debug, Clone)]
pub struct Error {
    /// Type of error and additional information
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub error_type: ErrorType,

    /// Where this error occurred
    pub location: String,
}

/// Possible error types
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[derive(Debug, Clone)]
pub enum ErrorType {
    /// This error was not labeled :(
    LabelMe,

    // ? Onboarding related errors
    AlreadyOnboarded,

    // ? User related errors
    UsernameTaken,
    InvalidUsername,
    DiscriminatorChangeRatelimited,
    UnknownUser,
    AlreadyFriends,
    AlreadySentRequest,
    Blocked,
    BlockedByOther,
    NotFriends,

    // ? Channel related errors
    UnknownChannel,
    UnknownAttachment,
    UnknownMessage,
    CannotEditMessage,
    CannotJoinCall,
    TooManyAttachments {
        max: usize,
    },
    TooManyEmbeds {
        max: usize,
    },
    TooManyReplies {
        max: usize,
    },
    TooManyChannels {
        max: usize,
    },
    EmptyMessage,
    PayloadTooLarge,
    CannotRemoveYourself,
    GroupTooLarge {
        max: usize,
    },
    AlreadyInGroup,
    NotInGroup,

    // ? Server related errors
    UnknownServer,
    InvalidRole,
    Banned,
    TooManyServers {
        max: usize,
    },
    TooManyEmoji {
        max: usize,
    },
    TooManyRoles {
        max: usize,
    },
    AlreadyInServer,

    // ? Bot related errors
    ReachedMaximumBots,
    IsBot,
    BotIsPrivate,

    // ? User safety related errors
    CannotReportYourself,

    // ? Permission errors
    MissingPermission {
        permission: String,
    },
    MissingUserPermission {
        permission: String,
    },
    NotElevated,
    NotPrivileged,
    CannotGiveMissingPermissions,
    NotOwner,

    // ? General errors
    DatabaseError {
        operation: String,
        collection: String,
    },
    InternalError,
    InvalidOperation,
    InvalidCredentials,
    InvalidProperty,
    InvalidSession,
    DuplicateNonce,
    NotFound,
    NoEffect,
    FailedValidation {
        error: String,
    },

    // ? Voice errors
    LiveKitUnavailable,
    AlreadyInVoiceChannel,
    NotAVoiceChannel,
    AlreadyConnected
}

#[macro_export]
macro_rules! create_error {
    ( $error: ident $( $tt:tt )? ) => {
        $crate::Error {
            error_type: $crate::ErrorType::$error $( $tt )?,
            location: format!("{}:{}:{}", file!(), line!(), column!()),
        }
    };
}

#[macro_export]
macro_rules! create_database_error {
    ( $operation: expr, $collection: expr ) => {
        $crate::create_error!(DatabaseError {
            operation: $operation.to_string(),
            collection: $collection.to_string()
        })
    };
}

#[macro_export]
#[cfg(debug_assertions)]
macro_rules! query {
    ( $self: ident, $type: ident, $collection: expr, $($rest:expr),+ ) => {
        Ok($self.$type($collection, $($rest),+).await.unwrap())
    };
}

#[macro_export]
#[cfg(not(debug_assertions))]
macro_rules! query {
    ( $self: ident, $type: ident, $collection: expr, $($rest:expr),+ ) => {
        $self.$type($collection, $($rest),+).await
            .map_err(|_| create_database_error!(stringify!($type), $collection))
    };
}

pub trait ToRevoltError<T> {
    #[track_caller]
    fn to_internal_error(self) -> Result<T, Error>;
}

impl<T, E> ToRevoltError<T> for Result<T, E> {
    fn to_internal_error(self) -> Result<T, Error> {
        self.map_err(|_| {
            let loc = Location::caller();

            Error {
                error_type: ErrorType::InternalError,
                location: format!("{}:{}:{}", loc.file(), loc.line(), loc.column())
            }
        })
    }
}

impl<T> ToRevoltError<T> for Option<T> {
    fn to_internal_error(self) -> Result<T, Error> {
        self.ok_or_else(|| {
            let loc = Location::caller();

            Error {
                error_type: ErrorType::InternalError,
                location: format!("{}:{}:{}", loc.file(), loc.line(), loc.column())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::ErrorType;

    #[test]
    fn use_macro_to_construct_error() {
        let error = create_error!(LabelMe);
        assert!(matches!(error.error_type, ErrorType::LabelMe));
    }

    #[test]
    fn use_macro_to_construct_complex_error() {
        let error = create_error!(LabelMe);
        assert!(matches!(error.error_type, ErrorType::LabelMe));
    }
}
