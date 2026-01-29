use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Failed to run migration on db: {0}")]
    MigrationFailed(String),

    #[error("Failed to connect to db")]
    ConnectionError(#[from] diesel::result::ConnectionError),

    #[error("Failed to load env")]
    EnvironmentVarError(#[from] std::env::VarError),

    #[error("Query error occurred")]
    QueryError(#[from] diesel::result::Error),

    #[error("DB error occurred")]
    SeaOrm(#[from] sea_orm::DbErr),

    #[error("Unknown database type {0}")]
    UnknownDbType(String),

    #[error("Error with DB connection pool")]
    PoolInit(#[from] r2d2::Error),

    #[error("Uuid could not be decoded")]
    Uuid(#[from] uuid::Error),
}
