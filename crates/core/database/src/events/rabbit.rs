use std::collections::HashMap;

use revolt_models::v0::PushNotification;
use serde::{Deserialize, Serialize};

use crate::User;

#[derive(Serialize, Deserialize)]
pub struct MessageSentPayload {
    pub notification: PushNotification,
    pub users: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct MassMessageSentPayload {
    pub notifications: Vec<PushNotification>,
    pub server_id: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FRAcceptedPayload {
    pub accepted_user: User,
    pub user: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FRReceivedPayload {
    pub from_user: User,
    pub user: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GenericPayload {
    pub title: String,
    pub body: String,
    pub icon: Option<String>,
    pub user: User,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
#[allow(clippy::large_enum_variant)]
pub enum PayloadKind {
    MessageNotification(PushNotification),
    FRAccepted(FRAcceptedPayload),
    FRReceived(FRReceivedPayload),
    BadgeUpdate(usize),
    Generic(GenericPayload),
}

#[derive(Serialize, Deserialize)]
pub struct PayloadToService {
    pub notification: PayloadKind,
    pub user_id: String,
    pub session_id: String,
    pub token: String,
    pub extras: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct AckPayload {
    pub user_id: String,
    pub channel_id: String,
    pub message_id: String,
}
