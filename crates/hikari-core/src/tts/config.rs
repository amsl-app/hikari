use hikari_utils::{args::tts::TTSCacheConfig, loader::s3::S3Config};

#[derive(Debug, Clone)]
#[allow(clippy::struct_field_names)]
pub struct TTSConfig {
    pub api_key: String,
    pub model: String,
    pub voice: String,
    pub cache_config: Option<CacheConfig>,
}

impl From<hikari_utils::args::tts::TTSConfig> for TTSConfig {
    fn from(value: hikari_utils::args::tts::TTSConfig) -> Self {
        TTSConfig {
            api_key: value.api_key,
            model: value.model,
            voice: value.voice,
            cache_config: value.cache_config.map(|c| c.into()),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::struct_field_names)]
pub struct CacheConfig {
    pub bucket: String,
    pub s3_config: S3Config,
}

impl From<TTSCacheConfig> for CacheConfig {
    fn from(value: TTSCacheConfig) -> Self {
        let s3 = S3Config {
            endpoint: value.endpoint,
            region: value.region,
            access_key: value.access_key,
            secret_key: value.secret_key,
        };
        CacheConfig {
            bucket: value.bucket,
            s3_config: s3,
        }
    }
}
