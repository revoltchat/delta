use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, FieldsMessage, PartialMessage, SystemMessage, User,
};
use revolt_models::v0::MessageAuthor;
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::{create_error, Result};
use rocket::State;
use rocket_empty::EmptyResponse;

/// # Unpins a message
///
/// Unpins a message by its id.
#[openapi(tag = "Messaging")]
#[delete("/<target>/messages/<msg>/pin")]
pub async fn message_unpin(
    db: &State<Database>,
    user: User,
    target: Reference,
    msg: Reference,
) -> Result<EmptyResponse> {
    let channel = target.as_channel(db).await?;

    let mut query = DatabasePermissionQuery::new(db, &user).channel(&channel);
    calculate_channel_permissions(&mut query)
        .await
        .throw_if_lacking_channel_permission(ChannelPermission::ManageMessages)?;

    let mut message = msg.as_message_in_channel(db, channel.id()).await?;

    if !message.pinned.unwrap_or_default() {
        return Err(create_error!(NotPinned));
    }

    message
        .update(db, PartialMessage::default(), vec![FieldsMessage::Pinned])
        .await?;

    SystemMessage::MessageUnpinned {
        id: message.id.clone(),
        by: user.id.clone(),
    }
    .into_message(channel.id().to_string())
    .send(
        db,
        MessageAuthor::System {
            username: &user.username,
            avatar: user.avatar.as_ref().map(|file| file.id.as_ref()),
        },
        None,
        None,
        &channel,
        false,
    )
    .await?;

    Ok(EmptyResponse)
}

#[cfg(test)]
mod test {
    use crate::{rocket, util::test::TestHarness};
    use revolt_database::{
        events::client::EventV1,
        util::{idempotency::IdempotencyKey, reference::Reference},
        Member, Message, PartialMessage, Server,
    };
    use revolt_models::v0::{self, FieldsMessage, SystemMessage};
    use rocket::http::{Header, Status};

    #[rocket::async_test]
    async fn unpin_message() {
        let mut harness = TestHarness::new().await;
        let (_, session, user) = harness.new_user().await;

        let (server, channels) = Server::create(
            &harness.db,
            v0::DataCreateServer {
                name: "Test Server".to_string(),
                ..Default::default()
            },
            &user,
            true,
        )
        .await
        .expect("Failed to create test server");

        let channel = &channels[0];

        Member::create(&harness.db, &server, &user, Some(channels.clone()), None)
            .await
            .expect("Failed to create member");
        let member = Reference::from_unchecked(user.id.clone())
            .as_member(&harness.db, &server.id)
            .await
            .expect("Failed to get member");

        let message = Message::create_from_api(
            &harness.db,
            channel.clone(),
            v0::DataMessageSend {
                content: Some("Test message".to_string()),
                nonce: None,
                attachments: None,
                replies: None,
                embeds: None,
                masquerade: None,
                interactions: None,
                flags: None,
            },
            v0::MessageAuthor::User(&user.clone().into(&harness.db, Some(&user)).await),
            Some(user.clone().into(&harness.db, Some(&user)).await),
            Some(member.into(None)),
            user.limits().await,
            IdempotencyKey::unchecked_from_string("0".to_string()),
            false,
            false,
        )
        .await
        .expect("Failed to create message");

        harness
            .db
            .update_message(
                &message.id,
                &PartialMessage {
                    pinned: Some(true),
                    ..Default::default()
                },
                vec![],
            )
            .await
            .expect("Failed to update message");

        let response = harness
            .client
            .delete(format!(
                "/channels/{}/messages/{}/pin",
                channel.id(),
                &message.id
            ))
            .header(Header::new("x-session-token", session.token.to_string()))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::NoContent);
        drop(response);

        harness
            .wait_for_event(channel.id(), |event| match event {
                EventV1::Message(message) => match &message.system {
                    Some(SystemMessage::MessageUnpinned { by, .. }) => {
                        assert_eq!(by, &user.id);

                        true
                    }
                    _ => false,
                },
                _ => false,
            })
            .await;

        harness
            .wait_for_event(channel.id(), |event| match event {
                EventV1::MessageUpdate { id, clear, .. } => {
                    assert_eq!(&message.id, id);
                    assert_eq!(clear, &[FieldsMessage::Pinned]);

                    true
                }
                _ => false,
            })
            .await;

        let updated_message = Reference::from_unchecked(message.id)
            .as_message(&harness.db)
            .await
            .expect("Failed to find updated message");

        assert_eq!(updated_message.pinned, None);
    }
}
