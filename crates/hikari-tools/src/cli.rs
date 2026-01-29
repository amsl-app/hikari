pub(crate) mod opt;
pub(crate) mod schema;

use crate::opt::Commands;
use anyhow::Error;

pub(crate) fn exec(command: Commands) -> Result<(), Error> {
    match command {
        Commands::Schema(schema) => schema::exec(schema),
    }
}
