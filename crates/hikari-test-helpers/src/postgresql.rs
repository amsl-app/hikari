use crate::TestDb;
use postgresql_embedded::{Error, PostgreSQL, Settings};
use semver::VersionReq;
use std::borrow::Cow;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use thiserror::Error;
use tokio::net::TcpListener;

pub enum PostgresqlDb {
    Embedded(Box<PostgreSQL>, TempDir),
    Native(String),
}

#[derive(Error, Debug)]
pub enum PgError {
    #[error(transparent)]
    PgEmbed(#[from] Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl PostgresqlDb {
    pub async fn new() -> Result<Self, PgError> {
        let postgres_db = std::env::var("POSTGRES_URI");
        let db = if let Ok(postgres_db) = postgres_db {
            tracing::info!("using postgres db from env var");
            Self::new_native(postgres_db)
        } else {
            tracing::info!("using embedded postgres db");
            Self::new_embedded().await?
        };
        Ok(db)
    }

    #[must_use]
    pub fn new_native(db_uri: String) -> Self {
        Self::Native(db_uri)
    }

    pub async fn new_embedded() -> Result<Self, PgError> {
        let temp_dir = TempDir::with_prefix("test-pg-db")?;

        let pg = Self::start_db(temp_dir.path().to_path_buf()).await?;

        Ok(Self::Embedded(Box::new(pg), temp_dir))
    }

    async fn setup_db(database_dir: PathBuf) -> Result<PostgreSQL, Error> {
        let installation_dir = database_dir.join("installation");
        let data_dir = database_dir.join("data");
        let password_file = database_dir.join(".pgpass");
        let configuration = HashMap::default();

        let pg_settings = Settings {
            releases_url: postgresql_archive::configuration::theseus::URL.to_string(),
            version: VersionReq::parse("~16.11.0")?,
            installation_dir,
            password_file,
            data_dir,
            host: "localhost".to_string(),
            port: 0,
            username: "postgres".to_owned(),
            password: "password".to_owned(),
            temporary: false,
            timeout: Some(Duration::from_secs(10)),
            configuration,
            trust_installation_dir: false,
        };
        let mut pg = PostgreSQL::new(pg_settings);
        tracing::info!("setting up db");
        pg.setup().await?;

        postgresql_extensions::install(
            pg.settings(),
            "portal-corp",
            "pgvector_compiled",
            &VersionReq::default(),
        )
        .await
        .unwrap();

        Ok(pg)
    }

    async fn start_db(database_dir: PathBuf) -> Result<PostgreSQL, Error> {
        const MAX_TRIES: u32 = 5;
        let mut retry = 0;
        loop {
            let port = {
                let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
                listener.into_std()?.local_addr()?.port()
            };

            tracing::info!(retry, port, "staring db");
            let mut pg = Self::setup_db(database_dir.clone()).await?;

            let res = pg.start().await;
            match res {
                Ok(()) => return Ok(pg),
                Err(error) => {
                    retry += 1;
                    if retry >= MAX_TRIES {
                        return Err(error);
                    }
                }
            }
        }
    }
}

impl TestDb for PostgresqlDb {
    fn db_uri(&self) -> Cow<'_, str> {
        match &self {
            Self::Embedded(db, ..) => db.settings().url("postgres").into(),
            Self::Native(db_uri) => db_uri.as_str().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test(tokio::test)]
    async fn test_start_stop() {
        tracing::info!("starting db");
        let db = PostgresqlDb::new().await.unwrap();
        tracing::info!("stopping db");
        drop(db);
        tracing::info!("stopped db");
    }
}
