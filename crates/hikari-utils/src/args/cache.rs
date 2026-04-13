use clap::Args;
use url::Url;

#[derive(Debug, Clone, Args)]
#[allow(clippy::struct_field_names)]
pub struct CacheConfig {
    #[arg(long = "cache-s3-endpoint", required = false)]
    pub cache_endpoint: Url,
    #[arg(long = "cache-s3-region", required = false)]
    pub cache_region: String,
    #[arg(long = "cache-s3-bucket", required = false)]
    pub cache_bucket: String,
    #[arg(long = "cache-s3-access-key", required = false)]
    pub cache_access_key: String,
    #[arg(long = "cache-s3-secret-key", required = false)]
    pub cache_secret_key: String,
}
