//! `voxora serve` subcommand — placeholder for a future HTTP
//! front-end (per ROADMAP.md, this lands in a later phase).

use crate::args::Cli;
use crate::error::CliError;

pub fn run(cli: &Cli) -> Result<(), CliError> {
    let _ = cli; // silence unused-by-default warnings
    Err(CliError::InvalidInput(
        "`voxora serve` is not implemented yet; tracked in docs/ROADMAP.md (Phase 5+)".into(),
    ))
}
