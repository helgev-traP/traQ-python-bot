use serde::{Deserialize, Serialize};

pub mod fields;
pub use fields::*;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Event {
    System { req_id: String, body: System },
    Message { req_id: String, body: Message },
    // todo: not implemented yet
    // Channel,
    // User,
    // UserGroup,
    // Stamp,
    // Tag,
}

impl Event {
    pub fn from_json(json: &str) -> Result<Event, serde_json::Error> {
        let cache: serde_json::Value = serde_json::from_str(json).unwrap();

        let r#type = cache.get("type").unwrap().as_str().unwrap();
        let req_id = cache.get("reqId").unwrap().as_str().unwrap();
        let body = cache.get("body").unwrap().clone();

        match r#type {
            // System
            "PING" => Ok(Event::System {
                req_id: req_id.to_string(),
                body: System::Ping {
                    event_time: serde_json::from_value(body).unwrap(),
                },
            }),
            "JOINED" => Ok(Event::System {
                req_id: req_id.to_string(),
                body: System::Joined(serde_json::from_value(body).unwrap()),
            }),
            "LEFT" => Ok(Event::System {
                req_id: req_id.to_string(),
                body: System::Left(serde_json::from_value(body).unwrap()),
            }),
            // Message
            "MESSAGE_CREATED" => Ok(Event::Message {
                req_id: req_id.to_string(),
                body: Message::MessageCreated(serde_json::from_value(body).unwrap()),
            }),
            "MESSAGE_DELETED" => Ok(Event::Message {
                req_id: req_id.to_string(),
                body: Message::MessageDeleted(serde_json::from_value(body).unwrap()),
            }),
            "MESSAGE_UPDATED" => Ok(Event::Message {
                req_id: req_id.to_string(),
                body: Message::MessageUpdated(serde_json::from_value(body).unwrap()),
            }),
            "DIRECT_MESSAGE_CREATED" => Ok(Event::Message {
                req_id: req_id.to_string(),
                body: Message::DirectMessageCreated(serde_json::from_value(body).unwrap()),
            }),
            "DIRECT_MESSAGE_DELETED" => Ok(Event::Message {
                req_id: req_id.to_string(),
                body: Message::DirectMessageDeleted(serde_json::from_value(body).unwrap()),
            }),
            "DIRECT_MESSAGE_UPDATED" => Ok(Event::Message {
                req_id: req_id.to_string(),
                body: Message::DirectMessageUpdated(serde_json::from_value(body).unwrap()),
            }),
            "BOT_MESSAGE_STAMPS_UPDATED" => Ok(Event::Message {
                req_id: req_id.to_string(),
                body: Message::BotMessageStampsUpdated(serde_json::from_value(body).unwrap()),
            }),
            // channel
            "CHANNEL_CREATED" => todo!(),
            "CHANNEL_TOPIC_CHANGED" => todo!(),
            // user
            "USER_CREATED" => todo!(),
            "USER_ACTIVATED" => todo!(),
            // user group
            "USER_GROUP_CREATED" => todo!(),
            "USER_GROUP_UPDATED" => todo!(),
            "USER_GROUP_DELETED" => todo!(),
            "USER_GROUP_MEMBER_ADDED" => todo!(),
            "USER_GROUP_MEMBER_UPDATED" => todo!(),
            "USER_GROUP_MEMBER_REMOVED" => todo!(),
            "USER_GROUP_ADMIN_ADDED" => todo!(),
            "USER_GROUP_ADMIN_REMOVED" => todo!(),
            // stamp
            "STAMP_CREATED" => todo!(),
            // tag
            "TAG_ADDED" => todo!(),
            "TAG_REMOVED" => todo!(),
            // invalid
            _ => panic!("Invalid event type: {}", r#type),
        }
    }
}
