// Queue Type: Debounced
use crate::{Database, AMQP};

use deadqueue::limited::Queue;
use once_cell::sync::Lazy;
use std::{collections::HashMap, time::Duration};

use revolt_result::Result;

use super::{
    //apple_notifications::{self, ApnJob},
    DelayedTask,
};

/// Enumeration of possible events
#[derive(Debug, Eq, PartialEq)]
pub enum AckEvent {
    /// Add mentions for a user in a channel
    AddMention {
        /// Message IDs
        ids: Vec<String>,
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
    user: String,
    /// Event
    event: AckEvent,
}

#[derive(Debug)]
struct Task {
    event: AckEvent,
}

static Q: Lazy<Queue<Data>> = Lazy::new(|| Queue::new(10_000));

/// Queue a new task for a worker
pub async fn queue(channel: String, user: String, event: AckEvent) {
    Q.try_push(Data {
        channel,
        user,
        event,
    })
    .ok();

    info!("Queue is using {} slots from {}.", Q.len(), Q.capacity());
}

pub async fn handle_ack_event(
    event: &AckEvent,
    db: &Database,
    amqp: &AMQP,
    user: &str,
    channel: &str,
) -> Result<()> {
    match &event {
        #[allow(clippy::disallowed_methods)] // event is sent by higher level function
        AckEvent::AckMessage { id } => {
            if let Err(resp) = db.acknowledge_message(channel, user, id).await {
                revolt_config::capture_error(&resp);
            }

            if let Err(resp) = amqp
                .ack_message(user.into(), channel.into(), id.into())
                .await
            {
                revolt_config::capture_error(&resp);
            }
        }
        AckEvent::AddMention { ids } => {
            db.add_mention_to_unread(channel, user, ids).await?;
        }
    };

    Ok(())
}

/// Start a new worker
pub async fn worker(db: Database, amqp: AMQP) {
    let mut tasks = HashMap::<(String, String), DelayedTask<Task>>::new();
    let mut keys = vec![];

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
                let (user, channel) = key;

                if let Err(err) = handle_ack_event(&event, &db, &amqp, user, channel).await {
                    error!("{err:?} for {event:?}. ({user}, {channel})");
                } else {
                    info!("User {user} ack in {channel} with {event:?}");
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
            match &mut event {
                // this is a bit of a test, we're no longer delaying/batching AddMentions in an effort for
                // pushd to have the mention in the db when it sends a message notification.
                AckEvent::AddMention { .. } => {
                    handle_ack_event(&event, &db, &amqp, &user, &channel).await;
                }
                AckEvent::AckMessage { .. } => {
                    let key = (user, channel);
                    if let Some(task) = tasks.get_mut(&key) {
                        task.delay();
                        task.data.event = event;
                    } else {
                        tasks.insert(key, DelayedTask::new(Task { event }));
                    }
                }
            }
        }

        // Sleep for an arbitrary amount of time.
        async_std::task::sleep(Duration::from_secs(1)).await;
    }
}
