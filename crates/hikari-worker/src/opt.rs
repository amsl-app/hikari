use std::net::IpAddr;

use clap::{Parser, Subcommand};
use hikari_utils::{args::llm::LlmServices, loader::s3::S3Config};
use url::Url;

#[derive(Debug, Parser)]
#[command(name = "hikari-worker", about = "Run the hikari worker")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    Run(Run),
}

#[derive(Debug, Clone, Parser)]
pub(crate) struct Run {
    #[arg(long)]
    pub(crate) host: Option<IpAddr>,

    #[arg(short, long)]
    pub(crate) port: Option<u16>,

    #[arg(short, long, help = "The path were the global config is stored")]
    pub(crate) config: Option<Url>,

    #[arg(long = "sentry-dsn", help = "Sentry url")]
    pub(crate) sentry_dsn: Option<String>,

    #[arg(
        long,
        default_value = "dev",
        help = "Set the environment used by sentry and prometheus"
    )]
    pub(crate) env: String,

    #[command(flatten)]
    pub(crate) llm_services: LlmServices,

    #[command(flatten)]
    pub(crate) s3: Option<S3Config>,

    #[arg(long)]
    pub(crate) db_url: Url,

    #[arg(long, help = "Min connections")]
    pub(crate) db_min_connections: Option<u32>,

    #[arg(long, help = "Max connections")]
    pub(crate) db_max_connections: Option<u32>,

    #[arg(long)]
    pub(crate) otlp_endpoint: Option<String>,
}
