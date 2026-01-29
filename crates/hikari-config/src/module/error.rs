use thiserror::Error;

#[derive(Error, Debug)]
pub enum ModuleError {
    #[error("Content not found.")]
    ContentNotFound,

    #[error(transparent)]
    ParseError(#[from] serde_yml::Error),

    #[error("Source not found: {0}")]
    SourceNotFound(String),

    #[error("Bot not found.")]
    BotNotFound,

    #[error("Flow not found.")]
    FlowNotFound,

    #[error("Assessment not found.")]
    AssessmentNotFound,

    #[error("LLM agent not found.")]
    LlmAgentNotFound,

    #[error("The requested module was not found.")]
    ModuleNotFound,

    #[error("The requested session was not found.")]
    SessionNotFound,

    #[error("The specified module group was not found.")]
    ModuleGroupNotFound,
}

#[derive(Error, Debug)]
pub enum LlmServiceError {
    #[error("Unknown service: {0}")]
    UnknownService(String),
}
