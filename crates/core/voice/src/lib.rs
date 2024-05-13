use livekit_api::{access_token::{AccessToken, VideoGrants}, services::room::{CreateRoomOptions, RoomClient}};
use livekit_protocol::Room;
use redis_kiss::{get_connection, redis::Pipeline, AsyncCommands};
use revolt_database::{Channel, User};
use revolt_models::v0::{self, PartialUserVoiceState, UserVoiceState};
use revolt_permissions::{ChannelPermission, PermissionValue};
use revolt_result::{Result, ToRevoltError, create_error};
use std::time::Duration;
use revolt_config::config;

pub async fn raise_if_in_voice(user: &User, target: &str) -> Result<()> {
    let mut conn = get_connection()
        .await
        .to_internal_error()?;

    if user.bot.is_some()
    // bots can be in as many voice channels as it wants so we just check if its already connected to the one its trying to connect to
        && conn.sismember(format!("vc-{}", &user.id), target)
            .await
            .to_internal_error()?
    {
        Err(create_error!(AlreadyConnected))

    } else if conn.scard::<_, u32>(format!("vc-{}", &user.id))  // check if the current vc set is empty
        .await
        .to_internal_error()? > 0
    {
        Err(create_error!(AlreadyInVoiceChannel))
    } else {
        Ok(())
    }
}

pub fn get_allowed_sources(permissions: PermissionValue) -> Vec<String> {
    let mut allowed_sources = Vec::new();

    if permissions.has(ChannelPermission::Speak as u64) {
        allowed_sources.push("MICROPHONE".to_string())
    };

    if permissions.has(ChannelPermission::Video as u64) {
        allowed_sources.extend([
            "CAMERA".to_string(),
            "SCREEN_SHARE".to_string(),
            "SCREEN_SHARE_AUDIO".to_string()
        ]);
    };

    allowed_sources
}

pub async fn create_voice_state(channel_id: &str, server_id: Option<&str>, user_id: &str) -> Result<UserVoiceState> {
    let unique_key = format!(
        "{}-{}",
        &user_id,
        server_id.unwrap_or(channel_id)
    );

    let voice_state = UserVoiceState {
        id: user_id.to_string(),
        can_receive: true,
        can_publish: false,
        screensharing: false,
        camera: false,
    };

    Pipeline::new()
        .sadd(format!("vc-members-{channel_id}"), user_id)
        .sadd(format!("vc-{user_id}"), channel_id)
        .set(format!("can_publish-{unique_key}"), voice_state.can_publish)
        .set(format!("can_receive-{unique_key}"), voice_state.can_receive)
        .set(format!("screensharing-{unique_key}"), voice_state.screensharing)
        .set(format!("camera-{unique_key}"), voice_state.camera)
        .query_async(&mut get_connection()
            .await
            .to_internal_error()?
            .into_inner())
        .await
        .to_internal_error()?;

    Ok(voice_state)
}

pub async fn delete_voice_state(channel_id: &str, server_id: Option<&str>, user_id: &str) -> Result<()> {
    let unique_key = format!(
        "{}-{}",
        &user_id,
        server_id.unwrap_or(channel_id)
    );

    Pipeline::new()
        .srem(format!("vc-members-{channel_id}"), user_id)
        .srem(format!("vc-{user_id}"), channel_id)
        .del(&[
            format!("can_publish-{unique_key}"),
            format!("can_receive-{unique_key}"),
            format!("screensharing-{unique_key}"),
            format!("camera-{unique_key}"),
        ])
        .query_async(&mut get_connection()
            .await
            .to_internal_error()?
            .into_inner())
        .await
        .to_internal_error()?;

    Ok(())
}

pub async fn update_voice_state_tracks(channel_id: &str, server_id: Option<&str>, user_id: &str, added: bool, track: i32) -> Result<PartialUserVoiceState> {
    let partial = match track {
        /* TrackSource::Unknown */ 0 => PartialUserVoiceState::default(),
        /* TrackSource::Camera */ 1 => {
            PartialUserVoiceState {
                camera: Some(added),
                ..Default::default()
            }
        }
        /* TrackSource::Microphone */ 2 => {
            PartialUserVoiceState {
                can_publish: Some(added),
                ..Default::default()
            }
        }
        /* TrackSource::ScreenShare | TrackSource::ScreenShareAudio */ 3 | 4 => {
            PartialUserVoiceState {
                screensharing: Some(added),
                ..Default::default()
            }
        }
        _ => unreachable!(),
    };

    update_voice_state(channel_id, server_id, user_id, &partial).await?;

    Ok(partial)
}

