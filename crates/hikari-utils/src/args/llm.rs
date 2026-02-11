use clap::Args;

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
