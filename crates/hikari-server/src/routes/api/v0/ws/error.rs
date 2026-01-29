use axum::extract::ws::{CloseFrame, Message};
use thiserror::Error;
use tungstenite::protocol::frame::coding::CloseCode;

#[derive(Debug, Error)]
pub enum WsError {
    #[error(transparent)]
    ReceiveError(#[from] axum::Error),
    #[error("Invalid Request: {0}")]
    RequestError(String),
    // #[error(transparent)]
    // ModuleError(data::modules::Error),
    #[error("Json Error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Database Error")]
    Database(#[from] sea_orm::error::DbErr),
    #[error("Csml Error")]
    CsmlError(#[from] csml_engine::data::EngineError),
    #[error(transparent)]
    Send(#[from] tokio::sync::mpsc::error::SendError<Message>),
}

impl WsError {
    pub fn into_close_frame(self) -> CloseFrame {
        let code = match self {
            Self::RequestError(_) => CloseCode::Protocol.into(),
            // Self::ModuleError(_) => CloseCode::Invalid.into(),
            Self::ReceiveError(_) | Self::Json(_) | Self::Database(_) | Self::CsmlError(_) | Self::Send(_) => {
                CloseCode::Error.into()
            }
        };
        CloseFrame {
            code,
            reason: self.to_string().into(),
        }
    }
}
