use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "hikari", about = "Cli for a bright bot")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Commands {
    Schema(Schema),
}

#[derive(Debug, Parser)]
pub(crate) struct Schema {
    #[arg(required = true)]
    pub(crate) output_folder: String,
}
