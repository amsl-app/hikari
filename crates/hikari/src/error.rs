use openidconnect::http;
use openidconnect::http::StatusCode;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Http(#[from] HttpError),

    #[error("Received invalid json data")]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    Internal(#[from] InternalError),

    #[error("Odic error: {0}")]
    Oidc(String),
}

#[derive(Error, Debug)]
pub enum InternalError {
    #[error(transparent)]
    InvalidUrl(#[from] url::ParseError),

    #[error(transparent)]
    InvalidHeaderValue(#[from] http::header::InvalidHeaderValue),
}

#[derive(Error, Debug)]
pub enum HttpError {
    #[error(transparent)]
    Hikari(#[from] hikari_http::Error),

    #[error("Request failed ({1}): {0}")]
    Status(String, StatusCode),

    #[error(transparent)]
    UrlToUriError(#[from] uri_url::UrlToUriError),

    #[error(transparent)]
    Http(#[from] http::Error),
}
