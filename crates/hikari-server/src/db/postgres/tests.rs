use super::MIGRATIONS;
use crate::db::error::DbError;
use diesel::migration::MigrationVersion;
use diesel::prelude::*;
use diesel::sql_query;
use diesel_migrations::MigrationHarness;
use std::borrow::Cow;

use hikari_test_helpers::{PostgresqlDb, TestDb};

use crate::db;
use serial_test::serial;
use test_log::test;

pub(crate) fn revert_all_migrations(conn: &'_ mut PgConnection) -> Result<Vec<MigrationVersion<'_>>, DbError> {
    tracing::debug!("reverting migrations");
    conn.revert_all_migrations(MIGRATIONS)
        .map_err(|err| DbError::MigrationFailed(format! {"{err}"}))
}

struct Db {
    pg: PostgresqlDb,
}

impl Db {
    fn db_uri(&'_ self) -> Cow<'_, str> {
        self.pg.db_uri()
    }
}

impl Drop for Db {
    fn drop(&mut self) {
        match &self.pg {
            PostgresqlDb::Native(db_uri) => {
                let mut conn = PgConnection::establish(db_uri.as_str()).unwrap();
                revert_all_migrations(&mut conn).unwrap();
            }
            PostgresqlDb::Embedded(_, _) => {}
        }
    }
}

async fn setup_db(run_migrations: bool) -> Db {
    let db = Db {
        pg: PostgresqlDb::new().await.unwrap(),
    };

    let db_uri = db.db_uri();

    println!("DB: {db_uri}");

    let mut conn = PgConnection::establish(db_uri.as_ref()).unwrap();
    if run_migrations {
        db::run_migrations(&mut conn, MIGRATIONS).unwrap();
    }

    db
}

#[test(tokio::test)]
#[serial(postgres)]
async fn test_migrations() {
    let db = setup_db(false).await;
    let db_uri = db.db_uri();
    let db_uri = db_uri.as_ref();
    let mut conn = PgConnection::establish(db_uri).unwrap();
    // Do it twice to make sure the migrations are idempotent
    for _ in 0..2 {
        let migrations = db::run_migrations(&mut conn, MIGRATIONS).unwrap();

        let mut conn = PgConnection::establish(db_uri).unwrap();
        let query = sql_query("SELECT id FROM users WHERE subject = 'test'");
        query.clone().execute(&mut conn).unwrap();

        let mut reverted_migrations = revert_all_migrations(&mut conn).unwrap();
        reverted_migrations.reverse();
        assert_eq!(migrations, reverted_migrations);

        query.execute(&mut conn).unwrap_err();
    }
}
