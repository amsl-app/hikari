use crate::loader::error::LoadingError;
use crate::loader::file::{File, FileHash};
use crate::loader::{FileMetadata, Filter, LoaderTrait};
use async_stream::try_stream;
use aws_sdk_s3::config::{BehaviorVersion, Credentials, SharedCredentialsProvider};
use aws_sdk_s3::operation::get_object::GetObjectOutput;
use aws_sdk_s3::types::Object;
use aws_sdk_s3::{Client, config::Region};
use chrono::{DateTime, Utc};
use clap::Args;
use futures::future::BoxFuture;
use futures::{FutureExt, Stream};
use num_traits::cast::ToPrimitive;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use url::Url;

#[derive(Debug, Clone, Args)]
#[allow(clippy::struct_field_names)]
pub struct S3Config {
    #[arg(long = "s3-endpoint", required = false)]
    pub endpoint: Url,
    #[arg(long = "s3-region", required = false)]
    pub region: String,
    #[arg(long = "s3-access_key", required = false)]
    pub access_key: String,
    #[arg(long = "s3-secret_key", required = false)]
    pub secret_key: String,
}

#[derive(Clone, Debug)]
pub struct S3Loader {
    client: Client,
    bucket: String,
    prefix: String,
}

impl S3Loader {
    pub fn new(client: Client, url: &Url) -> Result<Self, LoadingError> {
        let bucket_name = url
            .host_str()
            .ok_or_else(|| LoadingError::InvalidURL(url.to_string()))?;
        let base_path = url.path();
        Ok(Self {
            client,
            bucket: bucket_name.to_owned(),
            prefix: base_path.strip_prefix("/").unwrap_or(base_path).into(),
        })
    }

    async fn load_object(&self, key: String) -> Result<File, LoadingError> {
        tracing::debug!(?key, "loading object from s3");
        let object = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .inspect_err(|error| tracing::error!(error = error as &dyn Error, key, "failed to get object"))?;
        let hash = get_object_to_file_hash(&object);
        let bytes = object
            .body
            .collect()
            .await
            .inspect_err(|error| tracing::error!(error = error as &dyn Error, key, "failed to get object body"))?
            .to_vec();
        let last_modified = object.last_modified.map(aws_datetime_to_chronos);
        tracing::trace!(key, e_tag = ?object.e_tag, ?last_modified, size = bytes.len(), "loaded object");
        let metadata = FileMetadata {
            key,
            last_modified,
            hash,
        };
        let file = File::new(metadata, bytes);
        Ok(file)
    }

    fn load_keys<'a>(&'a self, prefix: String, filter: &'a Filter) -> BoxFuture<'a, Result<Vec<String>, LoadingError>> {
        async move {
            tracing::debug!(bucket = &self.bucket, prefix, "listing objects");
            let objects = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(&prefix)
                .send()
                .await
                .inspect_err(|error| tracing::error!(error = error as &dyn Error, "failed to list objects"))?;

            let keys = objects
                .contents
                .unwrap_or_default()
                .into_iter()
                .filter_map(|obj| obj.key)
                .filter(|key| filter.apply(key))
                .collect();

            // We do not descend into subfolders here as the s3 api will get us all child objets
            //  (because we did not specify a delimiter)
            Ok(keys)
        }
        .boxed()
    }

    fn sub_key(&self, path: impl AsRef<Path>) -> String {
        let path = path.as_ref();
        if path.as_os_str().is_empty() {
            return self.prefix.clone();
        }
        PathBuf::from(&self.prefix).join(path).to_string_lossy().to_string()
    }
}

macro_rules! impl_to_file_hash {
    ($name:ident, $t:ty) => {
        fn $name(object: &$t) -> Option<FileHash> {
            // The e_tag is actually the md5 sum of the file
            object.e_tag.as_ref().map(|e_tag| FileHash {
                hash: e_tag.clone(),
                algorithm: "md5".into(),
            })
        }
    };
}

impl_to_file_hash!(object_to_file_hash, Object);
impl_to_file_hash!(get_object_to_file_hash, GetObjectOutput);

fn aws_datetime_to_chronos(date_time: aws_smithy_types::DateTime) -> DateTime<Utc> {
    DateTime::from_timestamp_nanos(
        // Default to January 1, 1970, UTC.
        date_time.as_nanos().to_i64().unwrap_or_default(),
    )
}

