use crate::loader::error::LoadingError;
use crate::loader::file::{File, FileHash, FileMetadata};
use crate::loader::{Filter, LoaderTrait};
use async_stream::try_stream;
use async_walkdir::{DirEntry, Filtering, WalkDir};
use chrono::{DateTime, Utc};
use futures::{Stream, StreamExt};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use tokio::fs;
use xxhash_rust::xxh3::xxh3_64;

#[derive(Clone, Debug)]
pub struct FileSystemLoader {
    base_path: PathBuf,
}

impl FileSystemLoader {
    #[must_use]
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn sub_path(&self, path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();
        if path.as_os_str().is_empty() {
            return self.base_path.clone();
        }
        self.base_path.join(path)
    }
}

impl LoaderTrait for FileSystemLoader {
    fn load_dir<'a, P: AsRef<Path>>(
        &'a self,
        path: P,
        filter: Filter,
    ) -> Pin<Box<dyn Stream<Item = Result<File, LoadingError>> + Send + 'a>> {
        let path = self.sub_path(path);
        tracing::trace!(?path, "Loading dir");
        let mut walker = WalkDir::new(path).filter(move |entry| crate::loader::file_system::filter(entry, filter));
        let stream = try_stream! {
            while let Some(entry) = walker.next().await {
                let entry = entry?;
                if entry.file_type().await?.is_file() {
                    let path = entry.path();
                    tracing::trace!(?path, "Loading file");
                    let data = fs::read(&path).await?;
                    let last_modified = get_last_modified(&path).await?;
                    let hash = xxh3_64(&data);
                    let file_hash = FileHash {
                        hash: hex::encode(hash.to_le_bytes()),
                        algorithm: "xxh3_64".into(),
                    };
                    let metadata = FileMetadata {
                        key: path.to_string_lossy().into(),
                        last_modified: Some(last_modified),
                        hash: Some(file_hash),
                    };
                    yield File::new(metadata, data)
                }
            }
        };
        Box::pin(stream)
    }

    async fn load_file<P: AsRef<Path>>(&self, path: P) -> Result<File, LoadingError> {
        let path = self.sub_path(path);
        tracing::trace!(?path, "Loading file");
        let data = fs::read(&path).await?;
        let last_modified = get_last_modified(&path).await?;
        let hash = xxh3_64(&data);
        let file_hash = FileHash {
            hash: hex::encode(hash.to_le_bytes()),
            algorithm: "xxh3_64".into(),
        };
        let metadata = FileMetadata {
            key: path.to_string_lossy().into(),
            last_modified: Some(last_modified),
            hash: Some(file_hash),
        };
        Ok(File::new(metadata, data))
    }

    async fn store_file<P: AsRef<Path>>(&self, path: P, content: &[u8]) -> Result<(), LoadingError> {
        todo!("Storing to local path")
    }

    async fn get_file_metadata<P: AsRef<Path>>(&self, path: P) -> Result<FileMetadata, LoadingError> {
        let path = self.sub_path(path);
        tracing::trace!(?path, "Loading file");
        let last_modified = get_last_modified(&path).await?;
        Ok(FileMetadata::new(
            path.to_string_lossy().to_string(),
            Some(last_modified),
            None,
        ))
    }
}

async fn filter(entry: DirEntry, filter: Filter) -> Filtering {
    let Ok(ft) = entry.file_type().await else {
        panic!("Could not get file type of {entry:?}");
    };
    if ft.is_dir() {
        return Filtering::Continue;
    }

    if filter.apply(entry.path()) {
        Filtering::Continue
    } else {
        Filtering::Ignore
    }
}

async fn get_last_modified<P: AsRef<Path>>(path: P) -> Result<DateTime<Utc>, LoadingError> {
    let modified = fs::metadata(path).await?.modified()?;
    Ok(DateTime::<Utc>::from(modified))
}
