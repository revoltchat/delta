use revolt_database::util::permissions::DatabasePermissionQuery;
use revolt_database::Member;
use revolt_database::{util::reference::Reference, Database, User};
use revolt_models::v0;
use revolt_permissions::{
    calculate_channel_permissions, calculate_server_permissions, ChannelPermission,
};
use revolt_result::{create_error, Result};
use rocket::State;

use rocket::serde::json::Json;
use rocket_empty::EmptyResponse;

/// # Invite Bot
///
/// Invite a bot to a server or group by its id.`
#[openapi(tag = "Bots")]
#[post("/<target>/invite", data = "<dest>")]
pub async fn invite_bot(
    db: &State<Database>,
    user: User,
    target: Reference,
    dest: Json<v0::InviteBotDestination>,
) -> Result<EmptyResponse> {
    if user.bot.is_some() {
        return Err(create_error!(IsBot));
    }

    let bot = target.as_bot(db).await?;
    if !bot.public && bot.owner != user.id {
        return Err(create_error!(BotIsPrivate));
    }

    let bot_user = db.fetch_user(&bot.id).await?;

    match dest.into_inner() {
        v0::InviteBotDestination::Server { server } => {
            let server = db.fetch_server(&server).await?;

            let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
            calculate_server_permissions(&mut query)
                .await
                .throw_if_lacking_channel_permission(ChannelPermission::ManageServer)?;

            Member::create(db, &server, &bot_user, None, None)
                .await
                .map(|_| EmptyResponse)
        }
        v0::InviteBotDestination::Group { group } => {
            let mut channel = db.fetch_channel(&group).await?;

            let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
            calculate_channel_permissions(&mut query)
                .await
                .throw_if_lacking_channel_permission(ChannelPermission::InviteOthers)?;

            channel
                .add_user_to_group(db, &bot_user, &user.id)
                .await
                .map(|_| EmptyResponse)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{events::client::EventV1, Bot, Channel, Server};
    use revolt_models::v0::{self, DataCreateServer};
    use rocket::http::{ContentType, Header, Status};

    #[rocket::async_test]
    async fn invite_bot_to_group() {
        let mut harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;

        let (bot, _) = Bot::create(&harness.db, TestHarness::rand_string(), &user, None)
            .await
            .expect("`Bot`");

        let group = Channel::create_group(
            &harness.db,
            v0::DataCreateGroup {
                name: TestHarness::rand_string(),
                ..Default::default()
            },
            user.id.to_string(),
        )
        .await
        .unwrap();

        let response = harness
            .client
            .post(format!("/bots/{}/invite", bot.id))
            .header(ContentType::JSON)
            .body(json!(v0::InviteBotDestination::Group { group: group.id() }).to_string())
            .header(Header::new("x-session-token", session.token.to_string()))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::NoContent);
        drop(response);

        let event = harness
            .wait_for_event(&group.id(), |event| match event {
                EventV1::ChannelGroupJoin { id, .. } => id == &group.id(),
                _ => false,
            })
            .await;

        match event {
            EventV1::ChannelGroupJoin { user, .. } => {
                assert_eq!(bot.id, user);
            }
            _ => unreachable!(),
        }
    }

    #[rocket::async_test]
    async fn invite_bot_to_server() {
        let mut harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;

        let (bot, _) = Bot::create(&harness.db, TestHarness::rand_string(), &user, None)
            .await
            .expect("`Bot`");

        let (server, _) = Server::create(
            &harness.db,
            DataCreateServer {
                name: TestHarness::rand_string(),
                ..Default::default()
            },
            &user,
            false,
        )
        .await
        .unwrap();

        let response = harness
            .client
            .post(format!("/bots/{}/invite", bot.id))
            .header(ContentType::JSON)
            .body(
                json!(v0::InviteBotDestination::Server {
                    server: server.id.to_string()
                })
                .to_string(),
            )
            .header(Header::new("x-session-token", session.token.to_string()))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::NoContent);
        drop(response);

        let event = harness
            .wait_for_event(&server.id, |event| match event {
                EventV1::ServerMemberJoin { id, .. } => id == &server.id,
                _ => false,
            })
            .await;

        match event {
            EventV1::ServerMemberJoin { user, .. } => {
                assert_eq!(bot.id, user);
            }
            _ => unreachable!(),
        }
    }
}
