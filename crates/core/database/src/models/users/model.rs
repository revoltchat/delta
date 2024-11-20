use std::{collections::HashSet, str::FromStr, time::Duration};

use crate::{events::client::EventV1, Database, File, RatelimitEvent};

use authifier::config::{EmailVerificationConfig, Template};
use iso8601_timestamp::Timestamp;
use once_cell::sync::Lazy;
use rand::seq::SliceRandom;
use revolt_config::{config, FeaturesLimits};
use revolt_models::v0::{self, UserFlags};
use revolt_presence::filter_online;
use revolt_result::{create_error, Result};
use serde_json::json;
use ulid::Ulid;

auto_derived_partial!(
    /// # User
    pub struct User {
        /// Unique Id
        #[serde(rename = "_id")]
        pub id: String,
        /// Username
        pub username: String,
        /// Discriminator
        pub discriminator: String,
        /// Display name
        #[serde(skip_serializing_if = "Option::is_none")]
        pub display_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        /// Avatar attachment
        pub avatar: Option<File>,
        /// Relationships with other users
        #[serde(skip_serializing_if = "Option::is_none")]
        pub relations: Option<Vec<Relationship>>,

        /// Bitfield of user badges
        #[serde(skip_serializing_if = "Option::is_none")]
        pub badges: Option<i32>,
        /// User's current status
        #[serde(skip_serializing_if = "Option::is_none")]
        pub status: Option<UserStatus>,
        /// User's profile page
        #[serde(skip_serializing_if = "Option::is_none")]
        pub profile: Option<UserProfile>,

        /// Enum of user flags
        #[serde(skip_serializing_if = "Option::is_none")]
        pub flags: Option<i32>,
        /// Whether this user is privileged
        #[serde(skip_serializing_if = "crate::if_false", default)]
        pub privileged: bool,
        /// Bot information
        #[serde(skip_serializing_if = "Option::is_none")]
        pub bot: Option<BotInformation>,

        /// Time until user is unsuspended
        #[serde(skip_serializing_if = "Option::is_none")]
        pub suspended_until: Option<Timestamp>,
    },
    "PartialUser"
);

auto_derived!(
    /// Optional fields on user object
    pub enum FieldsUser {
        Avatar,
        StatusText,
        StatusPresence,
        ProfileContent,
        ProfileBackground,
        DisplayName,

        // internal fields
        Suspension,
        None,
    }

    /// User's relationship with another user (or themselves)
    pub enum RelationshipStatus {
        None,
        User,
        Friend,
        Outgoing,
        Incoming,
        Blocked,
        BlockedOther,
    }

    /// Relationship entry indicating current status with other user
    pub struct Relationship {
        #[serde(rename = "_id")]
        pub id: String,
        pub status: RelationshipStatus,
    }

    /// Presence status
    pub enum Presence {
        /// User is online
        Online,
        /// User is not currently available
        Idle,
        /// User is focusing / will only receive mentions
        Focus,
        /// User is busy / will not receive any notifications
        Busy,
        /// User appears to be offline
        Invisible,
    }

    /// User's active status
    #[derive(Default)]
    pub struct UserStatus {
        /// Custom status text
        #[serde(skip_serializing_if = "Option::is_none")]
        pub text: Option<String>,
        /// Current presence option
        #[serde(skip_serializing_if = "Option::is_none")]
        pub presence: Option<Presence>,
    }

    /// User's profile
    #[derive(Default)]
    pub struct UserProfile {
        /// Text content on user's profile
        #[serde(skip_serializing_if = "Option::is_none")]
        pub content: Option<String>,
        /// Background visible on user's profile
        #[serde(skip_serializing_if = "Option::is_none")]
        pub background: Option<File>,
    }

    /// Bot information for if the user is a bot
    pub struct BotInformation {
        /// Id of the owner of this bot
        pub owner: String,
    }

    /// Enumeration providing a hint to the type of user we are handling
    pub enum UserHint {
        /// Could be either a user or a bot
        Any,
        /// Only match bots
        Bot,
        /// Only match users
        User,
    }
);

pub static DISCRIMINATOR_SEARCH_SPACE: Lazy<HashSet<String>> = Lazy::new(|| {
    let mut set = (2..9999)
        .map(|v| format!("{:0>4}", v))
        .collect::<HashSet<String>>();

    for discrim in [
        123, 1234, 1111, 2222, 3333, 4444, 5555, 6666, 7777, 8888, 9999,
    ] {
        set.remove(&format!("{:0>4}", discrim));
    }

    set.into_iter().collect()
});

