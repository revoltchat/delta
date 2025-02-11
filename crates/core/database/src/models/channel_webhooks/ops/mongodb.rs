use futures::StreamExt;
use revolt_result::Result;

use crate::{FieldsWebhook, PartialWebhook, Webhook};
use crate::{IntoDocumentPath, MongoDb};

use super::AbstractWebhooks;

static COL: &str = "channel_webhooks";

#[async_trait]
impl AbstractWebhooks for MongoDb {
    /// Insert new webhook into the database
    async fn insert_webhook(&self, webhook: &Webhook) -> Result<()> {
        query!(self, insert_one, COL, &webhook).map(|_| ())
    }

    /// Fetch webhook by id
    async fn fetch_webhook(&self, webhook_id: &str) -> Result<Webhook> {
        query!(self, find_one_by_id, COL, webhook_id)?.ok_or_else(|| create_error!(NotFound))
    }

    /// Fetch webhooks for channel
    async fn fetch_webhooks_for_channel(&self, channel_id: &str) -> Result<Vec<Webhook>> {
        Ok(self
            .col::<Webhook>(COL)
            .find(doc! {
                "channel_id": channel_id,
            })
            .await
            .map_err(|_| create_database_error!("find", COL))?
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

    /// Update webhook with new information
    async fn update_webhook(
        &self,
        webhook_id: &str,
        partial: &PartialWebhook,
        remove: &[FieldsWebhook],
    ) -> Result<()> {
        query!(
            self,
            update_one_by_id,
            COL,
            webhook_id,
            partial,
            remove.iter().map(|x| x as &dyn IntoDocumentPath).collect(),
            None
        )
        .map(|_| ())
    }

    /// Delete webhook by id
    async fn delete_webhook(&self, webhook_id: &str) -> Result<()> {
        query!(self, delete_one_by_id, COL, webhook_id).map(|_| ())
    }
}

impl IntoDocumentPath for FieldsWebhook {
    fn as_path(&self) -> Option<&'static str> {
        Some(match self {
            FieldsWebhook::Avatar => "avatar",
        })
    }
}
