use bson::to_document;
use bson::Document;
use revolt_config::report_internal_error;
use revolt_result::Result;

use crate::File;
use crate::FileUsedFor;
use crate::MongoDb;

use super::AbstractAttachments;

static COL: &str = "attachments";

#[async_trait]
impl AbstractAttachments for MongoDb {
    /// Insert attachment into database.
    async fn insert_attachment(&self, attachment: &File) -> Result<()> {
        query!(self, insert_one, COL, &attachment).map(|_| ())
    }

    /// Fetch an attachment by its id.
    async fn fetch_attachment(&self, tag: &str, file_id: &str) -> Result<File> {
        query!(
            self,
            find_one,
            COL,
            doc! {
                "_id": file_id,
                "tag": tag
            }
        )?
        .ok_or_else(|| create_error!(NotFound))
    }

    /// Fetch all deleted attachments.
    async fn fetch_deleted_attachments(&self) -> Result<Vec<File>> {
        query!(
            self,
            find,
            COL,
            doc! {
                "deleted": true,
                "reported": {
                    "$ne": true
                }
            }
        )
    }

    /// Fetch all dangling attachments.
    async fn fetch_dangling_files(&self) -> Result<Vec<File>> {
        query!(
            self,
            find,
            COL,
            doc! {
                "used_for.type": {
                    "$exists": 0
                },
                "deleted": {
                    "$ne": true
                }
            }
        )
    }

    /// Count references to a given hash.
    async fn count_file_hash_references(&self, hash: &str) -> Result<usize> {
        query!(
            self,
            count_documents,
            COL,
            doc! {
                "hash": hash
            }
        )
        .map(|count| count as usize)
    }

    /// Find an attachment by its details and mark it as used by a given parent.
    async fn find_and_use_attachment(
        &self,
        id: &str,
        tag: &str,
        used_for: FileUsedFor,
        uploader_id: String,
    ) -> Result<File> {
        let file = query!(
            self,
            find_one,
            COL,
            doc! {
                "_id": id,
                "tag": tag,
                "used_for": {
                    "$exists": false
                }
            }
        )?
        .ok_or_else(|| create_error!(NotFound))?;

        self.col::<Document>(COL)
            .update_one(
                doc! {
                    "_id": id
                },
                doc! {
                    "$set": {
                        "used_for": report_internal_error!(to_document(&used_for))?,
                        "uploader_id": uploader_id
                    }
                },
            )
            .await
            .map_err(|_| create_database_error!("update_one", COL))?;

        Ok(file)
    }

    /// Mark an attachment as having been reported.
    async fn mark_attachment_as_reported(&self, id: &str) -> Result<()> {
        self.col::<Document>(COL)
            .update_one(
                doc! {
                    "_id": id
                },
                doc! {
                    "$set": {
                        "reported": true
                    }
                },
            )
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("update_one", COL))
    }

    /// Mark an attachment as having been deleted.
    async fn mark_attachment_as_deleted(&self, id: &str) -> Result<()> {
        self.col::<Document>(COL)
            .update_one(
                doc! {
                    "_id": id
                },
                doc! {
                    "$set": {
                        "deleted": true
                    }
                },
            )
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("update_one", COL))
    }

    /// Mark multiple attachments as having been deleted.
    async fn mark_attachments_as_deleted(&self, ids: &[String]) -> Result<()> {
        self.col::<Document>(COL)
            .update_many(
                doc! {
                    "_id": {
                        "$in": ids
                    }
                },
                doc! {
                    "$set": {
                        "deleted": true
                    }
                },
            )
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("update_many", COL))
    }

    /// Delete the attachment entry.
    async fn delete_attachment(&self, id: &str) -> Result<()> {
        query!(self, delete_one_by_id, COL, id).map(|_| ())
    }
}

impl MongoDb {
    pub async fn delete_many_attachments(&self, projection: Document) -> Result<()> {
        self.col::<Document>(COL)
            .update_many(
                projection,
                doc! {
                    "$set": {
                        "deleted": true
                    }
                },
            )
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("update_many", COL))
    }
}
