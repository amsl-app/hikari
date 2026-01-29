use clap::ArgAction;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "hikari", about = "Cli for a bright bot")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    Run(Run),

    Validate(Validate),
}

#[derive(Debug, Parser)]
pub(crate) struct Run {
    #[arg(short, long)]
    pub(crate) debug: bool,
    #[arg(short, long)]
    pub(crate) bot: String,
    #[arg(short, long)]
    pub(crate) flow: Option<String>,
    #[arg(short, long)]
    pub(crate) endpoint: Option<String>,
}

#[derive(Debug, Parser)]
pub(crate) struct Validate {
    #[arg(required = true)]
    pub(crate) paths: Vec<PathBuf>,

    #[arg(long)]
    pub(crate) prefix: Option<String>,

    #[arg(
        long,
        default_missing_value("true"),
        default_value("true"),
        num_args(0..=1),
        require_equals(true),
        action = ArgAction::Set
    )]
    pub(crate) strict: bool,
}
