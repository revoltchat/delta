use futures::future::join_all;
use revolt_database::{
    util::{permissions::DatabasePermissionQuery, reference::Reference},
    Database, Member, User,
};
use revolt_models::v0;
use revolt_permissions::{calculate_channel_permissions, PermissionQuery};
use revolt_result::{create_error, Result};
use rocket::{serde::json::Json, State};
use serde::{Deserialize, Serialize};

/// # Query Parameters
#[derive(Deserialize, JsonSchema, FromForm)]
pub struct OptionsQueryMembers {
    /// String to search for
    query: String,

    /// Discourage use of this API
    experimental_api: bool,
}

/// # Query members by name
#[derive(Serialize, JsonSchema)]
pub struct MemberQueryResponse {
    /// List of members
    members: Vec<v0::Member>,
    /// List of users
    users: Vec<v0::User>,
}

/// # Query members by name
///
/// Query members by a given name, this API is not stable and will be removed in the future.
#[openapi(tag = "Server Members")]
#[get("/<target>/members_experimental_query?<options..>")]
pub async fn member_experimental_query(
    db: &State<Database>,
    user: User,
    target: Reference,
    options: OptionsQueryMembers,
) -> Result<Json<MemberQueryResponse>> {
    if !options.experimental_api {
        return Err(create_error!(InternalError));
    }

    let server = target.as_server(db).await?;
    let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
    if !query.are_we_a_member().await {
        return Err(create_error!(NotFound));
    }

    let permissions = calculate_channel_permissions(&mut query).await;
    let mut members = db.fetch_all_members(&server.id).await?;

    let mut user_ids = vec![];
    for member in &members {
        user_ids.push(member.id.user.clone());
    }

    let mut users = db.fetch_users(&user_ids).await?;

    // Ensure the lists match up exactly
    members.sort_by(|a, b| a.id.user.cmp(&b.id.user));
    users.sort_by(|a, b| a.id.cmp(&b.id));

    // Filter all matches
    let mut zipped_vec: Vec<(Member, User)> = members
        .into_iter()
        .zip(users)
        .filter(|(member, user)| {
            if let Some(nickname) = &member.nickname {
                nickname.contains(&options.query)
            } else {
                user.username.contains(&options.query)
            }
        })
        .collect();

    // Sort remaining matches by length
    zipped_vec.sort_by(|(member_a, user_a), (member_b, user_b)| {
        let left = member_a.nickname.as_ref().unwrap_or(&user_a.username);
        let right = member_b.nickname.as_ref().unwrap_or(&user_b.username);
        left.len().cmp(&right.len())
    });

    // Take the first ten and return them
    let (members, users): (Vec<Member>, Vec<User>) = zipped_vec.into_iter().take(10).unzip();
    Ok(Json(MemberQueryResponse {
        members: members
            .into_iter()
            .map(|m| m.into(Some(permissions)))
            .collect(),
        users: join_all(
            users
                .into_iter()
                .map(|other_user| other_user.into(db, &user)),
        )
        .await,
    }))
}
