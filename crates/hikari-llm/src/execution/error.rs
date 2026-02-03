use crate::builder::{error::LlmBuildingError, slot::paths::SlotPath};
use hikari_core::tts::error::CombinedError;
use sea_orm::DbErr;
use std::str::ParseBoolError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmExecutionError {
    #[error("All Actions are executed")]
    Completed,
    #[error("No Action found, but there should be one")]
    NoAction,
    #[error("Module not found: {0}")]
    ModuleNotFound(String),
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("NextSession not found: {0}")]
    NextSessionNotFound(String),
    #[error(transparent)]
    DatabaseError(#[from] DbErr),
    #[error(transparent)]
    BoolParseError(#[from] ParseBoolError),
    #[error("Invalid state")]
    InvalidState,
    #[error(transparent)]
    ParsingError(#[from] hikari_model_tools::error::Error),
    #[error("Unexpected error: {0}")]
    Unexpected(String),
    #[error(transparent)]
    BuilderError(#[from] LlmBuildingError),
    #[error(transparent)]
    OpenAiError(#[from] hikari_core::openai::error::OpenAiError),
    #[error(transparent)]
    PgVector(#[from] hikari_core::pgvector::error::PgVectorError),
    #[error(transparent)]
    ApiError(#[from] APIExecutionError),
    #[error("Slot not found: {0}")]
    SlotNotFound(SlotPath),
    #[error(transparent)]
    ValuesError(#[from] hikari_utils::values::error::ValuesError),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error(transparent)]
    YamlError(#[from] serde_yml::Error),
    #[error(transparent)]
    CombinedError(#[from] CombinedError),
    #[error("Text-to-Speech not configured")]
    TextToSpeechNotConfigured,
    #[error("Unexpected response format")]
    UnexpectedResponseFormat,
    #[error(transparent)]
    Undefined(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, Error)]
pub enum APIExecutionError {
    #[error("Response path not found: {0}")]
    ResponsePathNotFound(String),
    #[error("Invalid response format: {0}")]
    InvalidResponseFormat(String),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error("ReqwestSseEventError: {0}")]
    ReqwestSseEventError(reqwest_sse::error::EventError),
    #[error("ReqwestSseEventSourceError: {0}")]
    ReqwestSseEventSourceError(reqwest_sse::error::EventSourceError),
}
