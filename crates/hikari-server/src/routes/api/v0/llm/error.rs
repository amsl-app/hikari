use axum::{
    extract::ws::CloseFrame,
    response::{IntoResponse, Response},
};
use hikari_llm::execution::error::LlmExecutionError;
use hikari_model::chat::ErrorResponse;
use thiserror::Error;
use tungstenite::protocol::frame::coding::CloseCode;

use crate::routes::api::v0::modules::error::ModuleError;

#[derive(Error, Debug)]
pub(crate) enum LlmError {
    #[error("Invalid Request: {0}")]
    RequestError(String),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
    #[error("Failed to send response")]
    SendError(axum::Error),
    #[error("Failed to receive message")]
    ReceiveError(axum::Error),
    #[error(transparent)]
    ModuleError(#[from] ModuleError),
    #[error("Module not found: {0}")]
    ModuleNotFound(String),
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    #[error("Agent not specified")]
    AgentUnspecified,
    #[error("No agent found")]
    NoAgent,
    #[error(transparent)]
    LlmExecutionError(#[from] LlmExecutionError),
    #[error(transparent)]
    DbError(#[from] sea_orm::error::DbErr),
    #[error(transparent)]
    LoaderError(#[from] hikari_utils::loader::error::LoadingError),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ModuleDataError(#[from] crate::data::modules::error::ModuleError),
}

impl LlmError {
    pub fn into_close_frame(self) -> CloseFrame {
        let code = match self {
            Self::RequestError(_) => CloseCode::Protocol.into(),
            _ => CloseCode::Error.into(),
        };
        CloseFrame {
            code,
            reason: self.to_string().into(),
        }
    }

    pub fn as_response(&self) -> ErrorResponse {
        match self {
            LlmError::RequestError(msg) => ErrorResponse {
                error: msg.to_owned(),
                status_code: 400,
            },
            LlmError::ModuleError(e) => ErrorResponse {
                error: e.to_string(),
                status_code: 400,
            },
            LlmError::NoAgent => ErrorResponse {
                error: self.to_string(),
                status_code: 502,
            },
            other => ErrorResponse {
                error: other.to_string(),
                status_code: 500,
            },
        }
    }
}

impl IntoResponse for LlmError {
    fn into_response(self) -> Response {
        match self {
            LlmError::RequestError(_) | LlmError::ModuleError(_) => {
                http::status::StatusCode::BAD_REQUEST.into_response()
            }
            _ => http::status::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
