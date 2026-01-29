use super::MIGRATIONS;

use crate::db::error::DbError;
use diesel::migration::MigrationVersion;
use diesel::prelude::*;
use diesel::sql_query;
use diesel_migrations::MigrationHarness;

use hikari_test_helpers::{SqliteDb, TestDb};

use crate::db;
use test_log::test;

pub(crate) fn revert_all_migrations(conn: &'_ mut SqliteConnection) -> Result<Vec<MigrationVersion<'_>>, DbError> {
    tracing::debug!("migrating db for user tables");
    conn.revert_all_migrations(MIGRATIONS)
        .map_err(|err| DbError::MigrationFailed(format! {"{err}"}))
}

fn setup_db(run_migrations: bool) -> SqliteDb {
    let sqlite_db = SqliteDb::new().unwrap();
    let mut conn = SqliteConnection::establish(&sqlite_db.db_uri()).unwrap();
    if run_migrations {
        db::run_migrations(&mut conn, MIGRATIONS).unwrap();
    }
    sqlite_db
}

#[test]
fn test_migrations() {
    let db = setup_db(false);
    let db_uri = db.db_uri();
    let db_uri = db_uri.as_ref();
    let mut conn = SqliteConnection::establish(db_uri).unwrap();
    // Do it twice to make sure the migrations are idempotent
    for _ in 0..2 {
        let migrations = db::run_migrations(&mut conn, MIGRATIONS).unwrap();

        let mut conn = SqliteConnection::establish(db_uri).unwrap();
        let query = sql_query("SELECT id FROM users WHERE subject = 'test'");
        query.clone().execute(&mut conn).unwrap();

        let mut reverted_migrations = revert_all_migrations(&mut conn).unwrap();
        reverted_migrations.reverse();
        assert_eq!(migrations, reverted_migrations);

        query.execute(&mut conn).unwrap_err();
    }
}
