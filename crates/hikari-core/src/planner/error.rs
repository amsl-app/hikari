use hikari_db::sea_orm::DbErr;
use thiserror::Error;

use crate::openai::error::OpenAiError;

#[derive(Error, Debug)]
pub enum PlannerAssistantError {
    #[error(transparent)]
    Db(#[from] DbErr),
    #[error(transparent)]
    OpenAi(#[from] OpenAiError),
    #[error("invalid date from LLM: {0}")]
    InvalidDate(String),
}
