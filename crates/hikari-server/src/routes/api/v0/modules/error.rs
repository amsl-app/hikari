use crate::data::modules;
use crate::db::error::DbError;
use crate::routes::api::v0::{assessment, bots};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use csml_engine::data::EngineError;
use sea_orm::DbErr;
use std::str::Utf8Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum ModuleDbError {
    #[error(transparent)]
    Db(#[from] DbError),

    #[error(transparent)]
    SeaOrm(#[from] DbErr),
}

impl From<DbError> for ModuleError {
    fn from(error: DbError) -> Self {
        Self::Db(ModuleDbError::from(error))
    }
}

impl From<DbErr> for ModuleError {
    fn from(error: DbErr) -> Self {
        Self::Db(ModuleDbError::from(error))
    }
}

impl From<DbError> for UserError {
    fn from(error: DbError) -> Self {
        Self::Db(ModuleDbError::from(error))
    }
}

impl From<DbErr> for UserError {
    fn from(error: DbErr) -> Self {
        Self::Db(ModuleDbError::from(error))
    }
}

impl From<DbError> for MessagingError {
    fn from(error: DbError) -> Self {
        Self::Db(ModuleDbError::from(error))
    }
}

impl From<DbErr> for MessagingError {
    fn from(error: DbErr) -> Self {
        Self::Db(ModuleDbError::from(error))
    }
}

// TODO (LOW) Document error types
#[derive(Error, Debug)]
pub(crate) enum ModuleError {
    #[error(transparent)]
    Db(#[from] ModuleDbError),

    #[error(transparent)]
    DataError(#[from] modules::error::ModuleError),

    #[error("Configuration Error: {0}")]
    ConfigurationError(String),

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
    #[error(transparent)]
    Db(#[from] ModuleDbError),

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
    Db(#[from] ModuleDbError),

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

impl From<modules::error::ModuleError> for MessagingError {
    fn from(error: modules::error::ModuleError) -> Self {
        Self::Module(ModuleError::DataError(error))
    }
}

impl IntoResponse for MessagingError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Message(e) => return e.into_response(),
            Self::Module(e) => module_error_status(&e),
            Self::NoBot(_) => StatusCode::BAD_REQUEST,
            Self::NotRunning | Self::Exclusivity | Self::AlreadyStarted => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        status.into_response()
    }
}

impl IntoResponse for ModuleError {
    fn into_response(self) -> Response {
        module_error_status(&self).into_response()
    }
}

impl IntoResponse for UserError {
    fn into_response(self) -> Response {
        user_error_status(&self).into_response()
    }
}

fn module_error_status(error: &ModuleError) -> StatusCode {
    match error {
        ModuleError::DataError(_)
        | ModuleError::SourceNotFound(_)
        | ModuleError::Db(ModuleDbError::Db(DbError::QueryError(diesel::result::Error::NotFound))) => {
            StatusCode::NOT_FOUND
        }
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn user_error_status(error: &UserError) -> StatusCode {
    match error {
        UserError::InvalidKey => StatusCode::BAD_REQUEST,
        UserError::NotFound => StatusCode::NOT_FOUND,
        UserError::InvalidToken => StatusCode::NOT_ACCEPTABLE,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
