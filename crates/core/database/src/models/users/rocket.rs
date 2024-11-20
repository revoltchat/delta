use authifier::models::Session;
use rocket::http::Status;
use rocket::request::{self, FromRequest, Outcome, Request};

use crate::{Database, User};

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = authifier::Error;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let user: &Option<User> = request
            .local_cache_async(async {
                let db = request.rocket().state::<Database>().expect("`Database`");

                let header_bot_token = request
                    .headers()
                    .get("x-bot-token")
                    .next()
                    .map(|x| x.to_string());

                if let Some(bot_token) = header_bot_token {
                    if let Ok(bot) = db.fetch_bot_by_token(&bot_token).await {
                        if let Ok(user) = db.fetch_user(&bot.id).await {
                            return Some(user);
                        }
                    }
                } else if let Outcome::Success(session) = request.guard::<Session>().await {
                    if let Ok(user) = db.fetch_user(&session.user_id).await {
                        return Some(user);
                    }
                }

                None
            })
            .await;

        if let Some(user) = user {
            Outcome::Success(user.clone())
        } else {
            Outcome::Error((Status::Unauthorized, authifier::Error::InvalidSession))
        }
    }
}