#[allow(clippy::derivable_impls)]
impl Default for User {
    fn default() -> Self {
        Self {
            id: Default::default(),
            username: Default::default(),
            discriminator: Default::default(),
            display_name: Default::default(),
            avatar: Default::default(),
            relations: Default::default(),
            badges: Default::default(),
            status: Default::default(),
            profile: Default::default(),
            flags: Default::default(),
            privileged: Default::default(),
            bot: Default::default(),
            suspended_until: Default::default(),
        }
    }
}

#[allow(clippy::disallowed_methods)]
impl User {
    /// Create a new user
    pub async fn create<I, D>(
        db: &Database,
        username: String,
        account_id: I,
        data: D,
    ) -> Result<User>
    where
        I: Into<Option<String>>,
        D: Into<Option<PartialUser>>,
    {
        let username = User::validate_username(username)?;
        let mut user = User {
            id: account_id.into().unwrap_or_else(|| Ulid::new().to_string()),
            discriminator: User::find_discriminator(db, &username, None).await?,
            username,
            ..Default::default()
        };

        if let Some(data) = data.into() {
            user.apply_options(data);
        }

        db.insert_user(&user).await?;
        Ok(user)
    }

    /// Get limits for this user
    pub async fn limits(&self) -> FeaturesLimits {
        let config = config().await;
        if ulid::Ulid::from_str(&self.id)
            .expect("`ulid`")
            .datetime()
            .elapsed()
            .expect("time went backwards")
            <= Duration::from_secs(3600u64 * config.features.limits.global.new_user_hours as u64)
        {
            config.features.limits.new_user
        } else {
            config.features.limits.default
        }
    }

    /// Get the relationship with another user
    pub fn relationship_with(&self, user_b: &str) -> RelationshipStatus {
        if self.id == user_b {
            return RelationshipStatus::User;
        }

        if let Some(relations) = &self.relations {
            if let Some(relationship) = relations.iter().find(|x| x.id == user_b) {
                return relationship.status.clone();
            }
        }

        RelationshipStatus::None
    }

    pub fn is_friends_with(&self, user_b: &str) -> bool {
        matches!(
            self.relationship_with(user_b),
            RelationshipStatus::Friend | RelationshipStatus::User
        )
    }

    /// Check whether two users have a mutual connection
    ///
    /// This will check if user and user_b share a server or a group.
    pub async fn has_mutual_connection(&self, db: &Database, user_b: &str) -> Result<bool> {
        Ok(!db
            .fetch_mutual_server_ids(&self.id, user_b)
            .await?
            .is_empty()
            || !db
                .fetch_mutual_channel_ids(&self.id, user_b)
                .await?
                .is_empty())
    }

    /// Check if this user can acquire another server
    pub async fn can_acquire_server(&self, db: &Database) -> Result<()> {
        if db.fetch_server_count(&self.id).await? <= self.limits().await.servers {
            Ok(())
        } else {
            Err(create_error!(TooManyServers {
                max: self.limits().await.servers
            }))
        }
    }

    /// Sanitise and validate a username can be used
    pub fn validate_username(username: String) -> Result<String> {
        // Copy the username for validation
        let username_lowercase = username.to_lowercase();

        // Block homoglyphs
        if decancer::cure!(&username_lowercase).map_err(|_| create_error!(InvalidUsername))?
            != username_lowercase
        {
            return Err(create_error!(InvalidUsername));
        }

        // Ensure the username itself isn't blocked
        const BLOCKED_USERNAMES: &[&str] = &["admin", "revolt"];

        for username in BLOCKED_USERNAMES {
            if username_lowercase == *username {
                return Err(create_error!(InvalidUsername));
            }
        }

        // Ensure none of the following substrings show up in the username
        const BLOCKED_SUBSTRINGS: &[&str] = &["```"];

        for substr in BLOCKED_SUBSTRINGS {
            if username_lowercase.contains(substr) {
                return Err(create_error!(InvalidUsername));
            }
        }

        Ok(username)
    }

