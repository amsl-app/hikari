pub(crate) mod error;
#[cfg(feature = "postgres")]
pub(crate) mod postgres;
pub(crate) mod sea_orm;
#[cfg(feature = "sqlite")]
pub(crate) mod sqlite;

use diesel::backend::Backend;

use diesel::Connection;
#[cfg(feature = "postgres")]
use diesel::PgConnection;
#[cfg(feature = "sqlite")]
use diesel::SqliteConnection;
use diesel::migration::{MigrationSource, MigrationVersion};

use diesel_migrations::MigrationHarness;

use csml_engine::make_migrations_with_conn;
use url::Url;

use crate::db::error::DbError;

pub(crate) fn run_migrations<DB: Backend, C: MigrationHarness<DB>, S: MigrationSource<DB>>(
    conn: &mut C,
    source: S,
) -> Result<Vec<MigrationVersion<'static>>, DbError> {
    tracing::debug!("running migrations");
    let res = conn.run_pending_migrations(source);

    match res {
        Ok(versions) => Ok(versions.into_iter().map(|mv| mv.as_owned()).collect()),
        Err(err) => {
            tracing::error!(errro = ?err, "failed to migrate db");
            Err(DbError::MigrationFailed(err.to_string()))
        }
    }
}

pub(crate) async fn migration(url: &Url) -> Result<Vec<MigrationVersion<'static>>, DbError> {
    let Some(db_type) = url.scheme().split('+').next() else {
        return Err(DbError::UnknownDbType("NO_TYPE".to_string()));
    };

    match db_type {
        #[cfg(feature = "sqlite")]
        "sqlite" => {
            let mut conn = SqliteConnection::establish(url.as_ref())?;
            make_migrations_with_conn(&mut (&mut conn).into())
                .map_err(|err| DbError::MigrationFailed(err.to_string()))?;
            run_migrations(&mut conn, sqlite::MIGRATIONS)
        }
        #[cfg(feature = "postgres")]
        "postgresql" => {
            let mut conn = PgConnection::establish(url.as_ref())?;
            make_migrations_with_conn(&mut (&mut conn).into())
                .map_err(|err| DbError::MigrationFailed(err.to_string()))?;
            run_migrations(&mut conn, postgres::MIGRATIONS)
        }
        _ => Err(DbError::UnknownDbType(db_type.to_string())),
    }
}
