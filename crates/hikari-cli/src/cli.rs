pub(crate) mod opt;
mod run;
mod validate;

use crate::opt::Commands;
use anyhow::Error;
use run::run;
use validate::validate;

pub(crate) async fn exec(command: Commands) -> Result<(), Error> {
    match command {
        Commands::Run(o) => run(o).await,
        Commands::Validate(o) => validate(o).await,
    }
}
