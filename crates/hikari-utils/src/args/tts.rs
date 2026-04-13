use clap::Args;

#[derive(Debug, Clone, Args)]
#[allow(clippy::struct_field_names)]
pub struct TTSConfig {
    #[arg(long = "elevenlabs-key", required = false)]
    pub api_key: String,
    #[arg(long = "elevenlabs-model", required = false)]
    pub model: String,
    #[arg(long = "elevenlabs-voice", required = false)]
    pub voice: String,
}
