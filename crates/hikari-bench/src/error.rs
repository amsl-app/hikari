use openidconnect::url;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Hikari(#[from] hikari::error::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),
}
