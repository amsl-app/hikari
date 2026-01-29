use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "ornithopter-cli")]
pub struct Cli {
    #[arg(long)]
    pub url: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Stress(Stress),
    Status,
}

#[derive(Parser)]
pub struct Stress {
    #[arg(short, long)]
    pub users: PathBuf,

    #[arg(long)]
    pub oidc_client_id: String,

    #[arg(long)]
    pub oidc_client_secret: Option<String>,

    #[arg(long)]
    pub oidc_issuer_url: String,

    #[arg(long)]
    pub count: Option<usize>,

    #[arg(long)]
    pub module: String,

    #[arg(long)]
    pub session: String,
}
