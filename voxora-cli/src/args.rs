//! Parser and top-level `Cli` / `Command` definitions for `voxora`.
//!
//! Subcommand-specific argument structs live in `args/<subcommand>.rs`
//! (one module per subcommand). This file owns the global options that
//! apply to every subcommand (cache root, token, quiet flag), plus the
//! shared `Cli::parse()` entry point.

use std::path::PathBuf;

use clap::{Parser, ValueHint};

use crate::error::CliError;

pub mod download;
pub mod info;
pub mod list;
pub mod run;
pub mod serve;

#[cfg(test)]
mod tests;

/// Top-level CLI surface.
///
/// `clap` derives `--help` from every `#[command(...)]` / `#[arg(...)]`
/// attribute; the rendered help is the source of truth for the UX.
#[derive(Debug, Parser)]
#[command(
    name = "voxora",
    version,
    about = "Model-agnostic Speech-to-Text (Whisper, Qwen3-ASR, …) on top of candle.",
    long_about = "voxora-cli demonstrates the full voxora stack end-to-end.\n\n\
                  Use `voxora list` to see locally cached models, `voxora info <id>` \
                  to fetch model metadata without downloading, `voxora download <id>` \
                  to fetch weights, and `voxora run <id> <audio.wav>` to transcribe."
)]
pub struct Cli {
    /// Override the on-disk model cache root
    /// (`$XDG_CACHE_HOME/voxora/models/huggingface` by default).
    #[arg(long, value_name = "DIR", global = true, value_hint = ValueHint::DirPath)]
    pub cache: Option<PathBuf>,

    /// Override the Hugging Face bearer token. By default the
    /// `HF_TOKEN` / `HUGGING_FACE_HUB_TOKEN` environment variables are
    /// consulted in that order.
    #[arg(long, value_name = "TOKEN", global = true)]
    pub token: Option<String>,

    /// Override the Hugging Face base URL. Defaults to
    /// `https://huggingface.co`. Mostly useful for integration tests
    /// that point at a local mock server.
    #[arg(long, value_name = "URL", global = true, hide = true)]
    pub base_url: Option<String>,

    /// Suppress per-file progress and the final summary line on stderr.
    #[arg(long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Command,
}

/// Discriminated union of every subcommand.
#[derive(Debug, clap::Subcommand)]
pub enum Command {
    /// List models already cached locally.
    List,
    /// Print metadata + capabilities for a HF model id without
    /// downloading the weights.
    Info(info::InfoOpts),
    /// Resolve + download + cache a HF model id.
    Download(download::DownloadOpts),
    /// Load a cached (or download-on-demand) HF model, transcribe a
    /// WAV file, and print the result to stdout.
    Run(run::RunOpts),
    /// Start an HTTP front-end (placeholder).
    Serve,
}

impl Cli {
    /// Resolve the HF cache directory, appending the standard
    /// `voxora/models/huggingface` suffix if the user only supplied a
    /// base dir (mirrors the storage layout of `voxora-hf`).
    ///
    /// Lookup order — same precedence as `voxora-hf`'s own
    /// `default_cache_root` so the two stay in lock-step:
    ///
    /// 1. `--cache` CLI flag.
    /// 2. `VOXORA_CACHE_DIR` env var.
    /// 3. `dirs::cache_dir()` (which honours `XDG_CACHE_HOME` on
    ///    Linux and falls back to `$HOME/.cache/` when unset, plus
    ///    the OS-native locations on Windows / macOS).
    ///
    /// Returns `None` only when every lookup fails — typically
    /// because the platform has no notion of a user cache directory
    /// at all (rare; only happens in very stripped-down containers).
    pub fn hf_cache_dir(&self) -> Option<std::path::PathBuf> {
        resolve_hf_cache_dir(
            self.cache.as_deref(),
            std::env::var_os("VOXORA_CACHE_DIR")
                .map(std::path::PathBuf::from)
                .as_deref(),
            dirs::cache_dir().as_deref(),
        )
    }

    /// Construct a source builder preconfigured with the global CLI
    /// flags. Centralises the wiring so every subcommand constructs
    /// the source identically.
    pub fn build_source(&self) -> Result<voxora_hf::HuggingFaceSource, CliError> {
        let mut builder = voxora_hf::HuggingFaceSource::builder();
        if let Some(dir) = self.hf_cache_dir() {
            builder = builder.cache_dir(dir);
        }
        if let Some(ref url) = self.base_url {
            builder = builder.base_url(url.clone());
        }
        if let Some(ref token) = self.token {
            builder = builder.token(Some(token.clone()));
        }
        builder.build().map_err(CliError::from)
    }
}

/// Pure (and therefore testable) cache-resolution helper. The CLI
/// surface funnels every path through this function so the three
/// branches are covered by a single test matrix.
fn resolve_hf_cache_dir(
    flag: Option<&std::path::Path>,
    env: Option<&std::path::Path>,
    dirs_cache: Option<&std::path::Path>,
) -> Option<std::path::PathBuf> {
    let base = if let Some(dir) = flag {
        dir.to_path_buf()
    } else if let Some(dir) = env {
        dir.to_path_buf()
    } else {
        dirs_cache?.to_path_buf()
    };
    Some(base.join("voxora").join("models").join("huggingface"))
}
