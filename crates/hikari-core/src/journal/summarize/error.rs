use crate::openai::error::{FunctionCallError, OpenAiError};
use hikari_utils::date::error::DateError;
use sea_orm::DbErr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SummarizeError {
    #[error(transparent)]
    DbError(#[from] DbErr),

    #[error(transparent)]
    OpenAi(#[from] OpenAiError),

    #[error("Empty response")]
    EmptyResponse,

    #[error("Function call error")]
    FunctionCall(#[from] FunctionCallError),

    #[error("Other error during summarization: {0}")]
    Other(String),

    #[error("Operation timed out")]
    Timeout,

    #[error(transparent)]
    Date(#[from] DateError),
}
