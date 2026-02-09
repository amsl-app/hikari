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
    pub async fn new() -> Self {
        let postgres_db = std::env::var("POSTGRES_URI");

        if let Ok(postgres_db) = postgres_db {
            tracing::info!("using postgres db from env var");
            Self::new_native(postgres_db)
        } else {
            tracing::info!("using embedded postgres db");
            Self::new_embedded().await
        }
    }

    #[must_use]
    pub fn new_native(db_uri: String) -> Self {
        Self::Native(db_uri)
    }

    pub async fn new_embedded() -> Self {
        let database_dir = TempDir::with_prefix("test-pg-db").unwrap();
        let pg = Self::start_db(database_dir.path().to_path_buf()).await;

        Self::Embedded(Box::new(pg), database_dir)
    }

    async fn setup_db(database_dir: PathBuf) -> PostgreSQL {
        let installation_dir = tempfile::env::temp_dir().join("test-pg-installation");
        match std::fs::create_dir(&installation_dir) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => panic!("failed to create installation dir: {}", e),
        };
        let data_dir = database_dir.join("data");
        let password_file = database_dir.join(".pgpass");
        let configuration = HashMap::default();

        let pg_settings = Settings {
            releases_url: postgresql_archive::configuration::theseus::URL.to_string(),
            version: VersionReq::parse("=16.11.0").unwrap(),
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
        pg.setup().await.unwrap();

        postgresql_extensions::install(
            pg.settings(),
            "portal-corp",
            "pgvector_compiled",
            &VersionReq::default(),
        )
        .await
        .unwrap();

        pg
    }

    async fn start_db(database_dir: PathBuf) -> PostgreSQL {
        const MAX_TRIES: u32 = 5;
        let mut retry = 0;
        loop {
            let port = {
                let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
                listener.into_std().unwrap().local_addr().unwrap().port()
            };

            tracing::info!(retry, port, "staring db");
            let mut pg = Self::setup_db(database_dir.clone()).await;

            let res = pg.start().await;
            match res {
                Ok(()) => return pg,
                Err(error) => {
                    retry += 1;
                    if retry >= MAX_TRIES {
                        panic!("{:?}", error);
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
        let db = PostgresqlDb::new().await;
        tracing::info!("stopping db");
        drop(db);
        tracing::info!("stopped db");
    }
}
