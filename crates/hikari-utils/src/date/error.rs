use thiserror::Error;

#[derive(Debug, Error)]
pub enum DateError {
    #[error(transparent)]
    Parse(#[from] chrono::ParseError),

    #[error("Time is not valid")]
    InvalidLocalTime,

    #[error("Time out of range")]
    Overflow,
}
