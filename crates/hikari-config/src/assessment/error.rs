use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("invalid answer type: expected {expected_type}, got {actual_type}")]
    InvalidAnswerType { expected_type: String, actual_type: String },
    #[error("invalid answer value: {value}. min: {min}, max: {max}")]
    AnswerOutOfRange { min: u8, max: u8, value: u8 },
    #[error("invalid option: {value}")]
    InvalidOption { value: String },
}
