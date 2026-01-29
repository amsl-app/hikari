use crate::routes::error::{ErrorData, ErrorDataProvider, GetStatusCode};
use axum::response::{IntoResponse, Response};
use hikari_core::openai::error::{FunctionCallError, OpenAiError};
use http::status::InvalidStatusCode;
use sea_orm::DbErr;
use serde_derive::Serialize;
use std::error::Error;
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Error, Debug)]
pub(crate) enum AssistantError {
    #[error(transparent)]
    OpenAi(#[from] OpenAiError),

    #[error(transparent)]
    FunctionCall(#[from] FunctionCallError),

    #[error("Error loading data from db")]
    DbError(#[from] DbErr),

    #[error(transparent)]
    Request(#[from] reqwest::Error),

    #[error(transparent)]
    RequestMiddleware(#[from] reqwest_middleware::Error),

    #[error("Other error")]
    Other,

    #[error(transparent)]
    InvalidStatusCode(#[from] InvalidStatusCode),
}

impl ErrorDataProvider<AssistantErrorType> for FunctionCallError {
    fn error_data(self) -> Option<ErrorData<AssistantErrorType>> {
        Some(ErrorData::new(
            AssistantErrorType::Response,
            "the response from OpenAI was invalid",
        ))
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AssistantErrorType {
    OpenAi,
    Response,
    Database,
    Other,
}

impl GetStatusCode for AssistantErrorType {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::INTERNAL_SERVER_ERROR
    }
}

impl ErrorDataProvider<AssistantErrorType> for AssistantError {
    fn error_data(self) -> Option<ErrorData<AssistantErrorType>> {
        tracing::error!(error = &self as &dyn Error, "assistant error");
        let error_data = match self {
            Self::OpenAi(error) => {
                tracing::error!(error = &error as &dyn Error, "error communicating with OpenAI");
                ErrorData::new(AssistantErrorType::OpenAi, "error communicating with OpenAI")
            }
            Self::FunctionCall(fc) => fc.error_data()?,
            Self::DbError(error) => {
                tracing::error!(error = &error as &dyn Error, "error communicating with database");
                ErrorData::new(AssistantErrorType::Database, "error communicating with database")
            }
            Self::Request(error) | Self::RequestMiddleware(reqwest_middleware::Error::Reqwest(error)) => {
                tracing::error!(error = &error as &dyn Error, "worker request failed");
                ErrorData::new(AssistantErrorType::Other, "worker request failed")
            }
            Self::RequestMiddleware(reqwest_middleware::Error::Middleware(error)) => {
                tracing::error!(error = %error, "worker request failed");
                ErrorData::new(AssistantErrorType::Other, "worker request failed")
            }
            Self::Other | Self::InvalidStatusCode(_) => ErrorData::new(AssistantErrorType::Other, "other error"),
        };
        Some(error_data)
    }
}

impl IntoResponse for AssistantError {
    fn into_response(self) -> Response {
        http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}
