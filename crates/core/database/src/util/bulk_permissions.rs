use std::{collections::HashMap, hash::RandomState};

use revolt_permissions::{
    ChannelPermission, ChannelType, Override, OverrideField, PermissionValue, ALLOW_IN_TIMEOUT,
    DEFAULT_PERMISSION_DIRECT_MESSAGE,
};

use crate::{Channel, Database, Member, Server, User};

#[derive(Clone)]
pub struct BulkDatabasePermissionQuery<'a> {
    #[allow(dead_code)]
    database: &'a Database,

    server: Server,
    channel: Option<Channel>,
    users: Option<Vec<User>>,
    members: Option<Vec<Member>>,

    // In case the users or members are fetched as part of the permissions checking operation
    pub(crate) cached_users: Option<Vec<User>>,
    pub(crate) cached_members: Option<Vec<Member>>,

    cached_member_perms: Option<HashMap<String, PermissionValue>>,
}

impl<'z, 'x> BulkDatabasePermissionQuery<'x> {
    pub async fn members_can_see_channel(&'z mut self) -> HashMap<String, bool>
    where
        'z: 'x,
    {
        let member_perms = if self.cached_member_perms.is_some() {
            // This isn't done as an if let to prevent borrow checker errors with the mut self call when the perms aren't cached.
            let perms = self.cached_member_perms.as_ref().unwrap();
            perms
                .iter()
                .map(|(m, p)| {
                    (
                        m.clone(),
                        p.has_channel_permission(ChannelPermission::ViewChannel),
                    )
                })
                .collect()
        } else {
            calculate_members_permissions(self)
                .await
                .iter()
                .map(|(m, p)| {
                    (
                        m.clone(),
                        p.has_channel_permission(ChannelPermission::ViewChannel),
                    )
                })
                .collect()
        };
        member_perms
    }
}

impl<'z> BulkDatabasePermissionQuery<'z> {
    pub fn new(database: &Database, server: Server) -> BulkDatabasePermissionQuery<'_> {
        BulkDatabasePermissionQuery {
            database,
            server,
            channel: None,
            users: None,
            members: None,
            cached_members: None,
            cached_users: None,
            cached_member_perms: None,
        }
    }

    pub async fn from_server_id<'a>(
        db: &'a Database,
        server: &str,
    ) -> BulkDatabasePermissionQuery<'a> {
        BulkDatabasePermissionQuery {
            database: db,
            server: db.fetch_server(server).await.unwrap(),
            channel: None,
            users: None,
            members: None,
            cached_members: None,
            cached_users: None,
            cached_member_perms: None,
        }
    }

    pub fn channel(self, channel: &'z Channel) -> BulkDatabasePermissionQuery {
        BulkDatabasePermissionQuery {
            channel: Some(channel.clone()),
            ..self
        }
    }

    pub async fn from_channel_id(self, channel_id: String) -> BulkDatabasePermissionQuery<'z> {
        let channel = self
            .database
            .fetch_channel(channel_id.as_str())
            .await
            .expect("Valid channel id");

        drop(channel_id);

        BulkDatabasePermissionQuery {
            channel: Some(channel),
            ..self
        }
    }

    pub fn members(self, members: &'z [Member]) -> BulkDatabasePermissionQuery {
        BulkDatabasePermissionQuery {
            members: Some(members.to_owned()),
            cached_member_perms: None,
            users: None,
            cached_members: None,
            cached_users: None,
            ..self
        }
    }

    pub fn users(self, users: &'z [User]) -> BulkDatabasePermissionQuery {
        BulkDatabasePermissionQuery {
            users: Some(users.to_owned()),
            cached_member_perms: None,
            members: None,
            cached_members: None,
            cached_users: None,
            ..self
        }
    }

    /// Get the default channel permissions
    /// Group channel defaults should be mapped to an allow-only override
    #[allow(dead_code)]
    async fn get_default_channel_permissions(&mut self) -> Override {
        if let Some(channel) = &self.channel {
            match channel {
                Channel::Group { permissions, .. } => Override {
                    allow: permissions.unwrap_or(*DEFAULT_PERMISSION_DIRECT_MESSAGE as i64) as u64,
                    deny: 0,
                },
                Channel::TextChannel {
                    default_permissions,
                    ..
                }
                | Channel::VoiceChannel {
                    default_permissions,
                    ..
                } => default_permissions.unwrap_or_default().into(),
                _ => Default::default(),
            }
        } else {
            Default::default()
        }
    }

    #[allow(dead_code)]
    fn get_channel_type(&mut self) -> ChannelType {
        if let Some(channel) = &self.channel {
            match channel {
                Channel::DirectMessage { .. } => ChannelType::DirectMessage,
                Channel::Group { .. } => ChannelType::Group,
                Channel::SavedMessages { .. } => ChannelType::SavedMessages,
                Channel::TextChannel { .. } | Channel::VoiceChannel { .. } => {
                    ChannelType::ServerChannel
                }
            }
        } else {
            ChannelType::Unknown
        }
    }

    /// Get the ordered role overrides (from lowest to highest) for this member in this channel
    #[allow(dead_code)]
    async fn get_channel_role_overrides(&mut self) -> &HashMap<String, OverrideField> {
        if let Some(channel) = &self.channel {
            match channel {
                Channel::TextChannel {
                    role_permissions, ..
                }
                | Channel::VoiceChannel {
                    role_permissions, ..
                } => role_permissions,
                _ => panic!("Not supported for non-server channels"),
            }
        } else {
            panic!("No channel added to query")
        }
    }
}

