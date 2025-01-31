use std::{
    collections::{HashMap, HashSet},
    hash::RandomState,
};

use crate::consumers::inbound::internal::*;
use amqprs::{
    channel::{BasicPublishArguments, Channel},
    connection::Connection,
    consumer::AsyncConsumer,
    BasicProperties, Deliver,
};
use async_trait::async_trait;
use revolt_database::{
    events::rabbit::*, util::bulk_permissions::BulkDatabasePermissionQuery, Database, Member,
    MessageFlagsValue,
};
use revolt_models::v0::{MessageFlags, PushNotification};

pub struct MassMessageConsumer {
    #[allow(dead_code)]
    db: Database,
    authifier_db: authifier::Database,
    conn: Option<Connection>,
    channel: Option<Channel>,
}

impl Channeled for MassMessageConsumer {
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

impl MassMessageConsumer {
    pub fn new(db: Database, authifier_db: authifier::Database) -> MassMessageConsumer {
        MassMessageConsumer {
            db,
            authifier_db,
            conn: None,
            channel: None,
        }
    }

    async fn fire_notification_for_users(&mut self, push: &PushNotification, users: &[String]) {
        if let Ok(sessions) = self
            .authifier_db
            .find_sessions_with_subscription(users)
            .await
        {
            let config = revolt_config::config().await;
            for session in sessions {
                if let Some(sub) = session.subscription {
                    let mut sendable = PayloadToService {
                        notification: PayloadKind::MessageNotification(push.clone()),
                        token: sub.auth,
                        user_id: session.user_id,
                        session_id: session.id,
                        extras: HashMap::new(),
                    };

                    let args: BasicPublishArguments;

                    if sub.endpoint == "apn" {
                        args = BasicPublishArguments::new(
                            config.pushd.exchange.as_str(),
                            config.pushd.apn.queue.as_str(),
                        )
                        .finish();
                    } else if sub.endpoint == "fcm" {
                        args = BasicPublishArguments::new(
                            config.pushd.exchange.as_str(),
                            config.pushd.fcm.queue.as_str(),
                        )
                        .finish();
                    } else {
                        // web push (vapid)
                        args = BasicPublishArguments::new(
                            config.pushd.exchange.as_str(),
                            config.pushd.vapid.queue.as_str(),
                        )
                        .finish();
                        sendable.extras.insert("p265dh".to_string(), sub.p256dh);
                        sendable
                            .extras
                            .insert("endpoint".to_string(), sub.endpoint.clone());
                    }

                    let payload = serde_json::to_string(&sendable).unwrap();

                    publish_message(self, payload.into(), args).await;
                }
            }
        }
    }
}

#[allow(unused_variables)]
#[async_trait]
impl AsyncConsumer for MassMessageConsumer {
    /// This consumer handles adding mentions for all the users affected by a mass mention ping, and then sends out push notifications
    async fn consume(
        &mut self,
        channel: &Channel,
        deliver: Deliver,
        basic_properties: BasicProperties,
        content: Vec<u8>,
    ) {
        let config = revolt_config::config().await;
        let content = String::from_utf8(content).unwrap();
        let payload: MassMessageSentPayload = serde_json::from_str(content.as_str()).unwrap();

        debug!("Received mass message event");

        // We should only ever receive clumped messages from a single channel, so it's safe to reuse this many times.
        let mut query: Option<BulkDatabasePermissionQuery<'_>> = None;
        let query_db = self.db.clone();

        for push in payload.notifications {
            if query.is_none() {
                query = Some(
                    BulkDatabasePermissionQuery::from_server_id(&query_db, &payload.server_id)
                        .await
                        .from_channel_id(push.channel.id().to_string()) // wrong channel model, so fetch the right one
                        .await,
                );
            }

            let existing_mentions: HashSet<String, RandomState> =
                if let Some(ref mentions) = push.message.mentions {
                    HashSet::from_iter(mentions.iter().cloned())
                } else {
                    HashSet::new()
                };

            // KNOWN QUIRK: if you mention @online and role(s), the offline members with the role(s) wont get pinged
            if let Some(ref query) = query {
                let flags = MessageFlagsValue(push.message.flags);
                if flags.has(MessageFlags::MentionsEveryone) {
                    let mut db_query = self
                        .db
                        .fetch_all_members_chunked(&payload.server_id)
                        .await
                        .expect("Failed to fetch members from database");

                    let mut exhausted = false;
                    let ack_chnl = vec![push.channel.id().to_string()];
                    loop {
                        let mut chunk: Vec<Member> = vec![];
                        for _ in 0..config.pushd.mass_mention_chunk_size {
                            if let Some(member) = db_query.next().await {
                                chunk.push(member);
                            } else {
                                exhausted = true;
                                break;
                            }
                        }

                        let userids: Vec<String> =
                            chunk.iter().map(|member| member.id.user.clone()).collect();

                        debug!("Userids in chunk: {:?}", userids);

                        if let Err(err) = self
                            .db
                            .add_mention_to_many_unreads(push.channel.id(), &userids, &ack_chnl)
                            .await
                        {
                            revolt_config::capture_error(&err);
                        }

                        // ignore anyone in this list
                        let online_users = revolt_presence::filter_online(&userids).await;
                        let target_users: Vec<String> = userids
                            .iter()
                            .filter(|id| {
                                !online_users.contains(*id) && !existing_mentions.contains(*id)
                            })
                            .cloned()
                            .collect();

                        debug!(
                            "Userids after filter: {:?} (online: {:?}",
                            target_users, online_users
                        );

                        self.fire_notification_for_users(&push, &target_users).await;

                        if exhausted {
                            break;
                        }
                    }
                } else if let Some(roles) = &push.message.role_mentions {
                    // role mentions
                    let _role_members = self
                        .db
                        .fetch_all_members_with_roles_chunked(&payload.server_id, roles)
                        .await;

                    debug!("role members: {:?}", _role_members);

                    if _role_members.is_err() {
                        revolt_config::capture_error(&_role_members.err().unwrap());
                        return;
                    }

                    let mut role_members = _role_members.unwrap();
                    let mut chunk = vec![];
                    let mut exhausted = false;

                    while !exhausted {
                        chunk.clear();

                        for _ in 0..config.pushd.mass_mention_chunk_size {
                            if let Some(member) = role_members.next().await {
                                chunk.push(member);
                            } else {
                                exhausted = true;
                                break;
                            }
                        }

                        let mut q = query.clone().members(&chunk);
                        let viewing_members: Vec<String> = q
                            .members_can_see_channel()
                            .await
                            .iter()
                            .filter_map(|(uid, viewable)| {
                                if *viewable && !existing_mentions.contains(uid) {
                                    Some(uid.clone())
                                } else {
                                    None
                                }
                            })
                            .collect();

                        debug!("viewing members: {:?}", viewing_members);

                        let online = revolt_presence::filter_online(&viewing_members).await;
                        debug!("online: {:?}", online);

                        let targets: Vec<String> = viewing_members
                            .iter()
                            .filter(|m| !online.contains(*m))
                            .cloned()
                            .collect();

                        debug!("targets: {:?}", targets);

                        self.fire_notification_for_users(&push, &targets).await;
                    }
                }
            }
        }
    }
}
