use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum System {
    Ping { event_time: String },
    Joined(JoinedLeft),
    Left(JoinedLeft),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinedLeft {
    pub event_time: String,
    pub channel: Channel,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub path: String,
    pub parent_id: String,
    pub creator: (), // User
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub icon_id: String,
    pub bot: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Message {
    MessageCreated(MessageCreatedUpdated),
    MessageDeleted(MessageDeleted),
    MessageUpdated(MessageCreatedUpdated),
    DirectMessageCreated(MessageCreatedUpdated),
    DirectMessageDeleted(MessageDeleted),
    DirectMessageUpdated(MessageCreatedUpdated),
    BotMessageStampsUpdated(BotMessageStampsUpdated),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageCreatedUpdated {
    pub event_time: String,
    pub message: MessageBody,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageBody {
    pub id: String,
    pub user: User,
    pub channel_id: String,
    pub text: String,
    pub plain_text: String,
    pub embedded: Vec<Embedded>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Embedded {
    pub raw: String,
    pub r#type: String,
    pub id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageDeleted {
    pub event_time: String,
    pub message: MessageDeletedBody,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageDeletedBody {
    pub id: String,
    #[serde(default)]
    pub user_id: String,
    pub channel_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BotMessageStampsUpdated {
    pub event_time: String,
    pub message_id: String,
    pub stamps: Vec<Stamp>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stamp {
    pub stamp_id: String,
    pub user_id: String,
    pub count: u32,
    pub created_at: String,
    pub updated_at: String,
}
