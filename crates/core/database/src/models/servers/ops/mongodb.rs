use bson::{to_document, Bson, Document};
use futures::StreamExt;
use revolt_result::Result;

use crate::{FieldsRole, FieldsServer, PartialRole, PartialServer, Role, Server};
use crate::{IntoDocumentPath, MongoDb};

use super::AbstractServers;

static COL: &str = "servers";

#[async_trait]
impl AbstractServers for MongoDb {
    /// Insert a new server into database
    async fn insert_server(&self, server: &Server) -> Result<()> {
        query!(self, insert_one, COL, &server).map(|_| ())
    }

    /// Fetch a server by its id
    async fn fetch_server(&self, id: &str) -> Result<Server> {
        query!(self, find_one_by_id, COL, id)?.ok_or_else(|| create_error!(NotFound))
    }

    /// Fetch a servers by their ids
    async fn fetch_servers<'a>(&self, ids: &'a [String]) -> Result<Vec<Server>> {
        Ok(self
            .col::<Server>(COL)
            .find(doc! {
                "_id": {
                    "$in": ids
                }
            })
            .await
            .map_err(|_| create_database_error!("find", "servers"))?
            .filter_map(|s| async {
                if cfg!(debug_assertions) {
                    Some(s.unwrap())
                } else {
                    s.ok()
                }
            })
            .collect()
            .await)
    }

    /// Update a server with new information
    async fn update_server(
        &self,
        id: &str,
        partial: &PartialServer,
        remove: Vec<FieldsServer>,
    ) -> Result<()> {
        query!(
            self,
            update_one_by_id,
            COL,
            id,
            partial,
            remove.iter().map(|x| x as &dyn IntoDocumentPath).collect(),
            None
        )
        .map(|_| ())
    }

    /// Delete a server by its id
    async fn delete_server(&self, id: &str) -> Result<()> {
        self.delete_associated_server_objects(id).await?;
        query!(self, delete_one_by_id, COL, id).map(|_| ())
    }

    /// Insert a new role into server object
    async fn insert_role(&self, server_id: &str, role_id: &str, role: &Role) -> Result<()> {
        self.col::<Document>(COL)
            .update_one(
                doc! {
                    "_id": server_id
                },
                doc! {
                    "$set": {
                        "roles.".to_owned() + role_id: to_document(role)
                            .map_err(|_| create_database_error!("to_document", "role"))?
                    }
                },
            )
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("update_one", "server"))
    }

    /// Update an existing role on a server
    async fn update_role(
        &self,
        server_id: &str,
        role_id: &str,
        partial: &PartialRole,
        remove: Vec<FieldsRole>,
    ) -> Result<()> {
        query!(
            self,
            update_one_by_id,
            COL,
            server_id,
            partial,
            remove.iter().map(|x| x as &dyn IntoDocumentPath).collect(),
            "roles.".to_owned() + role_id + "."
        )
        .map(|_| ())
    }

    /// Delete a role from a server
    ///
    /// Also updates channels and members.
    async fn delete_role(&self, server_id: &str, role_id: &str) -> Result<()> {
        self.col::<Document>("server_members")
            .update_many(
                doc! {
                    "_id.server": server_id
                },
                doc! {
                    "$pull": {
                        "roles": &role_id
                    }
                },
            )
            .await
            .map_err(|_| create_database_error!("update_many", "server_members"))?;

        self.col::<Document>("channels")
            .update_one(
                doc! {
                    "server": server_id
                },
                doc! {
                    "$unset": {
                        "role_permissions.".to_owned() + role_id: 1_i32
                    }
                },
            )
            .await
            .map_err(|_| create_database_error!("update_one", "channels"))?;

        self.col::<Document>("servers")
            .update_one(
                doc! {
                    "_id": server_id
                },
                doc! {
                    "$unset": {
                        "roles.".to_owned() + role_id: 1_i32
                    }
                },
            )
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("update_one", "servers"))
    }
}

impl IntoDocumentPath for FieldsServer {
    fn as_path(&self) -> Option<&'static str> {
        Some(match self {
            FieldsServer::Banner => "banner",
            FieldsServer::Categories => "categories",
            FieldsServer::Description => "description",
            FieldsServer::Icon => "icon",
            FieldsServer::SystemMessages => "system_messages",
        })
    }
}

impl IntoDocumentPath for FieldsRole {
    fn as_path(&self) -> Option<&'static str> {
        Some(match self {
            FieldsRole::Colour => "colour",
        })
    }
}

impl MongoDb {
    pub async fn delete_associated_server_objects(&self, server_id: &str) -> Result<()> {
        // Find all channels
        let channels: Vec<String> = self
            .col::<Document>("channels")
            .find(doc! {
                "server": server_id
            })
            .await
            .map_err(|_| create_database_error!("find", "channels"))?
            .filter_map(|s| async {
                s.map(|d| d.get_str("_id").map(|s| s.to_string()).ok())
                    .ok()
                    .flatten()
            })
            .collect()
            .await;

        // Check if there are any attachments we need to delete.
        self.delete_bulk_messages(doc! {
            "channel": {
                "$in": &channels
            }
        })
        .await?;

        // Delete all emoji.
        self.col::<Document>("emojis")
            .update_many(
                doc! {
                    "parent.id": &server_id
                },
                doc! {
                    "$set": {
                        "parent": {
                            "type": "Detached"
                        }
                    }
                },
            )
            .await
            .map_err(|_| create_database_error!("update_many", "emojis"))?;

        // Delete all channels.
        self.col::<Document>("channels")
            .delete_many(doc! {
                "server": &server_id
            })
            .await
            .map_err(|_| create_database_error!("delete_many", "channels"))?;

        // Delete any associated objects, e.g. unreads and invites.
        self.delete_associated_channel_objects(Bson::Document(doc! { "$in": &channels }))
            .await?;

        // Delete members and bans.
        for with in &["server_members", "server_bans"] {
            self.col::<Document>(with)
                .delete_many(doc! {
                    "_id.server": &server_id
                })
                .await
                .map_err(|_| create_database_error!("delete_many", with))?;
        }

        // Update many attachments with parent id.
        self.delete_many_attachments(doc! {
            "used_for.id": &server_id
        })
        .await?;

        Ok(())
    }
}
