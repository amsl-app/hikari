use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct LlmConversationState {
    pub status: LlmStepStatus,
    pub current_step: String,
    pub value: StateValue,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct StateValue {
    #[serde(default)]
    pub response: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LlmStepStatus {
    #[default]
    NotStarted,
    Running,
    WaitingForInput,
    Completed,
    Error,
}
