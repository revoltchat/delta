use revolt_models::v0;
use revolt_database::{util::{permissions::perms, reference::Reference}, Database, User};
use revolt_permissions::{calculate_channel_permissions, ChannelPermission};
use revolt_result::Result;
use revolt_voice::{VoiceClient, raise_if_in_voice};

use rocket::{serde::json::Json, State};

/// # Join Call
///
/// Asks the voice server for a token to join the call.
#[openapi(tag = "Voice")]
#[post("/<target>/join_call")]
pub async fn call(db: &State<Database>, voice: &State<VoiceClient>, user: User, target: Reference) -> Result<Json<v0::CreateVoiceUserResponse>> {
    let channel = target.as_channel(db).await?;

    raise_if_in_voice(&user, &channel.id()).await?;

    let mut permissions = perms(db, &user).channel(&channel);

    let current_permissions = calculate_channel_permissions(&mut permissions).await;
    current_permissions.throw_if_lacking_channel_permission(ChannelPermission::Connect)?;

    let token = voice.create_token(&user, current_permissions, &channel)?;
    let room = voice.create_room(&channel).await?;

    log::debug!("created room {}", room.name);

    Ok(Json(v0::CreateVoiceUserResponse { token }))
}