    /// Find a user and session ID from a given token and hint
    #[async_recursion]
    pub async fn from_token(db: &Database, token: &str, hint: UserHint) -> Result<(User, String)> {
        match hint {
            UserHint::Bot => Ok((
                db.fetch_user(
                    &db.fetch_bot_by_token(token)
                        .await
                        .map_err(|_| create_error!(InvalidSession))?
                        .id,
                )
                .await?,
                String::new(),
            )),
            UserHint::User => {
                let session = db.fetch_session_by_token(token).await?;
                Ok((db.fetch_user(&session.user_id).await?, session.id))
            }
            UserHint::Any => {
                if let Ok(result) = User::from_token(db, token, UserHint::User).await {
                    Ok(result)
                } else {
                    User::from_token(db, token, UserHint::Bot).await
                }
            }
        }
    }

    /// Helper function to fetch many users as a mutually connected user
    /// (while optimising the online ID query)
    pub async fn fetch_many_ids_as_mutuals(
        db: &Database,
        perspective: &User,
        ids: &[String],
    ) -> Result<Vec<v0::User>> {
        let online_ids = filter_online(ids).await;

        Ok(db
            .fetch_users(ids)
            .await?
            .into_iter()
            .map(|user| {
                let is_online = online_ids.contains(&user.id);
                user.into_known(perspective, is_online)
            })
            .collect())
    }

    /// Find a free discriminator for a given username
    pub async fn find_discriminator(
        db: &Database,
        username: &str,
        preferred: Option<(String, String)>,
    ) -> Result<String> {
        let search_space: &HashSet<String> = &DISCRIMINATOR_SEARCH_SPACE;
        let used_discriminators: HashSet<String> = db
            .fetch_discriminators_in_use(username)
            .await?
            .into_iter()
            .collect();

        let available_discriminators: Vec<&String> =
            search_space.difference(&used_discriminators).collect();

        if available_discriminators.is_empty() {
            return Err(create_error!(UsernameTaken));
        }

        if let Some((preferred, target_id)) = preferred {
            if available_discriminators.contains(&&preferred) {
                return Ok(preferred);
            } else {
                if db
                    .has_ratelimited(
                        &target_id,
                        crate::RatelimitEventType::DiscriminatorChange,
                        Duration::from_secs(60 * 60 * 24),
                        1,
                    )
                    .await?
                {
                    return Err(create_error!(DiscriminatorChangeRatelimited));
                }

                RatelimitEvent::create(
                    db,
                    target_id,
                    crate::RatelimitEventType::DiscriminatorChange,
                )
                .await?;
            }
        }

        let mut rng = rand::thread_rng();
        Ok(available_discriminators
            .choose(&mut rng)
            .expect("we can assert this has an element")
            .to_string())
    }

    /// Update a user's username
    pub async fn update_username(&mut self, db: &Database, username: String) -> Result<()> {
        let username = User::validate_username(username)?;
        if self.username.to_lowercase() == username.to_lowercase() {
            self.update(
                db,
                PartialUser {
                    username: Some(username),
                    ..Default::default()
                },
                vec![],
            )
            .await
        } else {
            self.update(
                db,
                PartialUser {
                    discriminator: Some(
                        User::find_discriminator(
                            db,
                            &username,
                            Some((self.discriminator.to_string(), self.id.clone())),
                        )
                        .await?,
                    ),
                    username: Some(username),
                    ..Default::default()
                },
                vec![],
            )
            .await
        }
    }

    /// Set a relationship to another user
    pub async fn set_relationship(
        &mut self,
        db: &Database,
        user_b: &User,
        status: RelationshipStatus,
    ) -> Result<()> {
        db.set_relationship(&self.id, &user_b.id, &status).await?;

        if let RelationshipStatus::None | RelationshipStatus::User = status {
            if let Some(relations) = &mut self.relations {
                relations.retain(|relation| relation.id != user_b.id);
            }
        } else {
            let relation = Relationship {
                id: user_b.id.to_string(),
                status,
            };

            if let Some(relations) = &mut self.relations {
                relations.retain(|relation| relation.id != user_b.id);
                relations.push(relation);
            } else {
                self.relations = Some(vec![relation]);
            }
        }

        Ok(())
    }

