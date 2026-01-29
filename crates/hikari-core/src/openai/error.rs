use futures_retry_policies::ShouldRetry;
use std::error::Error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OpenAiError {
    #[error(transparent)]
    Api(#[from] async_openai::error::OpenAIError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    FunctionCall(#[from] FunctionCallError),

    #[error("No response from OpenAi")]
    EmptyResponse,

    #[error("Operation timed out")]
    Timeout,

    #[error("Could not run tool: {0}")]
    ToolError(String),

    #[error(transparent)]
    HttpClientBuild(#[from] reqwest::Error),
}

#[derive(Error, Debug)]
pub enum FunctionCallError {
    #[error("OpenAi returned the wrong function")]
    WrongFunction,

    #[error("Syntax returned by OpenAi is invalid")]
    InvalidSyntax,

    #[error("No function call in OpenAi response even though one was expected")]
    Missing,
}

impl ShouldRetry for OpenAiError {
    fn should_retry(&self, _: u32) -> bool {
        true
    }
}

// #[derive(Error, Debug)]
// pub enum StreamingError {
//     #[error(transparent)]
//     OpenAI(#[from] OpenAiError),
//     #[error("Unexpected: {0}")]
//     Unexpected(String),
// }

pub type StreamingError = Box<dyn Error + Send + Sync>;
