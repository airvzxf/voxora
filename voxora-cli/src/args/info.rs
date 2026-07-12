//! `voxora info <hf-model-id>` subcommand.

use clap::Args;

use voxora_core::ModelSource;

use crate::args::Cli;
use crate::error::CliError;

#[derive(Debug, Args)]
pub struct InfoOpts {
    /// The Hugging Face model id (e.g. `Qwen/Qwen3-ASR-0.6B`).
    pub model_id: String,

    /// Pin a specific git revision (branch, tag, or SHA).
    #[arg(long, value_name = "REV")]
    pub revision: Option<String>,
}

pub async fn run(cli: &Cli, opts: &InfoOpts) -> Result<(), CliError> {
    if !opts.model_id.contains('/') {
        return Err(CliError::InvalidInput(format!(
            "model id {:?} must be in 'org/name' form",
            opts.model_id
        )));
    }

    let source = cli.build_source()?;

    let caps = source.capabilities_for(&opts.model_id).await?;

    crate::output::print_info(&opts.model_id, source.name(), &caps);
    Ok(())
}

#[cfg(test)]
mod tests;
