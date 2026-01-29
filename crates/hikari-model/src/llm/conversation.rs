use uuid::Uuid;

pub struct LlmConversation {
    pub conversation_id: Uuid,
    pub status: ConversationStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversationStatus {
    Open,
    Completed,
    Closed,
}
