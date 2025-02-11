use ::mongodb::options::FindOneOptions;
use bson::to_bson;
use bson::Document;
use mongodb::options::UpdateOptions;
use revolt_result::Result;

use crate::MongoDb;
use crate::UserSettings;

use super::AbstractUserSettings;

static COL: &str = "user_settings";

#[async_trait]
impl AbstractUserSettings for MongoDb {
    /// Fetch a subset of user settings
    async fn fetch_user_settings(&'_ self, id: &str, filter: &'_ [String]) -> Result<UserSettings> {
        let mut projection = doc! {
            "_id": 0,
        };

        for key in filter {
            projection.insert(key, 1);
        }

        Ok(query!(
            self,
            find_one_with_options,
            COL,
            doc! {
                "_id": id
            },
            FindOneOptions::builder().projection(projection).build()
        )?
        .unwrap_or_default())
    }

    /// Update a subset of user settings
    async fn set_user_settings(&self, id: &str, settings: &UserSettings) -> Result<()> {
        let mut set = doc! {};
        for (key, data) in settings {
            set.insert(
                key,
                vec![to_bson(&data.0).unwrap(), to_bson(&data.1).unwrap()],
            );
        }

        self.col::<Document>(COL)
            .update_one(
                doc! {
                    "_id": id
                },
                doc! {
                    "$set": set
                },
            )
            .with_options(UpdateOptions::builder().upsert(true).build())
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("update_one", "user_settings"))
    }

    /// Delete all user settings
    async fn delete_user_settings(&self, id: &str) -> Result<()> {
        query!(self, delete_one_by_id, COL, id).map(|_| ())
    }
}
