use std::net::IpAddr;

use crate::data::opt::{NamedOptionalValue, NamedOptionalValueParser};
use clap::{Args, Parser, Subcommand};

use hikari_utils::{
    args::{cache::CacheConfig, llm::LlmServices, tts::TTSConfig},
    loader::s3::S3Config,
};
use url::Url;

#[derive(Debug, Parser)]
#[command(name = "hikari", about = "Run a bright bot")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    Run(Run),
}

#[derive(Debug, Clone, Args)]
#[group(multiple = true, required = false)]
pub(crate) struct Db {
    #[arg(long, help = "Min connections")]
    pub(crate) db_min_connections: Option<u32>,

    #[arg(long, help = "Max connections")]
    pub(crate) db_max_connections: Option<u32>,
}

#[derive(Debug, Clone, Args)]
#[group(multiple = true, required = false)]
pub(crate) struct Auth {
    #[arg(long, required = true)]
    pub(crate) oidc_issuer_url: Url,

    #[arg(long = "aud", value_delimiter = ',')]
    pub(crate) audience: Vec<String>,

    #[arg(long = "groups", value_delimiter = ',')]
    pub(crate) groups: Vec<String>,

    #[arg(long, help = "Groups tag of jwt, if present is stored and set in csml metadata")]
    pub(crate) groups_claim: Option<String>,

    #[arg(long)]
    pub(crate) origins: Vec<String>,

    #[arg(long = "require-claim", help = "Required claim. Value is optional and has to be the json value.", value_parser = NamedOptionalValueParser)]
    pub(crate) required_claims: Vec<NamedOptionalValue>,
}

#[derive(Debug, Clone, Parser)]
pub(crate) struct Run {
    #[arg(long)]
    pub(crate) host: Option<IpAddr>,

    #[arg(short, long)]
    pub(crate) port: Option<u16>,

    #[command(flatten)]
    pub(crate) auth: Auth,

    #[command(flatten)]
    pub(crate) llm_services: LlmServices,

    #[command(flatten)]
    pub(crate) s3: Option<S3Config>,

    #[command(flatten)]
    pub(crate) tts: Option<TTSConfig>,

    #[command(flatten)]
    pub(crate) cache: Option<CacheConfig>,

    #[arg(long)]
    pub(crate) workers: Option<usize>,

    #[arg(long, help = "The url where the csml files are stored")]
    pub(crate) csml: Url,

    #[arg(short, long, help = "The url where module config file is stored")]
    pub(crate) config: Url,

    #[arg(short, long, help = "The url where the global config is stored")]
    pub(crate) global_cfg: Option<Url>,

    #[arg(long, help = "The url where assessment config is stored")]
    pub(crate) assessment: Option<Url>,

    #[arg(long, required = false, help = "The url where the structures are stored")]
    pub llm_structures: Url,

    #[arg(long, required = false, help = "The url where the collections are stored")]
    pub llm_collections: Url,

    #[arg(long, required = false, help = "The url where the constants are stored")]
    pub constants: Option<Url>,

    #[arg(long, help = "If set it is possible to delete a user and all his data")]
    pub(crate) deletable: bool,

    #[arg(long = "sentry-dsn", help = "Sentry url")]
    pub(crate) sentry_dsn: Option<String>,

    #[arg(
        long,
        default_value = "dev",
        help = "Set the environment used by sentry and prometheus"
    )]
    pub(crate) env: String,

    #[command(flatten)]
    pub(crate) db: Db,

    #[arg(long, help = "The url of the worker")]
    pub(crate) worker_url: Url,

    #[arg(long)]
    pub(crate) otlp_endpoint: Option<String>,
}
