use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("WAV file is not PCM16")]
    NotPCM16,
    #[error(transparent)]
    Hound(#[from] hound::Error),
}
