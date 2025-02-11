use std::collections::HashMap;
use std::ops::Deref;

use futures::StreamExt;
use mongodb::bson::{doc, to_document, Document};
use mongodb::error::Result;
use mongodb::options::{FindOneOptions, FindOptions};
use mongodb::results::{DeleteResult, InsertOneResult, UpdateResult};
use serde::de::DeserializeOwned;
use serde::Serialize;

database_derived!(
    #[cfg(feature = "mongodb")]
    /// MongoDB implementation
    pub struct MongoDb(pub ::mongodb::Client, pub String);
);

impl Deref for MongoDb {
    type Target = mongodb::Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[allow(dead_code)]
impl MongoDb {
    /// Get the Revolt database
    pub fn db(&self) -> mongodb::Database {
        self.database(&self.1)
    }

    /// Get a collection by its name
    pub fn col<T: Send + Sync>(&self, collection: &str) -> mongodb::Collection<T> {
        self.db().collection(collection)
    }

    /// Insert one document into a collection
    pub async fn insert_one<T: Serialize + Send + Sync>(
        &self,
        collection: &'static str,
        document: T,
    ) -> Result<InsertOneResult> {
        self.col::<T>(collection).insert_one(document).await
    }

    /// Count documents by projection
    pub async fn count_documents(
        &self,
        collection: &'static str,
        projection: Document,
    ) -> Result<u64> {
        self.col::<Document>(collection)
            .count_documents(projection)
            .await
    }

    /// Find multiple documents in a collection with options
    pub async fn find_with_options<O, T: DeserializeOwned + Unpin + Send + Sync>(
        &self,
        collection: &'static str,
        projection: Document,
        options: O,
    ) -> Result<Vec<T>>
    where
        O: Into<Option<FindOptions>>,
    {
        Ok(self
            .col::<T>(collection)
            .find(projection)
            .with_options(options)
            .await?
            .filter_map(|s| async {
                if cfg!(debug_assertions) {
                    // Hard fail on invalid documents
                    Some(s.unwrap())
                } else {
                    s.ok()
                }
            })
            .collect::<Vec<T>>()
            .await)
    }

    /// Find multiple documents in a collection
    pub async fn find<T: DeserializeOwned + Unpin + Send + Sync>(
        &self,
        collection: &'static str,
        projection: Document,
    ) -> Result<Vec<T>> {
        self.find_with_options(collection, projection, None).await
    }

    /// Find one document with options
    pub async fn find_one_with_options<O, T: DeserializeOwned + Unpin + Send + Sync>(
        &self,
        collection: &'static str,
        projection: Document,
        options: O,
    ) -> Result<Option<T>>
    where
        O: Into<Option<FindOneOptions>>,
    {
        self.col::<T>(collection)
            .find_one(projection)
            .with_options(options)
            .await
    }

    /// Find one document
    pub async fn find_one<T: DeserializeOwned + Unpin + Send + Sync>(
        &self,
        collection: &'static str,
        projection: Document,
    ) -> Result<Option<T>> {
        self.find_one_with_options(collection, projection, None)
            .await
    }

    /// Find one document by its ID
    pub async fn find_one_by_id<T: DeserializeOwned + Unpin + Send + Sync>(
        &self,
        collection: &'static str,
        id: &str,
    ) -> Result<Option<T>> {
        self.find_one(
            collection,
            doc! {
                "_id": id
            },
        )
        .await
    }

    /// Update one document given a projection, partial document, and list of paths to unset
    pub async fn update_one<P, T: Serialize>(
        &self,
        collection: &'static str,
        projection: Document,
        partial: T,
        remove: Vec<&dyn IntoDocumentPath>,
        prefix: P,
    ) -> Result<UpdateResult>
    where
        P: Into<Option<String>>,
    {
        let prefix = prefix.into();

        let mut unset = doc! {};
        for field in remove {
            if let Some(path) = field.as_path() {
                if let Some(prefix) = &prefix {
                    unset.insert(prefix.to_owned() + path, 1_i32);
                } else {
                    unset.insert(path, 1_i32);
                }
            }
        }

        let query = doc! {
            "$unset": unset,
            "$set": if let Some(prefix) = &prefix {
                to_document(&prefix_keys(&partial, prefix))
            } else {
                to_document(&partial)
            }?
        };

        self.col::<Document>(collection)
            .update_one(projection, query)
            .await
    }

    /// Update one document given an ID, partial document, and list of paths to unset
    pub async fn update_one_by_id<P, T: Serialize>(
        &self,
        collection: &'static str,
        id: &str,
        partial: T,
        remove: Vec<&dyn IntoDocumentPath>,
        prefix: P,
    ) -> Result<UpdateResult>
    where
        P: Into<Option<String>>,
    {
        self.update_one(
            collection,
            doc! {
                "_id": id
            },
            partial,
            remove,
            prefix,
        )
        .await
    }

    /// Delete one document by the given projection
    pub async fn delete_one(
        &self,
        collection: &'static str,
        projection: Document,
    ) -> Result<DeleteResult> {
        self.col::<Document>(collection)
            .delete_one(projection)
            .await
    }

    /// Delete one document by the given ID
    pub async fn delete_one_by_id(
        &self,
        collection: &'static str,
        id: &str,
    ) -> Result<DeleteResult> {
        self.delete_one(
            collection,
            doc! {
                "_id": id
            },
        )
        .await
    }
}

/// Just a string ID struct
#[derive(Deserialize)]
pub struct DocumentId {
    #[serde(rename = "_id")]
    pub id: String,
}

pub trait IntoDocumentPath: Send + Sync {
    /// Create JSON key path
    fn as_path(&self) -> Option<&'static str>;
}

/// Prefix keys on an arbitrary object
pub fn prefix_keys<T: Serialize>(t: &T, prefix: &str) -> HashMap<String, serde_json::Value> {
    let v: String = serde_json::to_string(t).unwrap();
    let v: HashMap<String, serde_json::Value> = serde_json::from_str(&v).unwrap();
    v.into_iter()
        .filter(|(_k, v)| !v.is_null())
        .map(|(k, v)| (format!("{}{}", prefix.to_owned(), k), v))
        .collect()
}
