use crate::consumers::inbound::internal::*;
use amqprs::{
    channel::{BasicPublishArguments, Channel},
    connection::Connection,
    consumer::AsyncConsumer,
    BasicProperties, Deliver,
};
use async_trait::async_trait;
use revolt_database::{events::rabbit::*, Database};

pub struct AckConsumer {
    #[allow(dead_code)]
    db: Database,
    authifier_db: authifier::Database,
    conn: Option<Connection>,
    channel: Option<Channel>,
}

impl Channeled for AckConsumer {
    fn get_connection(&self) -> Option<&Connection> {
        if self.conn.is_none() {
            None
        } else {
            Some(self.conn.as_ref().unwrap())
        }
    }

    fn get_channel(&self) -> Option<&Channel> {
        if self.channel.is_none() {
            None
        } else {
            Some(self.channel.as_ref().unwrap())
        }
    }

    fn set_connection(&mut self, conn: Connection) {
        self.conn = Some(conn);
    }

    fn set_channel(&mut self, channel: Channel) {
        self.channel = Some(channel)
    }
}

impl AckConsumer {
    pub fn new(db: Database, authifier_db: authifier::Database) -> AckConsumer {
        AckConsumer {
            db,
            authifier_db,
            conn: None,
            channel: None,
        }
    }
}

#[allow(unused_variables)]
#[async_trait]
impl AsyncConsumer for AckConsumer {
    /// This consumer processes all acks the platform receives, and sends relevant badge updates to apple platforms.
    async fn consume(
        &mut self,
        channel: &Channel,
        deliver: Deliver,
        basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
        let content = String::from_utf8(content).unwrap();
        let payload: AckPayload = serde_json::from_str(content.as_str()).unwrap();

        // Step 1: fetch unreads and don't continue if there's no unreads
        #[allow(clippy::disallowed_methods)]
        let unreads = self.db.fetch_unread_mentions(&payload.user_id).await;

        debug!("Processing unreads for {:}", &payload.user_id);

        if let Ok(u) = &unreads {
            if u.is_empty() {
                debug!(
                    "Discarding unread task (no mentions found) for {:}",
                    &payload.user_id
                );
                return;
            }
        } else {
            return;
        }

        if let Ok(sessions) = self.authifier_db.find_sessions(&payload.user_id).await {
            let config = revolt_config::config().await;
            // Step 2: find any apple sessions, since we don't need to calculate this for anything else.
            // If there's no apple sessions, we can return early
            let apple_sessions: Vec<&authifier::models::Session> = sessions
                .iter()
                .filter(|session| {
                    if let Some(sub) = &session.subscription {
                        sub.endpoint == "apn"
                    } else {
                        false
                    }
                })
                .collect();

            if apple_sessions.is_empty() {
                debug!(
                    "Discarding unread task (no apn sessions found) for {:}",
                    &payload.user_id
                );
                return;
            }

            // Step 3: calculate the actual mention count, since we have to send it out
            let mut mention_count = 0;
            for u in &unreads.unwrap() {
                mention_count += u.mentions.as_ref().unwrap().len()
            }

            // Step 4: loop through each apple session and send the badge update
            for session in apple_sessions {
                let service_payload = PayloadToService {
                    notification: PayloadKind::BadgeUpdate(mention_count),
                    user_id: payload.user_id.clone(),
                    session_id: session.id.clone(),
                    token: session.subscription.as_ref().unwrap().auth.clone(),
                    extras: Default::default(),
                };
                let raw_service_payload = serde_json::to_string(&service_payload);

                if let Ok(p) = raw_service_payload {
                    let args = BasicPublishArguments::new(
                        config.pushd.exchange.as_str(),
                        config.pushd.apn.queue.as_str(),
                    )
                    .finish();

                    log::debug!(
                        "Publishing ack to apn session {}",
                        session.subscription.as_ref().unwrap().auth
                    );

                    publish_message(self, p.into(), args).await;
                } else {
                    log::warn!("Failed to serialize ack badge update payload!");
                    revolt_config::capture_error(&raw_service_payload.unwrap_err());
                }
            }
        }
    }
}
