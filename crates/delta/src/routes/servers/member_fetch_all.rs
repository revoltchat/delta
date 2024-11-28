use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, User,
};
use revolt_models::v0;
use revolt_permissions::{calculate_channel_permissions, PermissionQuery};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};

/// # Fetch Members
///
/// Fetch all server members.
#[openapi(tag = "Server Members")]
#[get("/<target>/members?<options..>")]
pub async fn fetch_all(
    db: &State<Database>,
    user: User,
    target: Reference,
    options: v0::OptionsFetchAllMembers,
) -> Result<Json<v0::AllMemberResponse>> {
    let server = target.as_server(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
    if !query.are_we_a_member().await {
        return Err(create_error!(NotFound));
    }

    let permissions = calculate_channel_permissions(&mut query).await;
    let mut members = db.fetch_all_members(&server.id).await?;

    let user_ids: Vec<String> = members
        .iter()
        .map(|member| member.id.user.clone())
        .collect();

    let mut users = User::fetch_many_ids_as_mutuals(db, &user, &user_ids).await?;

    // Ensure the lists match up exactly.
    members.sort_by(|a, b| a.id.user.cmp(&b.id.user));
    users.sort_by(|a, b| a.id.cmp(&b.id));

    // Optionally, remove all offline user entries.
    if let Some(true) = options.exclude_offline {
        let mut iter = users.iter();
        members.retain(|_| iter.next().unwrap().online);
        users.retain(|user| user.online);
    }

    Ok(Json(v0::AllMemberResponse {
        members: members
            .into_iter()
            .map(|m| m.into(Some(permissions)))
            .collect(),
        users,
    }))
}