impl LoaderTrait for S3Loader {
    fn load_dir<'a, P: AsRef<Path>>(
        &'a self,
        path: P,
        filter: Filter,
    ) -> Pin<Box<dyn Stream<Item = Result<File, LoadingError>> + Send + 'a>> {
        let path = self.sub_key(path);
        tracing::trace!(?path, "loading dir");
        let stream = try_stream! {
            let keys = self.load_keys(self.prefix.clone(), &filter).await?;
            for key in keys {
                tracing::trace!(?key, "loading object");
                let file = self.load_object(key).await?;
                yield file;
            }
        };
        Box::pin(stream)
    }

    async fn load_file<P: AsRef<Path>>(&self, path: P) -> Result<File, LoadingError> {
        let key = self.sub_key(path);
        tracing::trace!(?key, "loading object");

        self.load_object(key).await
    }

    async fn store_file<P: AsRef<Path>>(&self, path: P, content: &[u8]) -> Result<(), LoadingError> {
        let key = self.sub_key(path);
        tracing::trace!(?key, "Storing file");
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(content.to_vec().into())
            .send()
            .await
            .inspect_err(|error| tracing::error!(error = error as &dyn Error, key, "failed to put object"))?;
        Ok(())
    }

    async fn get_file_metadata<P: AsRef<Path>>(&self, path: P) -> Result<FileMetadata, LoadingError> {
        let key = self.sub_key(path);

        tracing::trace!(?key, "getting file metadata");

        // TODO HACK we want to use get_object_attributes here but that currently does not work
        //  with ceph so we do this funny little hack were we request a object listing
        //  and expecting only a single object as a response
        let object_list = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(key.clone())
            .delimiter("/")
            .send()
            .await
            .inspect_err(|error| {
                tracing::error!(
                    error = error as &dyn Error,
                    key,
                    response_body_head_bytes = ?error.raw_response().and_then(|response|
                        response.body().bytes().map(|bytes| bytes.get(..15).unwrap_or(bytes))
                    ),
                    response_body_head_str = ?error.raw_response().and_then(|response|
                        response.body().bytes().map(
                            |bytes| str::from_utf8(bytes.get(..300).unwrap_or(bytes))
                        )
                    ), "failed to get object attributes"
                );
            })?;
        if object_list.common_prefixes.is_some() {
            tracing::error!(key, "key is not unique");
            return Err(LoadingError::InvalidPath(PathBuf::from(key)));
        }
        let Some(mut objects) = object_list.contents else {
            tracing::error!(key, "key does not exist");
            return Err(LoadingError::InvalidPath(PathBuf::from(key)));
        };
        let Some(object) = objects.pop() else {
            tracing::error!(key, "key does not exist");
            return Err(LoadingError::InvalidPath(PathBuf::from(key)));
        };
        if !objects.is_empty() {
            tracing::error!(key, "key is not unique");
            return Err(LoadingError::InvalidPath(PathBuf::from(key)));
        }
        let Some(object_key) = &object.key else {
            tracing::error!(key, "object has no key");
            return Err(LoadingError::InvalidPath(PathBuf::from(key)));
        };
        if object_key != &key {
            tracing::error!(key, "object key did not match provided key");
            return Err(LoadingError::InvalidPath(PathBuf::from(key)));
        }
        let last_modified = if let Some(last_modified) = object.last_modified {
            Some(aws_datetime_to_chronos(last_modified))
        } else {
            tracing::warn!(key, "object has no last modified");
            None
        };
        tracing::trace!(key, ?last_modified, e_tag = ?object.e_tag, "got file metadata");
        let metadata = FileMetadata {
            key,
            last_modified,
            hash: object_to_file_hash(&object),
        };

        Ok(metadata)
    }
}

pub(crate) fn build_client(config: S3Config) -> Client {
    let region = Region::new(config.region);
    let credentials = Credentials::new(config.access_key, config.secret_key, None, None, "s3");

    let s3_config = aws_config::SdkConfig::builder()
        .behavior_version(BehaviorVersion::v2025_08_07())
        .region(region)
        .endpoint_url(config.endpoint.as_str())
        .credentials_provider(SharedCredentialsProvider::new(credentials))
        .build();

    Client::new(&s3_config)
}
