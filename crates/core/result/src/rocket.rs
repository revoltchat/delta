use std::io::Cursor;

use rocket::{
    http::{ContentType, Status},
    response::{self, Responder},
    Request, Response,
};

use crate::{Error, ErrorType};

/// HTTP response builder for Error enum
impl<'r> Responder<'r, 'static> for Error {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        let status = match self.error_type {
            ErrorType::LabelMe => Status::InternalServerError,

            ErrorType::AlreadyOnboarded => Status::Forbidden,

            ErrorType::UnknownUser => Status::NotFound,
            ErrorType::InvalidUsername => Status::BadRequest,
            ErrorType::UsernameTaken => Status::Conflict,
            ErrorType::DiscriminatorChangeRatelimited => Status::TooManyRequests,
            ErrorType::AlreadyFriends => Status::Conflict,
            ErrorType::AlreadySentRequest => Status::Conflict,
            ErrorType::Blocked => Status::Conflict,
            ErrorType::BlockedByOther => Status::Forbidden,
            ErrorType::NotFriends => Status::Forbidden,
            ErrorType::TooManyPendingFriendRequests { .. } => Status::BadRequest,

            ErrorType::UnknownChannel => Status::NotFound,
            ErrorType::UnknownMessage => Status::NotFound,
            ErrorType::UnknownAttachment => Status::BadRequest,
            ErrorType::CannotEditMessage => Status::Forbidden,
            ErrorType::CannotJoinCall => Status::BadRequest,
            ErrorType::TooManyAttachments { .. } => Status::BadRequest,
            ErrorType::TooManyReplies { .. } => Status::BadRequest,
            ErrorType::EmptyMessage => Status::UnprocessableEntity,
            ErrorType::PayloadTooLarge => Status::UnprocessableEntity,
            ErrorType::CannotRemoveYourself => Status::BadRequest,
            ErrorType::GroupTooLarge { .. } => Status::Forbidden,
            ErrorType::AlreadyInGroup => Status::Conflict,
            ErrorType::NotInGroup => Status::NotFound,
            ErrorType::AlreadyPinned => Status::BadRequest,
            ErrorType::NotPinned => Status::BadRequest,
            ErrorType::InvalidFlagValue => Status::BadRequest,

            ErrorType::UnknownServer => Status::NotFound,
            ErrorType::InvalidRole => Status::NotFound,
            ErrorType::Banned => Status::Forbidden,
            ErrorType::AlreadyInServer => Status::Conflict,
            ErrorType::CannotTimeoutYourself => Status::BadRequest,

            ErrorType::TooManyServers { .. } => Status::BadRequest,
            ErrorType::TooManyEmbeds { .. } => Status::BadRequest,
            ErrorType::TooManyEmoji { .. } => Status::BadRequest,
            ErrorType::TooManyChannels { .. } => Status::BadRequest,
            ErrorType::TooManyRoles { .. } => Status::BadRequest,

            ErrorType::ReachedMaximumBots => Status::BadRequest,
            ErrorType::IsBot => Status::BadRequest,
            ErrorType::IsNotBot => Status::BadRequest,
            ErrorType::BotIsPrivate => Status::Forbidden,

            ErrorType::CannotReportYourself => Status::BadRequest,

            ErrorType::MissingPermission { .. } => Status::Forbidden,
            ErrorType::MissingUserPermission { .. } => Status::Forbidden,
            ErrorType::NotElevated => Status::Forbidden,
            ErrorType::NotPrivileged => Status::Forbidden,
            ErrorType::CannotGiveMissingPermissions => Status::Forbidden,
            ErrorType::NotOwner => Status::Forbidden,

            ErrorType::DatabaseError { .. } => Status::InternalServerError,
            ErrorType::InternalError => Status::InternalServerError,
            ErrorType::InvalidOperation => Status::BadRequest,
            ErrorType::InvalidCredentials => Status::Unauthorized,
            ErrorType::InvalidProperty => Status::BadRequest,
            ErrorType::InvalidSession => Status::Unauthorized,
            ErrorType::NotAuthenticated => Status::Unauthorized,
            ErrorType::DuplicateNonce => Status::Conflict,
            ErrorType::VosoUnavailable => Status::BadRequest,
            ErrorType::NotFound => Status::NotFound,
            ErrorType::NoEffect => Status::Ok,
            ErrorType::FailedValidation { .. } => Status::BadRequest,
            ErrorType::FeatureDisabled { .. } => Status::BadRequest,

            ErrorType::ProxyError => Status::BadRequest,
            ErrorType::FileTooSmall => Status::UnprocessableEntity,
            ErrorType::FileTooLarge { .. } => Status::UnprocessableEntity,
            ErrorType::FileTypeNotAllowed => Status::BadRequest,
            ErrorType::ImageProcessingFailed => Status::InternalServerError,
            ErrorType::NoEmbedData => Status::BadRequest,
        };

        // Serialize the error data structure into JSON.
        let string = serde_json::to_string(&self).unwrap();

        // Build and send the request.
        Response::build()
            .sized_body(string.len(), Cursor::new(string))
            .header(ContentType::new("application", "json"))
            .status(status)
            .ok()
    }
}
