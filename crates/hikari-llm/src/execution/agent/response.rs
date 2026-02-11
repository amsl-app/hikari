use hikari_model::{
    chat::{ErrorResponse, TypeSafePayload},
    llm::message::ConversationMessage,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(ToSchema, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum Response {
    Chat(ChatChunk),          // Streaming of messages
    Payload(TypeSafePayload), // Non streaming payloads
    History(Vec<ConversationMessage>),
    ConversationEnd,
    Typing,
    Hold,
    Error(ErrorResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChatChunk {
    pub content: String,
    pub id: i32,
    pub step: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub audio: Vec<u8>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub audio_end: bool,
}

impl ChatChunk {
    #[must_use]
    pub fn new(content: String, audio: Vec<u8>, audio_end: bool, id: i32, step: String) -> Self {
        Self {
            content,
            audio,
            audio_end,
            id,
            step,
        }
    }
}
