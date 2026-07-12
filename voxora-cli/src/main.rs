//! `voxora` — the command-line front-end for the voxora model-agnostic
//! ASR bridge.
//!
//! Phase 5 deliverable. Subcommands:
//!
//! - `voxora list` — enumerate locally cached models.
//! - `voxora info <hf-model-id>` — fetch metadata + capabilities
//!   without downloading.
//! - `voxora download <hf-model-id>` — resolve + download + cache.
//! - `voxora run <hf-model-id> <audio.wav>` — load, transcribe, print.
//! - `voxora serve` — placeholder; HTTP wrapper deferred to a later
//!   phase.
//!
//! Engine dispatch for `voxora run`:
//!
//! 1. `--engine <whisper|qwen3-asr>` if provided.
//! 2. Otherwise auto-detect from `config.json`
//!    (`voxora-hf::capabilities_for`).
//!
//! See each subcommand module and the `Backend` trait for details.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod args;
mod audio;
mod backend;
mod engine;
mod error;
mod output;
mod resolve_opts;

use std::process::ExitCode;

use clap::Parser;

use crate::args::{Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let quiet = cli.quiet;
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("voxora: failed to start tokio runtime: {e}");
            return ExitCode::from(1);
        }
    };

    let result = runtime.block_on(run(cli));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(cli_err) => {
            let code = cli_err.exit_code();
            if !quiet {
                // Render the full chain once so users see the actual
                // failure (the conversion that produced "exit code 2"
                // — e.g. "unknown --quantization value") instead of a
                // bare number.
                eprintln!("voxora: {cli_err}");
                if let Some(source) = std::error::Error::source(&cli_err) {
                    eprintln!("voxora: caused by: {source}");
                }
            }
            ExitCode::from(code)
        }
    }
}

async fn run(cli: Cli) -> Result<(), error::CliError> {
    match cli.command {
        Command::List => args::list::run(&cli),
        Command::Info(ref opts) => args::info::run(&cli, opts).await,
        Command::Download(ref opts) => args::download::run(&cli, opts).await,
        Command::Run(ref opts) => args::run::run(&cli, opts).await,
        Command::Serve => args::serve::run(&cli),
    }
}
