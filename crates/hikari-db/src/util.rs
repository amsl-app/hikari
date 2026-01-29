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
