use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::chat::{Direction, Message, TextContent, TypeSafePayload};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, ToSchema)]
pub enum MessageStatus {
    #[default]
    Generating,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ConversationMessage {
    #[serde(skip_serializing)]
    pub conversation_id: Uuid,
    pub message_order: i32,
    pub message: TypeSafePayload,
    #[serde(skip_serializing)]
    pub step: String,
    pub direction: Direction,
    pub status: MessageStatus,
}

impl ConversationMessage {
    #[must_use]
    pub fn new(
        conversation_id: Uuid,
        message_order: i32,
        message: TypeSafePayload,
        step: String,
        direction: Direction,
        status: MessageStatus,
    ) -> Self {
        ConversationMessage {
            conversation_id,
            message_order,
            message,
            step,
            direction,
            status,
        }
    }

    #[must_use]
    pub fn new_text(
        conversation_id: Uuid,
        message_order: i32,
        text: String,
        step: String,
        direction: Direction,
    ) -> Self {
        ConversationMessage {
            conversation_id,
            message_order,
            message: TypeSafePayload::Text(TextContent { text }),
            step,
            direction,
            status: MessageStatus::default(),
        }
    }
}

impl From<ConversationMessage> for Message<TypeSafePayload> {
    fn from(message: ConversationMessage) -> Self {
        let ConversationMessage { message, direction, .. } = message;
        Message {
            direction,
            payload: message,
        }
    }
}
