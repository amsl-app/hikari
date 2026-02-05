use clap::Args;
use url::Url;

use crate::loader::s3::S3Config;

#[derive(Debug, Clone, Args)]
#[allow(clippy::struct_field_names)]
pub struct TTSConfig {
    #[arg(long = "elevenlabs-key", required = false)]
    pub api_key: String,
    #[arg(long = "elevenlabs-model", required = false)]
    pub model: String,
    #[arg(long = "elevenlabs-voice", required = false)]
    pub voice: String,
    #[command(flatten)]
    pub cache_config: Option<TTSCacheConfig>,
}

#[derive(Debug, Clone, Args)]
#[allow(clippy::struct_field_names)]
pub struct TTSCacheConfig {
    #[arg(long = "cache-s3-endpoint", required = false)]
    pub endpoint: Url,
    #[arg(long = "cache-s3-region", required = false)]
    pub region: String,
    #[arg(long = "cache-s3-access_key", required = false)]
    pub access_key: String,
    #[arg(long = "cache-s3-secret_key", required = false)]
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
