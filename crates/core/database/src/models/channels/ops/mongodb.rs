use super::AbstractChannels;
use crate::{AbstractServers, Channel, FieldsChannel, IntoDocumentPath, MongoDb, PartialChannel};
use bson::{Bson, Document};
use futures::StreamExt;
use revolt_permissions::OverrideField;
use revolt_result::Result;

static COL: &str = "channels";

#[async_trait]
impl AbstractChannels for MongoDb {
    /// Insert a new channel in the database
    async fn insert_channel(&self, channel: &Channel) -> Result<()> {
        query!(self, insert_one, COL, &channel).map(|_| ())
    }

    /// Fetch a channel from the database
    async fn fetch_channel(&self, channel_id: &str) -> Result<Channel> {
        query!(self, find_one_by_id, COL, channel_id)?.ok_or_else(|| create_error!(NotFound))
    }

    /// Fetch all channels from the database
    async fn fetch_channels<'a>(&self, ids: &'a [String]) -> Result<Vec<Channel>> {
        Ok(self
            .col::<Channel>(COL)
            .find(doc! {
                "_id": {
                    "$in": ids
                }
            })
            .await
            .map_err(|_| create_database_error!("fetch", "channels"))?
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

    /// Fetch all direct messages for a user
    async fn find_direct_messages(&self, user_id: &str) -> Result<Vec<Channel>> {
        query!(
            self,
            find,
            COL,
            doc! {
                "$or": [
                    {
                        "$or": [
                            {
                                "channel_type": "DirectMessage"
                            },
                            {
                                "channel_type": "Group"
                            }
                        ],
                        "recipients": user_id
                    },
                    {
                        "channel_type": "SavedMessages",
                        "user": user_id
                    }
                ]
            }
        )
    }

    // Fetch saved messages channel
    async fn find_saved_messages_channel(&self, user_id: &str) -> Result<Channel> {
        query!(
            self,
            find_one,
            COL,
            doc! {
                "channel_type": "SavedMessages",
                "user": user_id
            }
        )?
        .ok_or_else(|| create_error!(InternalError))
    }

    // Fetch direct message channel (DM or Saved Messages)
    async fn find_direct_message_channel(&self, user_a: &str, user_b: &str) -> Result<Channel> {
        let doc = match (user_a, user_b) {
            self_user if self_user.0 == self_user.1 => {
                doc! {
                    "channel_type": "SavedMessages",
                    "user": self_user.0
                }
            }
            users => {
                doc! {
                    "channel_type": "DirectMessage",
                    "recipients": {
                        "$all": [ users.0, users.1 ]
                    }
                }
            }
        };
        query!(self, find_one, COL, doc)?.ok_or_else(|| create_error!(NotFound))
    }

    /// Insert a user to a group
    async fn add_user_to_group(&self, channel: &str, user: &str) -> Result<()> {
        self.col::<Document>(COL)
            .update_one(
                doc! {
                    "_id": channel
                },
                doc! {
                    "$push": {
                        "recipients": user
                    }
                },
            )
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("update_one", "channel"))
    }

    /// Insert channel role permissions
    async fn set_channel_role_permission(
        &self,
        channel: &str,
        role: &str,
        permissions: OverrideField,
    ) -> Result<()> {
        self.col::<Document>(COL)
            .update_one(
                doc! { "_id": channel },
                doc! {
                "$set": {
                    "role_permissions.".to_owned() + role: permissions
                }
                },
            )
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("update_one", "channel"))
    }

    // Update channel
    async fn update_channel(
        &self,
        id: &str,
        channel: &PartialChannel,
        remove: Vec<FieldsChannel>,
    ) -> Result<()> {
        query!(
            self,
            update_one_by_id,
            COL,
            id,
            channel,
            remove.iter().map(|x| x as &dyn IntoDocumentPath).collect(),
            None
        )
        .map(|_| ())
    }

    // Remove a user from a group
    async fn remove_user_from_group(&self, channel: &str, user: &str) -> Result<()> {
        self.col::<Document>(COL)
            .update_one(
                doc! {
                    "_id": channel
                },
                doc! {
                    "$pull": {
                        "recipients": user
                    }
                },
            )
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("update_one", "channels"))
    }

    // Delete a channel
    async fn delete_channel(&self, channel: &Channel) -> Result<()> {
        let id = channel.id().to_string();
        let server_id = match channel {
            Channel::TextChannel { server, .. } | Channel::VoiceChannel { server, .. } => {
                Some(server)
            }
            _ => None,
        };

        // Delete invites and unreads.
        self.delete_associated_channel_objects(Bson::String(id.to_string()))
            .await?;

        // Delete messages.
        self.delete_bulk_messages(doc! {
            "channel": &id
        })
        .await?;

        // Remove from server object.
        if let Some(server) = server_id {
            let server = self.fetch_server(server).await?;
            let mut update = doc! {
                "$pull": {
                    "channels": &id
                }
            };

            if let Some(sys) = &server.system_messages {
                let mut unset = doc! {};

                if let Some(cid) = &sys.user_joined {
                    if &id == cid {
                        unset.insert("system_messages.user_joined", 1_i32);
                    }
                }

                if let Some(cid) = &sys.user_left {
                    if &id == cid {
                        unset.insert("system_messages.user_left", 1_i32);
                    }
                }

                if let Some(cid) = &sys.user_kicked {
                    if &id == cid {
                        unset.insert("system_messages.user_kicked", 1_i32);
                    }
                }

                if let Some(cid) = &sys.user_banned {
                    if &id == cid {
                        unset.insert("system_messages.user_banned", 1_i32);
                    }
                }

                if !unset.is_empty() {
                    update.insert("$unset", unset);
                }
            }

            self.col::<Document>("servers")
                .update_one(
                    doc! {
                        "_id": server.id
                    },
                    update,
                )
                .await
                .map_err(|_| create_database_error!("update_one", "servers"))?;
        }

        // Delete associated attachments
        self.delete_many_attachments(doc! {
            "used_for.id": &id
        })
        .await?;

        // Delete the channel itself
        query!(self, delete_one_by_id, COL, channel.id()).map(|_| ())
    }
}

impl MongoDb {
    pub async fn delete_associated_channel_objects(&self, id: Bson) -> Result<()> {
        // Delete all invites to these channels.
        self.col::<Document>("channel_invites")
            .delete_many(doc! {
                "channel": &id
            })
            .await
            .map_err(|_| create_database_error!("delete_many", "channel_invites"))?;

        // Delete unread message objects on channels.
        self.col::<Document>("channel_unreads")
            .delete_many(doc! {
                "_id.channel": &id
            })
            .await
            .map_err(|_| create_database_error!("delete_many", "channel_unreads"))
            .map(|_| ())?;

        // update many attachments with parent id

        // Delete all webhooks on this channel.
        self.col::<Document>("webhooks")
            .delete_many(doc! {
                "channel": &id
            })
            .await
            .map_err(|_| create_database_error!("delete_many", "webhooks"))
            .map(|_| ())
    }
}
