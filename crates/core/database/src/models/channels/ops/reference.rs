use revolt_permissions::OverrideField;
use revolt_result::Result;

use crate::{Channel, FieldsChannel, PartialChannel, ReferenceDb};
//use crate::{FieldsRole, FieldsServer, PartialRole, PartialServer, Role, Server};

use super::AbstractChannels;

#[async_trait]
impl AbstractChannels for ReferenceDb {
    async fn fetch_channel(&self, id: &str) -> Result<Channel> {
        todo!()
    }

    async fn fetch_channels<'a>(&self, ids: &'a [String]) -> Result<Vec<Channel>> {
        todo!()
    }
    async fn insert_channel(&self, channel: &Channel) -> Result<()> {
        todo!()
    }
    async fn update_channel(
        &self,
        id: &str,
        channel: &PartialChannel,
        remove: Vec<FieldsChannel>,
    ) -> Result<()> {
        todo!()
    }
    async fn delete_channel(&self, channel: &Channel) -> Result<()> {
        todo!()
    }
    async fn find_direct_messages(&self, user_id: &str) -> Result<Vec<Channel>> {
        todo!()
    }
    async fn find_saved_messages_channel(&self, user_id: &str) -> Result<Channel> {
        todo!()
    }
    async fn find_direct_message_channel(&self, user_a: &str, user_b: &str) -> Result<Channel> {
        todo!()
    }
    async fn add_user_to_group(&self, channel: &str, user: &str) -> Result<()> {
        todo!()
    }
    async fn remove_user_from_group(&self, channel: &str, user: &str) -> Result<()> {
        todo!()
    }
    async fn set_channel_role_permission(
        &self,
        channel: &str,
        role: &str,
        permissions: OverrideField,
    ) -> Result<()> {
        todo!()
    }
}
