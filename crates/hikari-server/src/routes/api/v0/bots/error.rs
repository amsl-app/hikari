use crate::db::error::DbError;
use axum::response::{IntoResponse, Response};
use csml_engine::data::EngineError;
use hikari_db::sea_orm::DbErr;
use sea_orm::TransactionError;
use std::num::ParseIntError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MessageError {
    #[error(transparent)]
    Flow(#[from] BotError),

    #[error(transparent)]
    CSMLEngine(#[from] EngineError),

    #[error("Bot could not be serialized")]
    Serialization(#[from] serde_json::Error),

    #[error(transparent)]
    Database(#[from] DbError),

    #[error(transparent)]
    SeaOrm(#[from] DbErr),

    #[error("The given number couldn't be parsed")]
    NotANumber(#[from] ParseIntError),

    #[error("Uuid could not be parsed")]
    Uuid(#[from] uuid::Error),

    #[error(transparent)]
    Conversion(#[from] hikari_model_tools::error::Error),
}
impl<E: Into<Self> + std::error::Error> From<TransactionError<E>> for MessageError {
    fn from(value: TransactionError<E>) -> Self {
        match value {
            TransactionError::Connection(err) => Self::SeaOrm(err),
            TransactionError::Transaction(err) => err.into(),
        }
    }
}

impl IntoResponse for MessageError {
    fn into_response(self) -> Response {
        match self {
            // MessageError::Bot(bot_err) => bot_err.into_response(),
            Self::Flow(flow_err) => flow_err.into_response(),
            _ => http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Error, Debug)]
pub enum BotError {
    #[error("Bot not found")]
    BotNotFound,

    #[error("Flow could not be serialized")]
    Serialization(#[from] serde_json::Error),
    // TODO (LOW) Add error for invalid flow
}

impl IntoResponse for BotError {
    fn into_response(self) -> Response {
        match self {
            Self::BotNotFound => http::StatusCode::NOT_FOUND.into_response(),
            Self::Serialization(_) => http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Error, Debug)]
pub enum ConversationError {
    #[error(transparent)]
    CSMLEngine(#[from] EngineError),
}

impl IntoResponse for ConversationError {
    fn into_response(self) -> Response {
        http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}
