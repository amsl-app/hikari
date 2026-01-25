use hikari_utils::loader::s3::S3Config;

#[derive(Debug, Clone)]
#[allow(clippy::struct_field_names)]
pub struct TTSConfig {
    pub api_key: String,
    pub model: String,
    pub voice: String,
    pub cache_config: Option<S3Config>,
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
