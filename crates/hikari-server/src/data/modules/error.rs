use thiserror::Error;

#[derive(Error, Debug)]
pub enum ModuleError {
    #[error("The requested module was not found.")]
    ModuleNotFound,

    #[error("The requested session was not found.")]
    SessionNotFound,
}
