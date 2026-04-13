use hikari_utils::{args::cache::CacheConfig, loader::s3::S3Config};

#[derive(Debug, Clone)]
#[allow(clippy::struct_field_names)]
pub struct TTSConfig {
    pub api_key: String,
    pub model: String,
    pub voice: String,
    pub cache_config: Option<TTSCacheConfig>,
}

impl From<(hikari_utils::args::tts::TTSConfig, Option<CacheConfig>)> for TTSConfig {
    fn from(value: (hikari_utils::args::tts::TTSConfig, Option<CacheConfig>)) -> Self {
        TTSConfig {
            api_key: value.0.api_key,
            model: value.0.model,
            voice: value.0.voice,
            cache_config: value.1.map(|c| c.into()),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::struct_field_names)]
pub struct TTSCacheConfig {
    pub bucket: String,
    pub s3_config: S3Config,
}

impl From<CacheConfig> for TTSCacheConfig {
    fn from(value: CacheConfig) -> Self {
        let s3 = S3Config {
            endpoint: value.cache_endpoint,
            region: value.cache_region,
            access_key: value.cache_access_key,
            secret_key: value.cache_secret_key,
        };
        TTSCacheConfig {
            bucket: value.cache_bucket,
            s3_config: s3,
        }
    }
}
