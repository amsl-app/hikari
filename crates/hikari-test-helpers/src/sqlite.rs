use crate::TestDb;
use std::borrow::Cow;
use tempfile::TempDir;
use thiserror::Error;

pub struct SqliteDb {
    // We keep this around so it does not get dropped early
    #[allow(dead_code)]
    temp_dir: TempDir,
    uri: String,
}

#[derive(Error, Debug)]
pub enum SqliteError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl SqliteDb {
    pub fn new() -> Result<Self, SqliteError> {
        let temp_dir = TempDir::with_prefix("test-sqlite-db")?;
        let uri = temp_dir
            .path()
            .to_path_buf()
            .join("db.sqlite")
            .to_str()
            .ok_or(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid path"))?
            .to_owned();
        let uri = format!("sqlite://{uri}?mode=rwc");

        tracing::info!(uri = ?uri, "return sqlite db uri");
        Ok(Self { temp_dir, uri })
    }
}

impl TestDb for SqliteDb {
    fn db_uri(&self) -> Cow<'_, str> {
        self.uri.as_str().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_start_stop() {
        tracing::info!("starting db");
        let db = SqliteDb::new().unwrap();
        tracing::info!("stopping db");
        drop(db);
        tracing::info!("stopped db");
    }
}
