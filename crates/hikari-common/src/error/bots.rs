use thiserror::Error;

#[derive(Error, Debug)]
pub enum BotError {
    // Not transparent so we don't leak server information
    #[error("Bot could not be loaded")]
    IO(#[from] std::io::Error),

    #[error("Bot could not be found")]
    NotFound,

    #[error("Bot could not be serialized")]
    Serialization(#[from] serde_json::Error),

    #[error("Error Converting Filename")]
    FileNameConversion,

    #[error("Bot can't be empty")]
    Empty,

    #[error("Error in CSML: {0}")]
    Csml(String),
    // TODO (LOW) Add error for invalid bot
}

impl From<csml_interpreter::error_format::ErrorInfo> for BotError {
    fn from(e: csml_interpreter::error_format::ErrorInfo) -> Self {
        Self::Csml(e.message)
    }
}
