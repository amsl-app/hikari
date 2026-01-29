use chrono::{DateTime, Utc};
use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMetadata {
    pub key: String,
    pub last_modified: Option<DateTime<Utc>>,
    pub hash: Option<FileHash>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    pub metadata: FileMetadata,
    pub content: Vec<u8>,
}

impl File {
    pub(crate) fn new(metadata: FileMetadata, content: Vec<u8>) -> Self {
        File { metadata, content }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHash {
    pub hash: String,
    pub algorithm: Cow<'static, str>,
}

impl FileMetadata {
    #[must_use]
    pub fn new(key: String, last_modified: Option<DateTime<Utc>>, hash: Option<FileHash>) -> Self {
        Self {
            key,
            last_modified,
            hash,
        }
    }
}
