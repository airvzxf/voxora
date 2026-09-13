//! `voxora serve` subcommand — placeholder for a future HTTP
//! front-end (per ROADMAP.md, this lands in a later phase).

use crate::args::Cli;
use crate::error::CliError;

pub fn run(cli: &Cli) -> Result<(), CliError> {
    let _ = cli; // silence unused-by-default warnings
    Err(CliError::NotImplemented {
        feature: "`voxora serve` (HTTP front-end); tracked in docs/ROADMAP.md (Phase 5+)".into(),
    })
}
