use hikari_utils::loader::error::LoadingError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PgVectorError {
    #[error("Config not provided")]
    ConfigNotProvided,
    #[error("Embedder not provided")]
    EmbedderNotProvided,
    #[error(transparent)]
    DBError(#[from] sea_orm::error::DbErr),
    #[error(transparent)]
    Api(#[from] async_openai::error::OpenAIError),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error("The vector length does not match the expected length")]
    VectorMissMatch,
    #[error(transparent)]
    OutputError(#[from] pdf_extract::OutputError),
    #[error(transparent)]
    LoadingError(#[from] LoadingError),
    #[error("The operation timed out")]
    Timeout,
}
