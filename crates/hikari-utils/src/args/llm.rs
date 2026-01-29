use clap::Args;
use url::Url;

#[derive(Debug, Clone, Args)]
pub struct LlmServices {
    #[arg(long, required = false)]
    pub openai_key: Option<String>,
    #[arg(long, required = false)]
    pub openai_default_model: Option<String>,
    #[arg(long, required = false)]
    pub gwdg_key: Option<String>,
    #[arg(long, required = false)]
    pub gwdg_default_model: Option<String>,
    #[arg(long, required = false)]
    pub win_key: Option<String>,
    #[arg(long, required = false)]
    pub win_default_model: Option<String>,
    #[arg(long, required = false)]
    pub journaling_model: Option<String>,
    #[arg(long, required = false)]
    pub journaling_service: Option<String>,
    #[arg(long, required = false)]
    pub embedding_model: Option<String>,
    #[arg(long, required = false)]
    pub embedding_service: Option<String>,
    #[arg(long, required = false)]
    pub quiz_model: Option<String>,
    #[arg(long, required = false)]
    pub quiz_service: Option<String>,
}

#[derive(Debug, Clone, Args)]
#[allow(clippy::struct_field_names)]
pub struct LlmConfig {
    #[arg(long, required = false)]
    pub llm_structures: Url,
    #[arg(long, required = false)]
    pub llm_collections: Url,
    #[arg(long, required = false, help = "The url were the constants are stored")]
    pub constants: Option<Url>,
}
