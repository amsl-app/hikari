use crate::data;
use crate::data::modules;
use crate::db::error::DbError;
use crate::routes::api::v0::{assessment, bots};
use axum::response::{IntoResponse, Response};
use csml_engine::data::EngineError;
use sea_orm::DbErr;
use std::str::Utf8Error;
use thiserror::Error;

// TODO (LOW) Document error types
// TODO (LOW) Document error types
#[derive(Error, Debug)]
pub(crate) enum ModuleError {
    #[error(transparent)]
    DBError(#[from] DbError),

    #[error(transparent)]
    SeaOrmError(#[from] DbErr),

    #[error(transparent)]
    DataError(#[from] modules::error::ModuleError),

    #[error("Configuration Error: {0}")]
    ConfigurationError(String),

    #[error("Assessment was not configured for this module")]
    AssessmentNotConfigured,

    #[error("Failed to serialize result")]
    SerdeError(#[from] serde_json::Error),

    #[error("Csml Engine Error")]
    CsmlEngine(#[from] EngineError),

    #[error(transparent)]
    AssessmentError(#[from] assessment::error::Error),

    #[error("Error deserializing uuid")]
    Uuid(#[from] uuid::Error),

    #[error("Source not found: {0}")]
    SourceNotFound(String),

    #[error(transparent)]
    LoadingError(#[from] hikari_utils::loader::error::LoadingError),
}

#[derive(Error, Debug)]
pub(crate) enum UserError {
    #[error("Database error.")]
    DBError(#[from] DbError),

    #[error("Database error.")]
    SeaOrmError(#[from] DbErr),

    #[error("Error creating response json")]
    Serde(#[from] serde_json::Error),

    #[error("Invalid key")]
    InvalidKey,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Key/Path not found")]
    NotFound,

    #[error("No groups to select")]
    NoGroupsToSelect,
}

#[derive(Error, Debug)]
pub(crate) enum MessagingError {
    #[error(transparent)]
    Message(#[from] bots::error::MessageError),

    #[error(transparent)]
    Module(#[from] ModuleError),

    #[error("Csml Engine Error")]
    Csml(#[from] EngineError),

    #[error(transparent)]
    DBError(#[from] DbError),

    #[error(transparent)]
    SeaOrm(#[from] DbErr),

    #[error("Error creating response json")]
    Serde(#[from] serde_json::Error),

    #[error("Data is not Utf-8")]
    Utf8(#[from] Utf8Error),

    #[error("Error parsing data")]
    Strum(#[from] strum::ParseError),

    #[error("DB Error")]
    Diesel(#[from] diesel::result::Error),

    #[error("Session was not started or already finished")]
    NotRunning,

    #[error("Session was already started")]
    AlreadyStarted,

    #[error("Uuid could not be decoded")]
    Uuid(#[from] uuid::Error),

    #[error("Tried to issue an action on a session that has no bot: {0}")]
    NoBot(String),

    #[error("Bot Not Found: {bot_id}")]
    BotNotFound { bot_id: String },

    #[error(transparent)]
    ChronoParse(#[from] chrono::ParseError),

    #[error("Exclusivity error")]
    Exclusivity,
}

impl From<data::modules::error::ModuleError> for MessagingError {
    fn from(error: data::modules::error::ModuleError) -> Self {
        Self::Module(ModuleError::DataError(error))
    }
}

impl IntoResponse for MessagingError {
    fn into_response(self) -> Response {
        match self {
            Self::Message(e) => e.into_response(),
            Self::Module(e) => e.into_response(),
            Self::NoBot(_) => http::status::StatusCode::BAD_REQUEST.into_response(),
            Self::NotRunning | Self::Exclusivity | Self::AlreadyStarted => {
                http::status::StatusCode::CONFLICT.into_response()
            }
            _ => http::status::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

impl IntoResponse for ModuleError {
    fn into_response(self) -> Response {
        match self {
            ModuleError::DataError(_)
            | ModuleError::SourceNotFound(_)
            | Self::DBError(DbError::QueryError(diesel::result::Error::NotFound)) => {
                http::status::StatusCode::NOT_FOUND.into_response()
            }
            _ => http::status::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

impl IntoResponse for UserError {
    fn into_response(self) -> Response {
        match self {
            Self::InvalidKey => http::status::StatusCode::BAD_REQUEST.into_response(),
            Self::NotFound => http::status::StatusCode::NOT_FOUND.into_response(),
            Self::InvalidToken => http::status::StatusCode::NOT_ACCEPTABLE.into_response(),
            _ => http::status::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
