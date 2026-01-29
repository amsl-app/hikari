use axum::response::{IntoResponse, Response};

use crate::db;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("assessment id wasn't found")]
    AssessmentConfigNotFound,
    #[error("question id \"{0}\" does not exist")]
    QuestionIdDoesNotExist(String),
    #[error("Invalid configuration: scale type \"{0}\" not supported")]
    InvalidScaleType(String),
    #[error("assessment session was not completed")]
    NotCompleted,
    #[error("answer id wasn't found")]
    AnswerNotFound,
    #[error("assessment session isn't running")]
    NotRunning,
    #[error("an invalid answer was submitted")]
    InvalidAnswer,
    #[error("answer {0} was not found")]
    MissingAnswer(String),
    #[error("session id doesn't belong to assessment id")]
    UnrelatedSessionId,
    #[error("couldn't parse stored answer value {0}")]
    InvalidValue(String),
    #[error(transparent)]
    DB(db::error::DbError),
    #[error("Data could not be found")]
    NotFound,
    #[error("{0}")]
    Other(String),
}

impl From<db::error::DbError> for Error {
    fn from(error: db::error::DbError) -> Self {
        match error {
            db::error::DbError::QueryError(diesel::result::Error::NotFound) => Self::NotFound,
            _ => Self::DB(error),
        }
    }
}

impl From<sea_orm::DbErr> for Error {
    fn from(error: sea_orm::DbErr) -> Self {
        Self::DB(error.into())
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::AnswerNotFound
            | Self::UnrelatedSessionId
            | Self::AssessmentConfigNotFound
            | Self::NotCompleted
            | Self::NotFound => http::StatusCode::NOT_FOUND.into_response(),
            Self::NotRunning => http::StatusCode::CONFLICT.into_response(),
            Self::InvalidAnswer | Self::MissingAnswer(_) => http::StatusCode::BAD_REQUEST.into_response(),
            _ => http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
