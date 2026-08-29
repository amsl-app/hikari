use base64::Engine;
use ring::rand::{self, SecureRandom};
use sea_orm::{DbErr, TransactionError};
use std::error::Error;

pub trait FlattenTransactionResultExt<T> {
    fn flatten_res(self) -> T;
}

pub trait InspectTransactionError<E> {
    #[must_use]
    fn inspect_transaction_err<F: FnOnce(&E)>(self, f: F) -> Self;
}

impl<T, E> FlattenTransactionResultExt<Result<T, E>> for Result<T, TransactionError<E>>
where
    E: From<DbErr> + Error,
{
    fn flatten_res(self) -> Result<T, E> {
        self.map_err(|err| match err {
            TransactionError::Connection(err) => err.into(),
            TransactionError::Transaction(err) => err,
        })
    }
}

impl<T, E: Error> InspectTransactionError<E> for Result<T, TransactionError<E>> {
    fn inspect_transaction_err<F: FnOnce(&E)>(self, f: F) -> Self {
        if let Err(TransactionError::Transaction(err)) = &self {
            f(err);
        }
        self
    }
}

pub trait RequireRecord<T> {
    fn require(self) -> Result<T, DbErr>;
}

impl<T> RequireRecord<T> for Result<Option<T>, DbErr> {
    fn require(self) -> Result<T, DbErr> {
        self?.ok_or_else(|| DbErr::RecordNotFound("record not found".to_string()))
    }
}

fn random_bytes_64() -> [u8; 64] {
    let rng = rand::SystemRandom::new();
    let mut bytes = [0u8; 64];
    rng.fill(&mut bytes).expect("Failed to generate random bytes");
    bytes
}

pub(crate) fn generate_token() -> String {
    base64::engine::general_purpose::STANDARD.encode(random_bytes_64())
}

pub(crate) fn generate_url_safe_token() -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes_64())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token() {
        let token = generate_token();
        let token = base64::engine::general_purpose::STANDARD
            .decode(&token)
            .expect("token should decode");
        assert_eq!(token.len(), 64);
        token
            .iter()
            .find(|&&b| b != 0)
            .expect("token is all zeros, this should never happen");
    }

    #[test]
    fn test_generate_url_safe_token() {
        let token = generate_url_safe_token();
        let token = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&token)
            .expect("token should decode");
        assert_eq!(token.len(), 64);
        token
            .iter()
            .find(|&&b| b != 0)
            .expect("token is all zeros, this should never happen");
    }
}
