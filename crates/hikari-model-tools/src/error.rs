use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("num conversion failed")]
    NumConversion,
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
}
