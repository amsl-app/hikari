use std::error::Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TTSError {
    #[error(transparent)]
    Elevenlabs(#[from] elevenlabs_rs::error::Error),
    #[error(transparent)]
    DBError(#[from] sea_orm::error::DbErr),
    #[error(transparent)]
    CacheLoadingError(#[from] hikari_utils::loader::error::LoadingError),
    #[error(transparent)]
    InvalidPath(#[from] url::ParseError),
    #[error("Undefined Elevenlabs error: {0}")]
    Undefined(#[from] Box<dyn Error + Send + Sync>),
}

#[derive(Error, Debug)]
pub enum CombinedError {
    #[error(transparent)]
    TTS(#[from] TTSError),
    #[error(transparent)]
    OpenAIStream(#[from] crate::openai::error::StreamingError),
    #[error(transparent)]
    TokioJoin(#[from] tokio::task::JoinError),
}
