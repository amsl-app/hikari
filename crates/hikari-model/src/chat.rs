use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{Display, IntoStaticStr};
use utoipa::ToSchema;
use uuid::Uuid;

pub use csml_model::Client;
use csml_model::FlowTrigger;

use crate::journal::MetaJournalEntryWithMetaContent;
use crate::llm::message::ConversationMessage;

pub trait MergeContent {
    fn merge(&mut self, other: Self) -> Result<(), anyhow::Error>;
}

#[derive(Deserialize, ToSchema)]
pub struct ClientInfo {
    pub client: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct FlowInfo<'a> {
    #[schema(example = "flow-id")]
    pub id: &'a str,

    #[schema(example = "flow-name")]
    pub name: &'a str,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct BotInfo<'a> {
    #[schema(example = "bot-name")]
    pub name: &'a str,

    #[schema(example = "bot-id")]
    pub id: &'a str,

    pub flows: Vec<FlowInfo<'a>>,
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct ConversationInfo {
    pub client: Client,
    pub flow_id: String,
    pub step_id: String,
    pub last_interaction_at: DateTime<FixedOffset>,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Direction {
    Send,
    Receive,
}

impl MergeContent for TextContent {
    fn merge(&mut self, other: Self) -> Result<(), anyhow::Error> {
        self.text.push_str(&other.text);
        Ok(())
    }
}

#[derive(ToSchema, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct TextContent {
    pub text: String,
}

impl MergeContent for PayloadContent {
    fn merge(&mut self, other: Self) -> Result<(), anyhow::Error> {
        if self.content_type != other.content_type || self.display_type != other.display_type {
            return Err(anyhow::anyhow!("Payload types are not the same"));
        }
        self.payload.push_str(&other.payload);
        Ok(())
    }
}

#[derive(ToSchema, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct PayloadContent {
    pub payload: String,
    pub content_type: Option<PayloadContentType>,
    pub display_type: Option<PayloadDisplayType>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, PartialEq, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum PayloadContentType {
    JournalFocus,
    JournalMood,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, PartialEq, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum PayloadDisplayType {
    Duration,
}

#[derive(ToSchema, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct ButtonContent {
    pub title: String,
    pub payload: Option<String>,
}

#[derive(ToSchema, Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct TypingContent {
    pub duration: Option<u16>,
}

impl MergeContent for TypeSafePayload {
    fn merge(&mut self, other: Self) -> Result<(), anyhow::Error> {
        match (self, other) {
            (TypeSafePayload::Text(s), TypeSafePayload::Text(o)) => s.merge(o),
            (TypeSafePayload::Payload(s), TypeSafePayload::Payload(o)) => s.merge(o),
            _ => Err(anyhow::anyhow!("Payload types are not the same")),
        }
    }
}

#[derive(ToSchema, Serialize, Deserialize, Debug, Clone, PartialEq, IntoStaticStr, Display)]
#[serde(rename_all = "snake_case", tag = "content_type", content = "content")]
#[strum(serialize_all = "snake_case")]
pub enum TypeSafePayload {
    Text(TextContent),
    Payload(PayloadContent),
    Button(ButtonContent),
    FlowTrigger(FlowTrigger),
}

impl TypeSafePayload {
    #[must_use]
    pub fn message_string(self) -> Option<String> {
        match self {
            TypeSafePayload::Text(text) => Some(text.text),
            TypeSafePayload::Payload(payload) => Some(payload.payload),
            TypeSafePayload::Button(_) | TypeSafePayload::FlowTrigger(_) => None,
        }
    }
}

impl TryFrom<TypeSafePayload> for Payload {
    type Error = serde_json::Error;

    fn try_from(value: TypeSafePayload) -> Result<Self, Self::Error> {
        let content_type = value.to_string();
        let inner = match value {
            TypeSafePayload::Text(inner) => Some(serde_json::to_value(inner)?),
            TypeSafePayload::Payload(inner) => Some(serde_json::to_value(inner)?),
            TypeSafePayload::Button(inner) => Some(serde_json::to_value(inner)?),
            TypeSafePayload::FlowTrigger(inner) => Some(serde_json::to_value(inner)?),
        };
        Ok(Payload {
            content_type,
            content: inner,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, utoipa::ToSchema, PartialEq)]
pub struct Payload {
    pub content_type: String,
    #[schema(value_type = Object)]
    pub content: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Message<T> {
    pub payload: T,
    pub direction: Direction,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "kebab-case")]
pub enum CreatableEntity {
    JournalEntry(MetaJournalEntryWithMetaContent),
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MessageResponse<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client: Option<Client>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<Uuid>,
    pub conversation_end: bool,
    pub history: bool,
    pub messages: Vec<Message<T>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub created_entities: Vec<CreatableEntity>,
}

impl From<ConversationMessage> for MessageResponse<TypeSafePayload> {
    fn from(conversation_message: ConversationMessage) -> Self {
        let message = Message {
            payload: conversation_message.message,
            direction: conversation_message.direction,
        };
        MessageResponse {
            client: None,
            request_id: None,
            conversation_id: Some(conversation_message.conversation_id),
            conversation_end: false,
            history: false,
            messages: vec![message],
            created_entities: vec![],
        }
    }
}

impl MessageResponse<TypeSafePayload> {
    #[must_use]
    pub fn conversation_end() -> MessageResponse<TypeSafePayload> {
        MessageResponse {
            client: None,
            request_id: None,
            conversation_id: None,
            conversation_end: true,
            history: false,
            messages: vec![],
            created_entities: vec![],
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    pub status_code: u16,
}

#[derive(Debug, Serialize, Deserialize, Default, ToSchema)]
pub struct RequestMetadata {
    pub time: Option<DateTime<FixedOffset>>,
}

#[derive(Deserialize, ToSchema)]
pub struct Request {
    pub client: String,
    pub payload: Payload,
    pub metadata: Option<RequestMetadata>,
}

#[derive(ToSchema, Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub struct ChatMode {
    #[serde(default)]
    pub voice_mode: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ChatRequest<T> {
    pub payload: T,
    #[serde(default)]
    pub metadata: RequestMetadata,
    #[serde(default)]
    pub chat_mode: ChatMode,
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    #[test]
    fn serialize_content() {
        let serialized = serde_json::to_value(TypeSafePayload::Text(TextContent { text: String::new() })).unwrap();
        let expected = json!({
            "content_type": "text",
            "content": {"text": ""}
        });
        assert_eq!(serialized, expected);
    }

    #[test]
    fn deserialize_content() {
        let deserialized = serde_json::from_value::<TypeSafePayload>(json!({
            "content_type": "text",
            "content": {"text": ""}
        }))
        .unwrap();
        let expected = TypeSafePayload::Text(TextContent { text: String::new() });
        assert_eq!(deserialized, expected);
    }

    #[test]
    fn test_payload_conversion() {
        let payload = TypeSafePayload::FlowTrigger(FlowTrigger {
            flow_id: "some_flow-id".to_owned(),
            step_id: None,
        });
        let converted = Payload::try_from(payload).unwrap();
        let expected = Payload {
            content_type: "flow_trigger".to_owned(),
            content: Some(json!({"flow_id": "some_flow-id"})),
        };
        assert_eq!(converted, expected);
    }
}
