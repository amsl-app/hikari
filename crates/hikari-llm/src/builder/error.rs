use thiserror::Error;

#[derive(Error, Debug)]
pub enum LlmBuildingError {
    #[error("Missing Prompt prefix: {0}")]
    MissingPrefix(String),
    #[error("Expected String: {0}")]
    ExpectedString(String),
    #[error("Expected Float: {0}")]
    ExpectedFloat(String),
    #[error(transparent)]
    ParseFloatError(#[from] std::num::ParseFloatError),
    #[error(transparent)]
    OpenAI(#[from] async_openai::error::OpenAIError),
    #[error("Missed formatation: {0}")]
    MissedFormatation(String),
}
