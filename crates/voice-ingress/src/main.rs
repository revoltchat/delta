use std::env;


use livekit_protocol::WebhookEvent;
use revolt_database::{
    events::client::EventV1, util::reference::Reference, Database, DatabaseInfo,
};
use revolt_voice::{create_voice_state, delete_voice_state, update_voice_state_tracks, VoiceClient};
use rocket::{build, post, routes, serde::json::Json, Config, State};
use rocket_empty::EmptyResponse;

use revolt_result::{Result, ToRevoltError};

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    revolt_config::configure!(voice_ingress);

    let database = DatabaseInfo::Auto.connect().await.unwrap();
    let voice_client = VoiceClient::from_revolt_config().await;

    let _rocket = build()
        .manage(database)
        .manage(voice_client)
        .mount("/", routes![ingress])
        .configure(Config {
            port: 8500,
            ..Default::default()
        })
        .ignite()
        .await?
        .launch()
        .await?;

    Ok(())
}

#[post("/", data = "<body>")]
async fn ingress(db: &State<Database>, voice_client: &State<VoiceClient>, body: Json<WebhookEvent>) -> Result<EmptyResponse> {
    log::debug!("received event: {body:?}");

    let channel_id = body.room.as_ref().map(|r| &r.name);
    let user_id = body.participant.as_ref().map(|r| &r.identity);

    match body.event.as_str() {
        "participant_joined" => {
            let channel_id = channel_id.to_internal_error()?;
            let user_id = user_id.to_internal_error()?;

            let channel = Reference::from_unchecked(channel_id.clone())
                .as_channel(db)
                .await?;

            let voice_state = create_voice_state(channel_id, channel.server().as_deref(), user_id).await?;

            EventV1::VoiceChannelJoin {
                id: channel_id.clone(),
                state: voice_state,
            }
            .p(channel_id.clone())
            .await
        }
        "participant_left" => {
            let channel_id = channel_id.to_internal_error()?;
            let user_id = user_id.to_internal_error()?;

            let channel = Reference::from_unchecked(channel_id.clone())
                .as_channel(db)
                .await?;

            delete_voice_state(channel_id, channel.server().as_deref(), user_id).await?;

            EventV1::VoiceChannelLeave {
                id: channel_id.clone(),
                user: user_id.clone(),
            }
            .p(channel_id.clone())
            .await
        }
        "track_published" | "track_unpublished" => {
            let channel_id = channel_id.to_internal_error()?;
            let user_id = user_id.to_internal_error()?;
            let track = body.track.as_ref().to_internal_error()?;

            let channel = Reference::from_unchecked(channel_id.clone())
                .as_channel(db)
                .await?;

            let partial = update_voice_state_tracks(
                channel_id,
                channel.server().as_deref(),
                user_id,
                body.event == "track_published",  // to avoid duplicating this entire case twice
                track.source
            ).await?;

            EventV1::UserVoiceStateUpdate {
                id: user_id.clone(),
                channel_id: channel_id.clone(),
                data: partial,
            }
            .p(channel_id.clone())
            .await;
        }
        _ => {}
    };

    Ok(EmptyResponse)
}
