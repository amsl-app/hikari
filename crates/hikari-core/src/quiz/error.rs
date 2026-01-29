use sea_orm::DbErr;
use thiserror::Error;

use crate::{openai::error::OpenAiError, pgvector::error::PgVectorError};

#[derive(Debug, Error)]
pub enum QuizError {
    #[error("OpenAI error: {0}")]
    OpenAi(#[from] OpenAiError),

    #[error("Database error: {0}")]
    Database(#[from] DbErr),

    #[error("Vector search error: {0}")]
    Vector(#[from] PgVectorError),

    #[error(transparent)]
    ParseError(#[from] serde_json::Error),

    #[error("Unexpected error: {0}")]
    Other(String),

    #[error("Unexpected response format from LLM")]
    UnexpectedResponseFormat,
}
