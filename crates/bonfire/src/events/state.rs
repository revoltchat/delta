use std::{
    collections::{HashMap, HashSet},
    num::NonZero,
    sync::Arc,
    time::Duration,
};

use async_std::sync::{Mutex, RwLock};
use lru::LruCache;
use lru_time_cache::{LruCache as LruTimeCache, TimedEntry};
use revolt_database::{Channel, Member, Server, User};

/// Enumeration representing some change in subscriptions
pub enum SubscriptionStateChange {
    /// No change
    None,
    /// Clear all subscriptions
    Reset,
    /// Append or remove subscriptions
    Change {
        add: Vec<String>,
        remove: Vec<String>,
    },
}

/// Dumb per-state cache implementation
///
/// Ideally this would use a global cache that
/// allows for mutations and could use Rc<> to
/// track usage. If Rc<> == 1, then it only
/// remains in global cache, hence should be
/// dropped.
///
/// ------------------------------------------------
/// We can strip these objects to core information!!
/// ------------------------------------------------
#[derive(Debug)]
pub struct Cache {
    pub user_id: String,
    pub is_bot: bool,

    pub users: HashMap<String, User>,
    pub channels: HashMap<String, Channel>,
    pub members: HashMap<String, Member>,
    pub servers: HashMap<String, Server>,

    pub seen_events: LruCache<String, ()>,
}

impl Default for Cache {
    fn default() -> Self {
        Cache {
            user_id: Default::default(),
            is_bot: false,

            users: Default::default(),
            channels: Default::default(),
            members: Default::default(),
            servers: Default::default(),

            seen_events: LruCache::new(NonZero::new(20).unwrap()),
        }
    }
}

/// Client state
pub struct State {
    pub cache: Cache,

    pub session_id: String,
    pub private_topic: String,
    pub state: SubscriptionStateChange,

    pub subscribed: Arc<RwLock<HashSet<String>>>,
    pub active_servers: Arc<Mutex<LruTimeCache<String, ()>>>,
}

impl State {
    /// Create state from User
    pub fn from(user: User, session_id: String) -> State {
        let mut subscribed = HashSet::new();
        let private_topic = format!("{}!", user.id);
        subscribed.insert(private_topic.clone());
        subscribed.insert(user.id.clone());

        let mut cache: Cache = Cache {
            user_id: user.id.clone(),
            ..Default::default()
        };

        cache.users.insert(user.id.clone(), user);

        State {
            cache,
            subscribed: Arc::new(RwLock::new(subscribed)),
            active_servers: Arc::new(Mutex::new(LruTimeCache::with_expiry_duration_and_capacity(
                Duration::from_secs(900),
                5,
            ))),
            session_id,
            private_topic,
            state: SubscriptionStateChange::Reset,
        }
    }

    /// Apply currently queued state
    pub async fn apply_state(&mut self) -> SubscriptionStateChange {
        // Check if we need to change subscriptions to member event topics
        if !self.cache.is_bot {
            enum Server {
                Subscribe(String),
                Unsubscribe(String),
            }

            let active_server_changes: Vec<Server> = {
                let mut active_servers = self.active_servers.lock().await;
                active_servers
                    .notify_iter()
                    .map(|e| match e {
                        TimedEntry::Valid(k, _) => Server::Subscribe(format!("{}u", k)),
                        TimedEntry::Expired(k, _) => Server::Unsubscribe(format!("{}u", k)),
                    })
                    .collect()
                // It is bad practice to open more than one Mutex at once and could
                // lead to a deadlock, so instead we choose to collect the changes.
            };

            for entry in active_server_changes {
                match entry {
                    Server::Subscribe(k) => {
                        self.insert_subscription(k).await;
                    }
                    Server::Unsubscribe(k) => {
                        self.remove_subscription(&k).await;
                    }
                }
            }
        }

        // Flush changes to subscriptions
        let state = std::mem::replace(&mut self.state, SubscriptionStateChange::None);
        let mut subscribed = self.subscribed.write().await;
        if let SubscriptionStateChange::Change { add, remove } = &state {
            for id in add {
                subscribed.insert(id.clone());
            }

            for id in remove {
                subscribed.remove(id);
            }
        }

        state
    }

    /// Clone the active user
    pub fn clone_user(&self) -> User {
        self.cache.users.get(&self.cache.user_id).unwrap().clone()
    }

    /// Reset the current state
    pub async fn reset_state(&mut self) {
        self.state = SubscriptionStateChange::Reset;
        self.subscribed.write().await.clear();
    }

    /// Add a new subscription
    pub async fn insert_subscription(&mut self, subscription: String) {
        let mut subscribed = self.subscribed.write().await;
        if subscribed.contains(&subscription) {
            return;
        }

        match &mut self.state {
            SubscriptionStateChange::None => {
                self.state = SubscriptionStateChange::Change {
                    add: vec![subscription.clone()],
                    remove: vec![],
                };
            }
            SubscriptionStateChange::Change { add, .. } => {
                add.push(subscription.clone());
            }
            SubscriptionStateChange::Reset => {}
        }

        subscribed.insert(subscription);
    }

    /// Remove existing subscription
    pub async fn remove_subscription(&mut self, subscription: &str) {
        let mut subscribed = self.subscribed.write().await;
        if !subscribed.contains(&subscription.to_string()) {
            return;
        }

        match &mut self.state {
            SubscriptionStateChange::None => {
                self.state = SubscriptionStateChange::Change {
                    add: vec![],
                    remove: vec![subscription.to_string()],
                };
            }
            SubscriptionStateChange::Change { remove, .. } => {
                remove.push(subscription.to_string());
            }
            SubscriptionStateChange::Reset => panic!("Should not remove during a reset!"),
        }

        subscribed.remove(subscription);
    }
}
