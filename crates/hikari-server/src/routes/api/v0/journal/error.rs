use crate::db::error::DbError;
use axum::response::{IntoResponse, Response};
use hikari_db::sea_orm::DbErr;
use http::StatusCode;
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum JournalError {
    #[error("Database error.")]
    DBError(#[from] DbError),

    #[error("Database error.")]
    SeaOrmError(#[from] DbErr),

    #[error("Error creating response json")]
    Serde(#[from] serde_json::Error),

    #[error("Invalid UUID data")]
    Uuid(#[from] uuid::Error),

    #[error("Journal entry/content could not be found")]
    NotFound,

    #[error("Field {0} data too large")]
    TooLarge(String),
}

impl IntoResponse for JournalError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound | Self::SeaOrmError(DbErr::RecordNotFound(_)) => StatusCode::NOT_FOUND.into_response(),
            Self::TooLarge(_) => StatusCode::BAD_REQUEST.into_response(),
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