/// Calculate members permissions in a server channel.
async fn calculate_members_permissions<'a>(
    query: &'a mut BulkDatabasePermissionQuery<'a>,
) -> HashMap<String, PermissionValue> {
    let mut resp = HashMap::new();

    let (_, channel_role_permissions, channel_default_permissions) = match query
        .channel
        .as_ref()
        .expect("A channel must be assigned to calculate channel permissions")
        .clone()
    {
        Channel::TextChannel {
            id,
            role_permissions,
            default_permissions,
            ..
        }
        | Channel::VoiceChannel {
            id,
            role_permissions,
            default_permissions,
            ..
        } => (id, role_permissions, default_permissions),
        _ => panic!("Calculation of member permissions must be done on a server channel"),
    };

    if query.users.is_none() {
        let ids: Vec<String> = query
            .members
            .as_ref()
            .expect("No users or members added to the query")
            .iter()
            .map(|m| m.id.user.clone())
            .collect();

        query.cached_users = Some(
            query
                .database
                .fetch_users(&ids[..])
                .await
                .expect("Failed to get data from the db"),
        );

        query.users = Some(query.cached_users.as_ref().unwrap().to_vec())
    }

    let users = query.users.as_ref().unwrap();

    if query.members.is_none() {
        let ids: Vec<String> = query
            .users
            .as_ref()
            .expect("No users or members added to the query")
            .iter()
            .map(|m| m.id.clone())
            .collect();

        query.cached_members = Some(
            query
                .database
                .fetch_members(&query.server.id, &ids[..])
                .await
                .expect("Failed to get data from the db"),
        );
        query.members = Some(query.cached_members.as_ref().unwrap().to_vec())
    }

    let members: HashMap<&String, &Member, RandomState> = HashMap::from_iter(
        query
            .members
            .as_ref()
            .unwrap()
            .iter()
            .map(|m| (&m.id.user, m)),
    );

    for user in users {
        let member = members.get(&user.id);

        // User isn't a part of the server
        if member.is_none() {
            resp.insert(user.id.clone(), 0_u64.into());
            continue;
        }

        let member = *member.unwrap();

        if user.privileged {
            resp.insert(
                user.id.clone(),
                PermissionValue::from(ChannelPermission::GrantAllSafe),
            );
            continue;
        }

        if user.id == query.server.owner {
            resp.insert(
                user.id.clone(),
                PermissionValue::from(ChannelPermission::GrantAllSafe),
            );
            continue;
        }

        // Get the user's server permissions
        let mut permission = calculate_server_permissions(&query.server, user, member);

        if let Some(defaults) = channel_default_permissions {
            permission.apply(defaults.into());
        }

        // Get the applicable role overrides
        let mut roles = channel_role_permissions
            .iter()
            .filter(|(id, _)| member.roles.contains(id))
            .filter_map(|(id, permission)| {
                query.server.roles.get(id).map(|role| {
                    let v: Override = (*permission).into();
                    (role.rank, v)
                })
            })
            .collect::<Vec<(i64, Override)>>();

        roles.sort_by(|a, b| b.0.cmp(&a.0));
        let overrides = roles.into_iter().map(|(_, v)| v);

        for role_override in overrides {
            permission.apply(role_override)
        }

        resp.insert(user.id.clone(), permission);
    }

    resp
}

/// Calculates a member's server permissions
fn calculate_server_permissions(server: &Server, user: &User, member: &Member) -> PermissionValue {
    if user.privileged || server.owner == user.id {
        return ChannelPermission::GrantAllSafe.into();
    }

    let mut permissions: PermissionValue = server.default_permissions.into();

    let mut roles = server
        .roles
        .iter()
        .filter(|(id, _)| member.roles.contains(id))
        .map(|(_, role)| {
            let v: Override = role.permissions.into();
            (role.rank, v)
        })
        .collect::<Vec<(i64, Override)>>();

    roles.sort_by(|a, b| b.0.cmp(&a.0));
    let role_overrides: Vec<Override> = roles.into_iter().map(|(_, v)| v).collect();

    for role in role_overrides {
        permissions.apply(role);
    }

    if member.in_timeout() {
        permissions.restrict(*ALLOW_IN_TIMEOUT);
    }

    permissions
}
