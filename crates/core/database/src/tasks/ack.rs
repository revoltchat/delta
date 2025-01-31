// Queue Type: Debounced
use crate::{Database, Message, AMQP};

use deadqueue::limited::Queue;
use once_cell::sync::Lazy;
use revolt_models::v0::PushNotification;
use rocket::form::validate::Contains;
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use validator::HasLen;

use revolt_result::Result;

use super::DelayedTask;
use crate::Channel::{TextChannel, VoiceChannel};

/// Enumeration of possible events
#[derive(Debug, Eq, PartialEq)]
pub enum AckEvent {
    /// Add mentions for a channel
    ProcessMessage {
        /// push notification, message, recipients, push silenced
        messages: Vec<(Option<PushNotification>, Message, Vec<String>, bool)>,
    },

    /// Acknowledge message in a channel for a user
    AckMessage {
        /// Message ID
        id: String,
    },
}

/// Task information
struct Data {
    /// Channel to ack
    channel: String,
    /// User to ack for
    user: Option<String>,
    /// Event
    event: AckEvent,
}

#[derive(Debug)]
struct Task {
    event: AckEvent,
}

static Q: Lazy<Queue<Data>> = Lazy::new(|| Queue::new(10_000));

/// Queue a new task for a worker
pub async fn queue_ack(channel: String, user: String, event: AckEvent) {
    Q.try_push(Data {
        channel,
        user: Some(user),
        event,
    })
    .ok();

    info!(
        "Queue is using {} slots from {}. Queued type: ACK",
        Q.len(),
        Q.capacity()
    );
}

pub async fn queue_message(channel: String, event: AckEvent) {
    Q.try_push(Data {
        channel,
        user: None,
        event,
    })
    .ok();

    info!(
        "Queue is using {} slots from {}. Queued type: MENTION",
        Q.len(),
        Q.capacity()
    );
}

