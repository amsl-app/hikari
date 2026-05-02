use hikari_model::{chat::ErrorResponse, llm::message::ConversationMessage};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(ToSchema, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum Response {
    Chat(ChatChunk), // Streaming of messages
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
}

impl ChatChunk {
    #[must_use]
    pub fn new(content: String, id: i32, step: String) -> Self {
        Self { content, id, step }
    }
}
