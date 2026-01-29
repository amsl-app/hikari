use axum::response::{IntoResponse, Response};
use hikari_db::sea_orm::DbErr;
use http::StatusCode;
use std::num::ParseIntError;
use thiserror::Error;
// use crate::routes::api::v0::journal::assistant::error::AssistantError;
// use crate::routes::api::v0::modules::error::UserError;

#[derive(Error, Debug)]
pub(crate) enum EndpointError {
    #[error("DB error")]
    SeaOrm(#[from] DbErr),

    #[error("Error (de)serializing JSON")]
    Json(#[from] serde_json::Error),

    #[error("The given number could be parsed")]
    Parse(#[from] ParseIntError),

    #[error("Other error: {0}")]
    Other(String),

    #[error("Invalid endpoint config: {0}")]
    Config(String),

    #[error(transparent)]
    Conversion(#[from] hikari_model_tools::error::Error),
}

impl IntoResponse for EndpointError {
    fn into_response(self) -> Response {
        let status_code = match self {
            Self::Parse(_) => StatusCode::BAD_REQUEST,
            // EndpointError::UserError(e) => e.status_code(),
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        status_code.into_response()
    }
    // fn status_code(&self) -> StatusCode {
    //     match self {
    //         EndpointError::Parse(_) => StatusCode::BAD_REQUEST,
    //         EndpointError::UserError(e) => e.status_code(),
    //         _ => StatusCode::INTERNAL_SERVER_ERROR,
    //     }
    // }
}