pub async fn handle_ack_event(
    event: &AckEvent,
    db: &Database,
    amqp: &AMQP,
    user: &Option<String>,
    channel: &str,
) -> Result<()> {
    match &event {
        #[allow(clippy::disallowed_methods)] // event is sent by higher level function
        AckEvent::AckMessage { id } => {
            let user = user.as_ref().unwrap();
            let user: &str = user.as_str();

            let unread = db.fetch_unread(user, channel).await?;
            let updated = db.acknowledge_message(channel, user, id).await?;

            if let (Some(before), Some(after)) = (unread, updated) {
                let before_mentions = before.mentions.unwrap_or_default().len();
                let after_mentions = after.mentions.unwrap_or_default().len();

                let mentions_acked = before_mentions - after_mentions;

                if mentions_acked > 0 {
                    if let Err(err) = amqp
                        .ack_message(user.to_string(), channel.to_string(), id.to_owned())
                        .await
                    {
                        revolt_config::capture_error(&err);
                    }
                };
            }
        }
        AckEvent::ProcessMessage { messages } => {
            let mut users: HashSet<&String> = HashSet::new();
            debug!(
                "Processing {} messages from channel {}",
                messages.len(),
                messages[0].1.channel
            );

            // find all the users we'll be notifying
            messages.iter().for_each(|(_, _, recipents, _)| {
                users.extend(recipents.iter());
            });

            debug!("Found {} users to notify.", users.len());

            for user in users {
                let message_ids: Vec<String> = messages
                    .iter()
                    .filter_map(|(_, message, recipients, _)| {
                        if recipients.contains(user) {
                            Some(message.id.clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                if !message_ids.is_empty() {
                    db.add_mention_to_unread(channel, user, &message_ids)
                        .await?;
                }
                debug!("Added {} mentions for user {}", message_ids.len(), &user);
            }

            let mut mass_mentions = vec![];

            for (push, message, recipients, silenced) in messages {
                if *silenced
                    || push.is_none()
                    || (recipients.is_empty() && !message.contains_mass_push_mention())
                {
                    debug!(
                        "Rejecting push: silenced: {}, recipient count: {}, push exists: {:?}",
                        *silenced,
                        recipients.length(),
                        push.is_some()
                    );
                    continue;
                }

                debug!(
                    "Sending push event to AMQP; message {} for {} users",
                    push.as_ref().unwrap().message.id,
                    recipients.len()
                );
                if let Err(err) = amqp
                    .message_sent(recipients.clone(), push.clone().unwrap())
                    .await
                {
                    revolt_config::capture_error(&err);
                }

                if message.contains_mass_push_mention() {
                    mass_mentions.push(push.clone().unwrap());
                }
            }

            if !mass_mentions.is_empty() {
                debug!(
                    "Sending mass mention push event to AMQP; channel {}",
                    &mass_mentions[0].message.channel
                );

                let channel = db
                    .fetch_channel(&mass_mentions[0].message.channel)
                    .await
                    .expect("Failed to fetch channel from db");

                match channel {
                    TextChannel { server, .. } | VoiceChannel { server, .. } => {
                        if let Err(err) =
                            amqp.mass_mention_message_sent(server, mass_mentions).await
                        {
                            revolt_config::capture_error(&err);
                        }
                    }
                    _ => {
                        panic!("Unknown channel type when sending mass mention event");
                    }
                }
            }
        }
    };

    Ok(())
}

/// Start a new worker
pub async fn worker(db: Database, amqp: AMQP) {
    let mut tasks = HashMap::<(Option<String>, String, u8), DelayedTask<Task>>::new();
    let mut keys: Vec<(Option<String>, String, u8)> = vec![];

    loop {
        // Find due tasks.
        for (key, task) in &tasks {
            if task.should_run() {
                keys.push(key.clone());
            }
        }

        // Commit any due tasks to the database.
        for key in &keys {
            if let Some(task) = tasks.remove(key) {
                let Task { event } = task.data;
                let (user, channel, _) = key;

                if let Err(err) = handle_ack_event(&event, &db, &amqp, user, channel).await {
                    revolt_config::capture_error(&err);
                    error!("{err:?} for {event:?}. ({user:?}, {channel})");
                } else {
                    debug!("User {user:?} ack in {channel} with {event:?}");
                }
            }
        }

        // Clear keys
        keys.clear();

        // Queue incoming tasks.
        while let Some(Data {
            channel,
            user,
            mut event,
        }) = Q.try_pop()
        {
            let key: (Option<String>, String, u8) = (
                user,
                channel,
                match &event {
                    AckEvent::AckMessage { .. } => 0,
                    AckEvent::ProcessMessage { .. } => 1,
                },
            );
            if let Some(task) = tasks.get_mut(&key) {
                match &mut event {
                    AckEvent::ProcessMessage { messages: new_data } => {
                        if let AckEvent::ProcessMessage { messages: existing } =
                            &mut task.data.event
                        {
                            // add the new message to the list of messages to be processed.
                            existing.append(new_data);

                            // if the message contains a mass mention, do not delay it any further.
                            if new_data[0].1.contains_mass_push_mention() {
                                task.run_immediately();
                                continue;
                            }

                            // put a cap on the amount of messages that can be queued, for particularly active channels
                            if (existing.length() as u16)
                                < revolt_config::config()
                                    .await
                                    .features
                                    .advanced
                                    .process_message_delay_limit
                            {
                                task.delay();
                            }
                        } else {
                            panic!("Somehow got an ack message in the add mention arm");
                        }
                    }
                    AckEvent::AckMessage { .. } => {
                        // replace the last acked message with the new acked message
                        task.data.event = event;
                        task.delay();
                    }
                }
            } else {
                tasks.insert(key, DelayedTask::new(Task { event }));
            }
        }

        // Sleep for an arbitrary amount of time.
        async_std::task::sleep(Duration::from_secs(1)).await;
    }
}
