pub mod user;

use sea_orm::{ConnectionTrait, DbConn, DbErr};

pub async fn setup_schema(db: &DbConn) -> Result<(), DbErr> {
    let migration = match db.get_database_backend() {
        sea_orm::DatabaseBackend::Postgres => include_str!("postgres.sql"),
        sea_orm::DatabaseBackend::Sqlite => include_str!("sqlite.sql"),
        #[allow(clippy::unimplemented)]
        sea_orm::DatabaseBackend::MySql => unimplemented!(),
    };
    // TODO Slightly altered definition for testing (no module foreign key)

    db.execute_unprepared(migration).await?;
    Ok(())
}