    /// Apply a certain relationship between two users
    pub async fn apply_relationship(
        &mut self,
        db: &Database,
        target: &mut User,
        local: RelationshipStatus,
        remote: RelationshipStatus,
    ) -> Result<()> {
        target.set_relationship(db, self, remote).await?;
        self.set_relationship(db, target, local).await?;

        EventV1::UserRelationship {
            id: target.id.clone(),
            user: self.clone().into(db, Some(&*target)).await,
        }
        .private(target.id.clone())
        .await;

        EventV1::UserRelationship {
            id: self.id.clone(),
            user: target.clone().into(db, Some(&*self)).await,
        }
        .private(self.id.clone())
        .await;

        Ok(())
    }

    /// Add another user as a friend
    pub async fn add_friend(&mut self, db: &Database, target: &mut User) -> Result<()> {
        match self.relationship_with(&target.id) {
            RelationshipStatus::User => Err(create_error!(NoEffect)),
            RelationshipStatus::Friend => Err(create_error!(AlreadyFriends)),
            RelationshipStatus::Outgoing => Err(create_error!(AlreadySentRequest)),
            RelationshipStatus::Blocked => Err(create_error!(Blocked)),
            RelationshipStatus::BlockedOther => Err(create_error!(BlockedByOther)),
            RelationshipStatus::Incoming => {
                // Accept incoming friend request
                self.apply_relationship(
                    db,
                    target,
                    RelationshipStatus::Friend,
                    RelationshipStatus::Friend,
                )
                .await
            }
            RelationshipStatus::None => {
                // Get this user's current count of outgoing friend requests
                let count = self
                    .relations
                    .as_ref()
                    .map(|relations| {
                        relations
                            .iter()
                            .filter(|r| matches!(r.status, RelationshipStatus::Outgoing))
                            .count()
                    })
                    .unwrap_or_default();

                // If we're over the limit, don't allow creating more requests
                if count >= self.limits().await.outgoing_friend_requests {
                    return Err(create_error!(TooManyPendingFriendRequests {
                        max: self.limits().await.outgoing_friend_requests
                    }));
                }

                // Send the friend request
                self.apply_relationship(
                    db,
                    target,
                    RelationshipStatus::Outgoing,
                    RelationshipStatus::Incoming,
                )
                .await
            }
        }
    }

    /// Remove another user as a friend
    pub async fn remove_friend(&mut self, db: &Database, target: &mut User) -> Result<()> {
        match self.relationship_with(&target.id) {
            RelationshipStatus::Friend
            | RelationshipStatus::Outgoing
            | RelationshipStatus::Incoming => {
                self.apply_relationship(
                    db,
                    target,
                    RelationshipStatus::None,
                    RelationshipStatus::None,
                )
                .await
            }
            _ => Err(create_error!(NoEffect)),
        }
    }

    /// Block another user
    pub async fn block_user(&mut self, db: &Database, target: &mut User) -> Result<()> {
        match self.relationship_with(&target.id) {
            RelationshipStatus::User | RelationshipStatus::Blocked => Err(create_error!(NoEffect)),
            RelationshipStatus::BlockedOther => {
                self.apply_relationship(
                    db,
                    target,
                    RelationshipStatus::Blocked,
                    RelationshipStatus::Blocked,
                )
                .await
            }
            RelationshipStatus::None
            | RelationshipStatus::Friend
            | RelationshipStatus::Incoming
            | RelationshipStatus::Outgoing => {
                self.apply_relationship(
                    db,
                    target,
                    RelationshipStatus::Blocked,
                    RelationshipStatus::BlockedOther,
                )
                .await
            }
        }
    }

    /// Unblock another user
    pub async fn unblock_user(&mut self, db: &Database, target: &mut User) -> Result<()> {
        match self.relationship_with(&target.id) {
            RelationshipStatus::Blocked => match target.relationship_with(&self.id) {
                RelationshipStatus::Blocked => {
                    self.apply_relationship(
                        db,
                        target,
                        RelationshipStatus::BlockedOther,
                        RelationshipStatus::Blocked,
                    )
                    .await
                }
                RelationshipStatus::BlockedOther => {
                    self.apply_relationship(
                        db,
                        target,
                        RelationshipStatus::None,
                        RelationshipStatus::None,
                    )
                    .await
                }
                _ => Err(create_error!(InternalError)),
            },
            _ => Err(create_error!(NoEffect)),
        }
    }

