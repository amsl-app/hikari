use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValuesError {
    #[error(transparent)]
    PathParse(#[from] serde_json_path::ParseError),
    #[error(transparent)]
    JsonParse(#[from] serde_json::Error),
    #[error(transparent)]
    YamlParse(#[from] serde_yml::Error),
}
