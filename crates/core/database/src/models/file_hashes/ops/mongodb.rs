use revolt_result::Result;

use crate::FileHash;
use crate::MongoDb;

use super::AbstractAttachmentHashes;

static COL: &str = "attachment_hashes";

#[async_trait]
impl AbstractAttachmentHashes for MongoDb {
    /// Insert a new attachment hash into the database.
    async fn insert_attachment_hash(&self, hash: &FileHash) -> Result<()> {
        query!(self, insert_one, COL, &hash).map(|_| ())
    }

    /// Fetch an attachment hash entry by sha256 hash.
    async fn fetch_attachment_hash(&self, hash: &str) -> Result<FileHash> {
        query!(
            self,
            find_one,
            COL,
            doc! {
                "$or": [
                    {"_id": hash},
                    {"processed_hash": hash}
                ]
            }
        )?
        .ok_or_else(|| create_error!(NotFound))
    }

    /// Update an attachment hash nonce value.
    async fn set_attachment_hash_nonce(&self, hash: &str, nonce: &str) -> Result<()> {
        self.col::<FileHash>(COL)
            .update_one(
                doc! {
                    "_id": hash
                },
                doc! {
                    "$set": {
                        "iv": nonce
                    }
                },
            )
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("update_one", COL))
    }

    /// Delete attachment hash by id.
    async fn delete_attachment_hash(&self, id: &str) -> Result<()> {
        query!(self, delete_one_by_id, COL, id).map(|_| ())
    }
}