    /// Update user data
    pub async fn update<'a>(
        &mut self,
        db: &Database,
        partial: PartialUser,
        remove: Vec<FieldsUser>,
    ) -> Result<()> {
        for field in &remove {
            self.remove_field(field);
        }

        self.apply_options(partial.clone());
        db.update_user(&self.id, &partial, remove.clone()).await?;

        EventV1::UserUpdate {
            id: self.id.clone(),
            data: partial.into(),
            clear: remove.into_iter().map(|v| v.into()).collect(),
            event_id: Some(Ulid::new().to_string()),
        }
        .p_user(self.id.clone(), db)
        .await;

        Ok(())
    }

    /// Remove a field from User object
    pub fn remove_field(&mut self, field: &FieldsUser) {
        match field {
            FieldsUser::Avatar => self.avatar = None,
            FieldsUser::StatusText => {
                if let Some(x) = self.status.as_mut() {
                    x.text = None;
                }
            }
            FieldsUser::StatusPresence => {
                if let Some(x) = self.status.as_mut() {
                    x.presence = None;
                }
            }
            FieldsUser::ProfileContent => {
                if let Some(x) = self.profile.as_mut() {
                    x.content = None;
                }
            }
            FieldsUser::ProfileBackground => {
                if let Some(x) = self.profile.as_mut() {
                    x.background = None;
                }
            }
            FieldsUser::DisplayName => self.display_name = None,
            FieldsUser::Suspension => self.suspended_until = None,
            FieldsUser::None => {}
        }
    }

    /// Suspend the user
    ///
    /// - If a duration is specified, the user will be automatically unsuspended after the given time.
    /// - If a reason is specified, an email will be sent.
    pub async fn suspend(
        &mut self,
        db: &Database,
        duration_days: Option<usize>,
        reason: Option<Vec<String>>,
    ) -> Result<()> {
        let authifier = db.clone().to_authifier().await;
        let mut account = authifier
            .database
            .find_account(&self.id)
            .await
            .map_err(|_| create_error!(InternalError))?;

        account
            .disable(&authifier)
            .await
            .map_err(|_| create_error!(InternalError))?;

        account
            .delete_all_sessions(&authifier, None)
            .await
            .map_err(|_| create_error!(InternalError))?;

        self.update(
            db,
            PartialUser {
                flags: Some(UserFlags::SuspendedUntil as i32),
                suspended_until: duration_days.and_then(|dur| {
                    Timestamp::now_utc().checked_add(iso8601_timestamp::Duration::days(dur as i64))
                }),
                ..Default::default()
            },
            vec![],
        )
        .await?;

        if let Some(reason) = reason {
            if let EmailVerificationConfig::Enabled { smtp, .. } =
                authifier.config.email_verification
            {
                smtp.send_email(
                    account.email.clone(),
                    // maybe move this to common area?
                    &Template {
                        title: "Account Suspension".to_string(),
                        html: Some(include_str!("../../../templates/suspension.html").to_owned()),
                        text: include_str!("../../../templates/suspension.txt").to_owned(),
                        url: Default::default(),
                    },
                    json!({
                        "email": account.email,
                        "list": reason.join(", "),
                        "duration": duration_days,
                        "duration_display": if duration_days.is_some() {
                            "block"
                        } else {
                            "none"
                        }
                    }),
                )
                .map_err(|_| create_error!(InternalError))?;
            }
        }

        Ok(())
    }

    /// Unsuspend the user
    pub async fn unsuspend(&mut self, db: &Database) -> Result<()> {
        self.update(
            db,
            PartialUser {
                flags: Some(0),
                suspended_until: None,
                ..Default::default()
            },
            vec![],
        )
        .await?;

        unimplemented!()
    }

    /// Permanently ban the user
    ///
    /// - If a reason is specified, an email will be sent.
    pub async fn ban(&mut self, _db: &Database, _reason: Option<String>) -> Result<()> {
        // Send ban email (if reason provided)
        unimplemented!()
    }

    /// Mark as deleted
    pub async fn mark_deleted(&mut self, db: &Database) -> Result<()> {
        self.update(
            db,
            PartialUser {
                username: Some(format!("Deleted User {}", self.id)),
                flags: Some(2),
                ..Default::default()
            },
            vec![
                FieldsUser::Avatar,
                FieldsUser::StatusText,
                FieldsUser::StatusPresence,
                FieldsUser::ProfileContent,
                FieldsUser::ProfileBackground,
                FieldsUser::Suspension,
            ],
        )
        .await
    }
}
