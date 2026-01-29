use axum::response::{IntoResponse, Response};
use http::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AssistantError {
    #[error(transparent)]
    Summary(#[from] hikari_core::journal::summarize::error::SummarizeError),
}

impl IntoResponse for AssistantError {
    fn into_response(self) -> Response {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}
