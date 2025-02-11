use bson::{to_bson, Document};
use futures::try_join;
use mongodb::options::FindOptions;
use revolt_models::v0::MessageSort;
use revolt_result::Result;

use crate::{
    AppendMessage, DocumentId, FieldsMessage, IntoDocumentPath, Message, MessageQuery,
    MessageTimePeriod, MongoDb, PartialMessage,
};

use super::AbstractMessages;

static COL: &str = "messages";

#[async_trait]
impl AbstractMessages for MongoDb {
    /// Insert a new message into the database
    async fn insert_message(&self, message: &Message) -> Result<()> {
        query!(self, insert_one, COL, &message).map(|_| ())
    }

    /// Fetch a message by its id
    async fn fetch_message(&self, id: &str) -> Result<Message> {
        query!(self, find_one_by_id, COL, id)?.ok_or_else(|| create_error!(NotFound))
    }

    /// Fetch multiple messages by given query
    async fn fetch_messages(&self, query: MessageQuery) -> Result<Vec<Message>> {
        let mut filter = doc! {};

        // 1. Apply message filters
        if let Some(channel) = query.filter.channel {
            filter.insert("channel", channel);
        }

        if let Some(author) = query.filter.author {
            filter.insert("author", author);
        }

        let is_search_query = if let Some(query) = query.filter.query {
            filter.insert(
                "$text",
                doc! {
                    "$search": query
                },
            );

            true
        } else {
            false
        };

        if let Some(pinned) = query.filter.pinned {
            filter.insert("pinned", pinned);
        };

        // 2. Find query limit
        let limit = query.limit.unwrap_or(50);

        // 3. Apply message time period
        match query.time_period {
            MessageTimePeriod::Relative { nearby } => {
                // 3.1. Prepare filters
                let mut older_message_filter = filter.clone();
                let mut newer_message_filter = filter;

                older_message_filter.insert(
                    "_id",
                    doc! {
                        "$lt": &nearby
                    },
                );

                newer_message_filter.insert(
                    "_id",
                    doc! {
                        "$gte": &nearby
                    },
                );

                // 3.2. Execute in both directions
                let (a, b) = try_join!(
                    self.find_with_options::<_, Message>(
                        COL,
                        newer_message_filter,
                        FindOptions::builder()
                            .limit(limit / 2 + 1)
                            .sort(doc! {
                                "_id": 1_i32
                            })
                            .build(),
                    ),
                    self.find_with_options::<_, Message>(
                        COL,
                        older_message_filter,
                        FindOptions::builder()
                            .limit(limit / 2 + 1)
                            .sort(doc! {
                                "_id": -1_i32
                            })
                            .build(),
                    )
                )
                .map_err(|_| create_database_error!("find", COL))?;

                Ok([a, b].concat())
            }
            MessageTimePeriod::Absolute {
                before,
                after,
                sort,
            } => {
                // 3.1. Apply message ID filter
                if let Some(doc) = match (before, after) {
                    (Some(before), Some(after)) => Some(doc! {
                        "$lt": before,
                        "$gt": after
                    }),
                    (Some(before), _) => Some(doc! {
                        "$lt": before
                    }),
                    (_, Some(after)) => Some(doc! {
                        "$gt": after
                    }),
                    _ => None,
                } {
                    filter.insert("_id", doc);
                }

                // 3.2. Execute with given message sort
                self.find_with_options(
                    COL,
                    filter,
                    FindOptions::builder()
                        .limit(limit)
                        .sort(match sort.unwrap_or(MessageSort::Latest) {
                            // Sort by relevance, fallback to latest
                            MessageSort::Relevance => {
                                if is_search_query {
                                    doc! {
                                        "score": {
                                            "$meta": "textScore"
                                        }
                                    }
                                } else {
                                    doc! {
                                        "_id": -1_i32
                                    }
                                }
                            }
                            // Sort by latest first
                            MessageSort::Latest => doc! {
                                "_id": -1_i32
                            },
                            // Sort by oldest first
                            MessageSort::Oldest => doc! {
                                "_id": 1_i32
                            },
                        })
                        .build(),
                )
                .await
                .map_err(|_| create_database_error!("find", COL))
            }
        }
    }

