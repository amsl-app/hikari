use clap::Args;
use url::Url;

use crate::loader::s3::S3Config;

#[derive(Debug, Clone, Args)]
#[allow(clippy::struct_field_names)]
pub struct TTSConfig {
    #[arg(long = "elevenlabs_key", required = false)]
    pub api_key: String,
    #[arg(long = "elevenlabs_model", required = false)]
    pub model: String,
    #[arg(long = "elevenlabs_voice", required = false)]
    pub voice: String,
    #[command(flatten)]
    pub cache_config: Option<TTSCacheConfig>,
}

#[derive(Debug, Clone, Args)]
#[allow(clippy::struct_field_names)]
pub struct TTSCacheConfig {
    #[arg(long = "cache_s3_endpoint", required = false)]
    pub endpoint: Url,
    #[arg(long = "cache_s3_region", required = false)]
    pub region: String,
    #[arg(long = "cache_s3_access_key", required = false)]
    pub access_key: String,
    #[arg(long = "cache_s3_secret_key", required = false)]
    pub secret_key: String,
}

impl From<TTSCacheConfig> for S3Config {
    fn from(value: TTSCacheConfig) -> Self {
        S3Config {
            endpoint: value.endpoint,
            region: value.region,
            access_key: value.access_key,
            secret_key: value.secret_key,
        }
    }
}
