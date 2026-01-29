use axum::Json;
use axum::response::{IntoResponse, Response};
use sea_orm::DbErr;
use std::borrow::Cow;

use serde_derive::Serialize;
use serde_json::{Map, Value};
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Error, Debug)]
pub(crate) enum LoginError {
    #[error("Invalid token data")]
    InvalidTokenData(#[from] std::str::Utf8Error),

    #[error("Token ")]
    Invalid,

    #[error("Database Error")]
    DatabaseError(#[from] DbErr),

    #[error("Error building response")]
    ResponseError,

    #[error(transparent)]
    Auth(#[from] crate::auth::AuthError),
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LoginErrorType {
    InvalidCredentials,
}

pub(crate) trait GetStatusCode {
    fn status_code(&self) -> http::StatusCode;
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ErrorData<T> {
    pub(crate) error: T,
    pub(crate) error_description: Cow<'static, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) data: Option<Map<String, Value>>,
}

impl<T> ErrorData<T> {
    pub fn new<A: Into<Cow<'static, str>>>(error: T, error_description: A) -> Self {
        Self {
            error,
            error_description: error_description.into(),
            data: None,
        }
    }
}

pub(crate) trait ErrorDataProvider<T: GetStatusCode> {
    fn error_data(self) -> Option<ErrorData<T>>;
}

impl ErrorDataProvider<LoginErrorType> for LoginError {
    fn error_data(self) -> Option<ErrorData<LoginErrorType>> {
        use LoginError::{Auth, DatabaseError, Invalid, InvalidTokenData, ResponseError};
        let res = match self {
            InvalidTokenData(_) | Auth(_) | Invalid => {
                ErrorData::new(LoginErrorType::InvalidCredentials, "invalid token data")
            }
            DatabaseError(_) | ResponseError => return None, // LoginError::NoSessionFound => return None,
        };
        Some(res)
    }
}

impl IntoResponse for LoginError {
    fn into_response(self) -> Response {
        error_to_axum_response(self)
    }
}

impl GetStatusCode for LoginError {
    fn status_code(&self) -> http::StatusCode {
        match self {
            Self::DatabaseError(_) => http::StatusCode::SERVICE_UNAVAILABLE,
            _ => http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl GetStatusCode for LoginErrorType {
    fn status_code(&self) -> http::StatusCode {
        match self {
            Self::InvalidCredentials => http::StatusCode::UNAUTHORIZED,
        }
    }
}

pub(crate) fn error_to_axum_response<E, T>(error: T) -> Response
where
    E: GetStatusCode + serde::Serialize,
    T: GetStatusCode + ErrorDataProvider<E>,
{
    let status_code = GetStatusCode::status_code(&error);
    let error_data = error.error_data();
    match error_data {
        Some(data) => {
            let status_code = GetStatusCode::status_code(&data.error);
            let json = Json(data);
            (status_code, json).into_response()
        }
        None => status_code.into_response(),
    }
}
