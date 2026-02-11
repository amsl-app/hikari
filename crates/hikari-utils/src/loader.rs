use crate::loader::error::{LoadingError, ParseError};
use crate::loader::file::{File, FileMetadata};
use crate::loader::file_system::FileSystemLoader;
use crate::loader::s3::{S3Config, S3Loader};
use futures::Stream;
use std::path::Path;
use std::pin::Pin;
use url::Url;

pub mod error;
pub mod file;
pub mod file_system;
pub mod s3;

#[derive(Debug, Clone, Copy, Default)]
pub enum Filter {
    Yaml,
    Csml,
    Pdf,
    #[default]
    Any,
}

impl Filter {
    pub fn apply<P: AsRef<Path>>(&self, path: P) -> bool {
        let path = path.as_ref();
        let extension = path.extension().and_then(|ext| ext.to_str());
        let Some(extension) = extension else {
            return false;
        };
        let allowed_extensions: &[&str] = match self {
            Filter::Yaml => &["yaml", "yml"],
            Filter::Csml => &["csml"],
            Filter::Pdf => &["pdf"],
            Filter::Any => return true,
        };
        allowed_extensions.contains(&extension)
    }
}

pub struct LoaderHandler {
    s3_client: Option<aws_sdk_s3::Client>,
}

impl LoaderHandler {
    #[must_use]
    pub fn new(s3_config: Option<S3Config>) -> Self {
        Self {
            s3_client: s3_config.map(s3::build_client),
        }
    }

    pub fn loader(&self, url: &Url) -> Result<Loader, LoadingError> {
        match url.scheme() {
            "s3" => {
                let client = self
                    .s3_client
                    .clone()
                    .ok_or_else(|| LoadingError::CredentialsError("S3 credentials not set".to_string()))?;

                let s3 = S3Loader::new(client, url)?;
                Ok(Loader::S3(s3))
            }
            "file" => {
                let path = url
                    .to_file_path()
                    .map_err(|()| LoadingError::InvalidURL(url.to_string()))?;
                Ok(Loader::FileSystem(FileSystemLoader::new(path)))
            }
            scheme => Err(LoadingError::Parse(ParseError::Other(format!(
                "Invalid scheme: {scheme}"
            )))),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Loader {
    S3(S3Loader),
    FileSystem(FileSystemLoader),
}

impl LoaderTrait for Loader {
    fn load_dir<'a, P: AsRef<Path>>(
        &'a self,
        path: P,
        filter: Filter,
    ) -> Pin<Box<dyn Stream<Item = Result<File, LoadingError>> + Send + 'a>> {
        match self {
            Loader::S3(loader) => loader.load_dir(path, filter),
            Loader::FileSystem(loader) => loader.load_dir(path, filter),
        }
    }

    async fn load_file<P: AsRef<Path>>(&self, path: P) -> Result<File, LoadingError> {
        match self {
            Loader::S3(loader) => loader.load_file(path).await,
            Loader::FileSystem(loader) => loader.load_file(path).await,
        }
    }

    async fn store_file<P: AsRef<Path>>(&self, path: P, content: &[u8]) -> Result<(), LoadingError> {
        match self {
            Loader::S3(loader) => loader.store_file(path, content).await,
            Loader::FileSystem(loader) => loader.store_file(path, content).await,
        }
    }

    async fn get_file_metadata<P: AsRef<Path>>(&self, path: P) -> Result<FileMetadata, LoadingError> {
        match self {
            Loader::S3(loader) => loader.get_file_metadata(path).await,
            Loader::FileSystem(loader) => loader.get_file_metadata(path).await,
        }
    }
}

pub trait LoaderTrait {
    fn load_dir<'a, P: AsRef<Path>>(
        &'a self,
        path: P,
        filter: Filter,
    ) -> Pin<Box<dyn Stream<Item = Result<File, LoadingError>> + Send + 'a>>;
    fn load_file<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = Result<File, LoadingError>>;
    fn store_file<P: AsRef<Path>>(&self, path: P, content: &[u8]) -> impl Future<Output = Result<(), LoadingError>>;

    fn get_file_metadata<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = Result<FileMetadata, LoadingError>>;
}
