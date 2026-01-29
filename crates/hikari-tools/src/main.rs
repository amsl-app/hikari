mod cli;

use anyhow::Result;
use clap::Parser;
use cli::opt;

#[tokio::main]
async fn main() -> Result<()> {
    let opt = opt::Cli::parse();
    cli::exec(opt.command)
}
