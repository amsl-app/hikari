use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Reqwest client error: {0}")]
    Client(#[from] reqwest::Error),

    #[error("Response failed with status: {}", reqwest::Response::status(.0))]
    StatusCode(Box<reqwest::Response>),

    #[error(transparent)]
    Http(#[from] http::Error),

    #[error("Operation timed out")]
    Timeout,
}
