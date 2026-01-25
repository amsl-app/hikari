use crate::db::error::DbError;
use axum::response::{IntoResponse, Response};
use http::StatusCode;
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum QuizError {
    #[error(transparent)]
    DBError(#[from] DbError),

    #[error(transparent)]
    SeaOrmError(#[from] sea_orm::DbErr),

    #[error("The requested quiz was not found.")]
    QuizNotFound,

    #[error("The requested question was not found.")]
    QuestionNotFound,

    #[error(transparent)]
    UuidError(#[from] uuid::Error),

    #[error("No session IDs provided for quiz creation.")]
    NoSessionIds,

    #[error("Module not found: {0}")]
    ModuleNotFound(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("No content provided for question generation.")]
    NoContentProvided,

    #[error(transparent)]
    QuizError(#[from] hikari_core::quiz::error::QuizError),

    #[error(transparent)]
    SerializeError(#[from] serde_json::Error),
}

impl IntoResponse for QuizError {
    fn into_response(self) -> Response {
        match self {
            QuizError::DBError(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {e}")).into_response()
            }
            QuizError::SeaOrmError(e) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {e}")).into_response()
            }
            QuizError::QuizNotFound => (StatusCode::NOT_FOUND, "Quiz not found").into_response(),
            QuizError::QuestionNotFound => (StatusCode::NOT_FOUND, "Question not found").into_response(),

            QuizError::UuidError(e) => (StatusCode::BAD_REQUEST, format!("Invalid UUID: {e}")).into_response(),
            QuizError::NoSessionIds => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "No session IDs provided for quiz creation",
            )
                .into_response(),
            QuizError::ModuleNotFound(module_id) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Module not found: {module_id}"),
            )
                .into_response(),
            QuizError::SessionNotFound(session_id) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Session not found: {session_id}"),
            )
                .into_response(),
            QuizError::NoContentProvided => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "No content provided for question generation",
            )
                .into_response(),
            QuizError::QuizError(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Quiz error: {e}")).into_response(),
            QuizError::SerializeError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to serialize response: {e}"),
            )
                .into_response(),
        }
    }
}
