use revolt_result::Result;

use crate::ChannelUnread;

mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractChannelUnreads: Sync + Send {
    /// Acknowledge a message, and returns updated channel unread.
    async fn acknowledge_message(
        &self,
        channel_id: &str,
        user_id: &str,
        message_id: &str,
    ) -> Result<Option<ChannelUnread>>;

    /// Acknowledge many channels.
    async fn acknowledge_channels(&self, user_id: &str, channel_ids: &[String]) -> Result<()>;

    /// Add a mention.
    async fn add_mention_to_unread<'a>(
        &self,
        channel_id: &str,
        user_id: &str,
        message_ids: &[String],
    ) -> Result<()>;

    /// Add a mention.
    async fn add_mention_to_many_unreads<'a>(
        &self,
        channel_id: &str,
        user_ids: &[String],
        message_ids: &[String],
    ) -> Result<()>;

    /// Fetch all unreads with mentions for a user.
    async fn fetch_unread_mentions(&self, user_id: &str) -> Result<Vec<ChannelUnread>>;

    /// Fetch all channel unreads for a user.
    async fn fetch_unreads(&self, user_id: &str) -> Result<Vec<ChannelUnread>>;

    /// Fetch unread for a specific user in a channel.
    async fn fetch_unread(&self, user_id: &str, channel_id: &str) -> Result<Option<ChannelUnread>>;
}
