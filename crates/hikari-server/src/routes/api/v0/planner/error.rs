use axum::response::{IntoResponse, Response};
use http::StatusCode;
use sea_orm::DbErr;
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum PlannerError {
    #[error(transparent)]
    SeaOrmError(#[from] DbErr),

    #[error("Planner entry could not be found")]
    NotFound,

    #[error("LLM error")]
    LlmError,

    #[error("Validation error: {0}")]
    ValidationError(String),
}

impl IntoResponse for PlannerError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound | Self::SeaOrmError(DbErr::RecordNotFound(_)) => StatusCode::NOT_FOUND.into_response(),
            Self::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