    /// Fetch multiple messages by given IDs
    async fn fetch_messages_by_id(&self, ids: &[String]) -> Result<Vec<Message>> {
        self.find_with_options(
            COL,
            doc! {
                "_id": {
                    "$in": ids
                }
            },
            None,
        )
        .await
        .map_err(|_| create_database_error!("find", COL))
    }

    /// Update a given message with new information
    async fn update_message(
        &self,
        id: &str,
        message: &PartialMessage,
        remove: Vec<FieldsMessage>,
    ) -> Result<()> {
        query!(
            self,
            update_one_by_id,
            COL,
            id,
            message,
            remove.iter().map(|x| x as &dyn IntoDocumentPath).collect(),
            None
        )
        .map(|_| ())
    }

    /// Append information to a given message
    async fn append_message(&self, id: &str, append: &AppendMessage) -> Result<()> {
        let mut query = doc! {};

        if let Some(embeds) = &append.embeds {
            if !embeds.is_empty() {
                query.insert(
                    "$push",
                    doc! {
                        "embeds": {
                            "$each": to_bson(embeds)
                                .map_err(|_| create_database_error!("to_bson", "embeds"))?
                        }
                    },
                );
            }
        }

        if query.is_empty() {
            return Ok(());
        }

        self.col::<Document>(COL)
            .update_one(
                doc! {
                    "_id": id
                },
                query,
            )
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("update_one", COL))
    }

    /// Add a new reaction to a message
    async fn add_reaction(&self, id: &str, emoji: &str, user: &str) -> Result<()> {
        self.col::<Document>(COL)
            .update_one(
                doc! {
                    "_id": id
                },
                doc! {
                    "$addToSet": {
                        format!("reactions.{emoji}"): user
                    }
                },
            )
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("update_one", COL))
    }

    /// Remove a reaction from a message
    async fn remove_reaction(&self, id: &str, emoji: &str, user: &str) -> Result<()> {
        self.col::<Document>(COL)
            .update_one(
                doc! {
                    "_id": id
                },
                doc! {
                    "$pull": {
                        format!("reactions.{emoji}"): user
                    }
                },
            )
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("update_one", COL))
    }

    /// Remove reaction from a message
    async fn clear_reaction(&self, id: &str, emoji: &str) -> Result<()> {
        self.col::<Document>(COL)
            .update_one(
                doc! {
                    "_id": id
                },
                doc! {
                    "$unset": {
                        format!("reactions.{emoji}"): 1
                    }
                },
            )
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("update_one", COL))
    }

    /// Delete a message from the database by its id
    async fn delete_message(&self, id: &str) -> Result<()> {
        query!(self, delete_one_by_id, COL, id).map(|_| ())
    }

    /// Delete messages from a channel by their ids and corresponding channel id
    async fn delete_messages(&self, channel: &str, ids: &[String]) -> Result<()> {
        self.col::<Document>(COL)
            .delete_many(doc! {
                "channel": channel,
                "_id": {
                    "$in": ids
                }
            })
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("delete_many", COL))
    }
}

impl IntoDocumentPath for FieldsMessage {
    fn as_path(&self) -> Option<&'static str> {
        Some(match self {
            FieldsMessage::Pinned => "pinned",
        })
    }
}

impl MongoDb {
    pub async fn delete_bulk_messages(&self, projection: Document) -> Result<()> {
        let mut for_attachments = projection.clone();
        for_attachments.insert(
            "attachments",
            doc! {
                "$exists": 1_i32
            },
        );

        // Check if there are any attachments we need to delete.
        let message_ids_with_attachments = self
            .find_with_options::<_, DocumentId>(
                COL,
                for_attachments,
                FindOptions::builder()
                    .projection(doc! { "_id": 1_i32 })
                    .build(),
            )
            .await
            .map_err(|_| create_database_error!("find_many", "attachments"))?
            .into_iter()
            .map(|x| x.id)
            .collect::<Vec<String>>();

        // If we found any, mark them as deleted.
        if !message_ids_with_attachments.is_empty() {
            self.col::<Document>("attachments")
                .update_many(
                    doc! {
                        "message_id": {
                            "$in": message_ids_with_attachments
                        }
                    },
                    doc! {
                        "$set": {
                            "deleted": true
                        }
                    },
                )
                .await
                .map_err(|_| create_database_error!("update_many", "attachments"))?;
        }

        // And then delete said messages.
        self.col::<Document>(COL)
            .delete_many(projection)
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("delete_many", COL))
    }
}
