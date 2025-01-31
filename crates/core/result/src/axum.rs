use axum::{http::StatusCode, response::IntoResponse, Json};

use crate::{Error, ErrorType};

/// HTTP response builder for Error enum
impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let status = match self.error_type {
            ErrorType::LabelMe => StatusCode::INTERNAL_SERVER_ERROR,

            ErrorType::AlreadyOnboarded => StatusCode::FORBIDDEN,

            ErrorType::UnknownUser => StatusCode::NOT_FOUND,
            ErrorType::InvalidUsername => StatusCode::BAD_REQUEST,
            ErrorType::UsernameTaken => StatusCode::CONFLICT,
            ErrorType::DiscriminatorChangeRatelimited => StatusCode::TOO_MANY_REQUESTS,
            ErrorType::AlreadyFriends => StatusCode::CONFLICT,
            ErrorType::AlreadySentRequest => StatusCode::CONFLICT,
            ErrorType::Blocked => StatusCode::CONFLICT,
            ErrorType::BlockedByOther => StatusCode::FORBIDDEN,
            ErrorType::NotFriends => StatusCode::FORBIDDEN,
            ErrorType::TooManyPendingFriendRequests { .. } => StatusCode::BAD_REQUEST,

            ErrorType::UnknownChannel => StatusCode::NOT_FOUND,
            ErrorType::UnknownMessage => StatusCode::NOT_FOUND,
            ErrorType::UnknownAttachment => StatusCode::BAD_REQUEST,
            ErrorType::CannotEditMessage => StatusCode::FORBIDDEN,
            ErrorType::CannotJoinCall => StatusCode::BAD_REQUEST,
            ErrorType::TooManyAttachments { .. } => StatusCode::BAD_REQUEST,
            ErrorType::TooManyReplies { .. } => StatusCode::BAD_REQUEST,
            ErrorType::EmptyMessage => StatusCode::UNPROCESSABLE_ENTITY,
            ErrorType::PayloadTooLarge => StatusCode::UNPROCESSABLE_ENTITY,
            ErrorType::CannotRemoveYourself => StatusCode::BAD_REQUEST,
            ErrorType::GroupTooLarge { .. } => StatusCode::FORBIDDEN,
            ErrorType::AlreadyInGroup => StatusCode::CONFLICT,
            ErrorType::NotInGroup => StatusCode::NOT_FOUND,
            ErrorType::AlreadyPinned => StatusCode::BAD_REQUEST,
            ErrorType::NotPinned => StatusCode::BAD_REQUEST,

            ErrorType::UnknownServer => StatusCode::NOT_FOUND,
            ErrorType::InvalidRole => StatusCode::NOT_FOUND,
            ErrorType::Banned => StatusCode::FORBIDDEN,
            ErrorType::AlreadyInServer => StatusCode::CONFLICT,

            ErrorType::TooManyServers { .. } => StatusCode::BAD_REQUEST,
            ErrorType::TooManyEmbeds { .. } => StatusCode::BAD_REQUEST,
            ErrorType::TooManyEmoji { .. } => StatusCode::BAD_REQUEST,
            ErrorType::TooManyChannels { .. } => StatusCode::BAD_REQUEST,
            ErrorType::TooManyRoles { .. } => StatusCode::BAD_REQUEST,

            ErrorType::ReachedMaximumBots => StatusCode::BAD_REQUEST,
            ErrorType::IsBot => StatusCode::BAD_REQUEST,
            ErrorType::IsNotBot => StatusCode::BAD_REQUEST,
            ErrorType::BotIsPrivate => StatusCode::FORBIDDEN,

            ErrorType::CannotReportYourself => StatusCode::BAD_REQUEST,

            ErrorType::MissingPermission { .. } => StatusCode::FORBIDDEN,
            ErrorType::MissingUserPermission { .. } => StatusCode::FORBIDDEN,
            ErrorType::NotElevated => StatusCode::FORBIDDEN,
            ErrorType::NotPrivileged => StatusCode::FORBIDDEN,
            ErrorType::CannotGiveMissingPermissions => StatusCode::FORBIDDEN,
            ErrorType::NotOwner => StatusCode::FORBIDDEN,

            ErrorType::DatabaseError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorType::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorType::InvalidOperation => StatusCode::BAD_REQUEST,
            ErrorType::InvalidCredentials => StatusCode::UNAUTHORIZED,
            ErrorType::InvalidProperty => StatusCode::BAD_REQUEST,
            ErrorType::InvalidSession => StatusCode::UNAUTHORIZED,
            ErrorType::NotAuthenticated => StatusCode::UNAUTHORIZED,
            ErrorType::DuplicateNonce => StatusCode::CONFLICT,
            ErrorType::VosoUnavailable => StatusCode::BAD_REQUEST,
            ErrorType::NotFound => StatusCode::NOT_FOUND,
            ErrorType::NoEffect => StatusCode::OK,
            ErrorType::FailedValidation { .. } => StatusCode::BAD_REQUEST,
            ErrorType::InvalidFlagValue => StatusCode::BAD_REQUEST,
            ErrorType::FeatureDisabled { .. } => StatusCode::BAD_REQUEST,

            ErrorType::ProxyError => StatusCode::BAD_REQUEST,
            ErrorType::FileTooSmall => StatusCode::UNPROCESSABLE_ENTITY,
            ErrorType::FileTooLarge { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            ErrorType::FileTypeNotAllowed => StatusCode::BAD_REQUEST,
            ErrorType::ImageProcessingFailed => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorType::NoEmbedData => StatusCode::BAD_REQUEST,
        };

        (status, Json(&self)).into_response()
    }
}