pub async fn update_voice_state(channel_id: &str, server_id: Option<&str>, user_id: &str, partial: &PartialUserVoiceState) -> Result<()> {
    let unique_key = format!(
        "{}-{}",
        &user_id,
        server_id.unwrap_or(channel_id)
    );

    let mut pipeline = Pipeline::new();

    if let Some(camera) = &partial.camera {
        pipeline.set(format!("camera-{unique_key}"), camera);
    };

    if let Some(can_publish) = &partial.can_publish {
        pipeline.set(format!("can_publish-{unique_key}"), can_publish);
    }

    if let Some(can_receive) = &partial.can_receive {
        pipeline.set(format!("can_receive-{unique_key}"), can_receive);
    }

    if let Some(screensharing) = &partial.screensharing {
        pipeline.set(format!("screensharing-{unique_key}"), screensharing);
    }

    pipeline.query_async(&mut get_connection()
        .await
        .to_internal_error()?
        .into_inner()
    )
    .await
    .to_internal_error()?;

    Ok(())
}

pub async fn get_voice_channel_members(channel_id: &str) -> Result<Vec<String>> {
    get_connection()
        .await
        .to_internal_error()?
        .smembers::<_, Vec<String>>(format!("vc-members-{}", channel_id))
        .await
        .to_internal_error()
}

pub async fn get_voice_state(channel_id: &str, server_id: Option<&str>, user_id: &str) -> Result<Option<UserVoiceState>> {
    let unique_key = format!("{}-{user_id}", server_id.unwrap_or(channel_id));

    let (can_publish, can_receive, screensharing, camera) = get_connection()
        .await
        .to_internal_error()?
        .mget::<_, (Option<bool>, Option<bool>, Option<bool>, Option<bool>)>(&[
            format!("can_publish-{unique_key}"),
            format!("can_receive-{unique_key}"),
            format!("screensharing-{unique_key}"),
            format!("camera-{unique_key}"),
        ])
        .await
        .to_internal_error()?;

    match (can_publish, can_receive, screensharing, camera) {
        (Some(can_publish), Some(can_receive), Some(screensharing), Some(camera)) => {
            Ok(Some(v0::UserVoiceState {
                id: user_id.to_string(),
                can_receive,
                can_publish,
                screensharing,
                camera,
            }))
        },
        _ => Ok(None)
    }
}

#[derive(Debug)]
pub struct VoiceClient {
    rooms: RoomClient,
    api_key: String,
    api_secret: String
}

impl VoiceClient {
    pub fn new(url: String, api_key: String, api_secret: String) -> Self {
        Self {
            rooms: RoomClient::with_api_key(&url, &api_key, &api_secret),
            api_key,
            api_secret
        }
    }

    pub async fn from_revolt_config() -> Self {
        let config = config().await;

        Self::new(config.hosts.livekit, config.api.livekit.key, config.api.livekit.secret)
    }

    pub fn create_token(&self, user: &User, permissions: PermissionValue, channel: &Channel) -> Result<String> {
        let allowed_sources = get_allowed_sources(permissions);

        AccessToken::with_api_key(&self.api_key, &self.api_secret)
            .with_name(&format!("{}#{}", user.username, user.discriminator))
            .with_identity(&user.id)
            .with_metadata(&serde_json::to_string(&user).to_internal_error()?)
            .with_ttl(Duration::from_secs(10))
            .with_grants(VideoGrants {
                room_join: true,
                can_publish_sources: allowed_sources,
                room: channel.id().to_string(),
                ..Default::default()
            })
            .to_jwt()
            .to_internal_error()
    }

    pub async fn create_room(&self, channel: &Channel) -> Result<Room> {
        let voice = channel
            .voice()
            .ok_or_else(|| create_error!(NotAVoiceChannel))?;

        self.rooms.create_room(&channel.id(), CreateRoomOptions {
            max_participants: voice.max_users.unwrap_or(u32::MAX),
            empty_timeout: 5 * 60,  // 5 minutes
            ..Default::default()
        })
        .await
        .map_err(|_| create_error!(InternalError))
    }
}