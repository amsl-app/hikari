use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::primitives::ByteStreamError;
use std::error::Error;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LoadingError {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    S3(Box<dyn Error + Send + Sync>),
    #[error(transparent)]
    ByteStream(#[from] ByteStreamError),
    #[error(transparent)]
    WalkDir(#[from] async_walkdir::Error),
    #[error("Undefined: {0}")]
    Undefined(String),
    #[error("Invalid credentials: {0}")]
    CredentialsError(String),
    #[error("Invalid URL: {0}")]
    InvalidURL(String),
    #[error("Invalid Path: {0}")]
    InvalidPath(PathBuf),
    #[error(transparent)]
    PDFError(#[from] pdf_extract::OutputError),
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),
    #[error("File already loaded")]
    FileAlreadyLoaded,
}

impl<E: 'static, R: 'static> From<SdkError<E, R>> for LoadingError
where
    SdkError<E, R>: Error + Send + Sync,
{
    fn from(error: SdkError<E, R>) -> Self {
        LoadingError::S3(Box::new(error))
    }
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error(transparent)]
    Yaml(#[from] serde_yml::Error),
    #[error(transparent)]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    Chrono(#[from] chrono::ParseError),
    #[error("{0}")]
    Other(String),
}

impl From<url::ParseError> for LoadingError {
    fn from(e: url::ParseError) -> Self {
        ParseError::Url(e).into()
    }
}

impl From<serde_yml::Error> for LoadingError {
    fn from(e: serde_yml::Error) -> Self {
        ParseError::Yaml(e).into()
    }
}

impl From<std::string::FromUtf8Error> for LoadingError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        ParseError::Utf8(e).into()
    }
}
