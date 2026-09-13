//! CLI error type and exit-code mapping.
//!
//! The CLI differentiates two failure modes:
//!
//! - **Usage / bad input** → exit `2` (conventional Unix "usage error").
//! - **Runtime / underlying failure** → exit `1`.
//!
//! `voxora run` keys off `CliError::exit_code()` so the underlying
//! `AsrError` and HF plumbing don't have to know about process state.

use voxora_traits::AsrError;

/// All failure modes the CLI knows about.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// User-supplied input was rejected (bad flag value, missing
    /// model id form, etc.).
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// The binary was built without the feature requested (e.g. the
    /// user passed `--engine whisper` but the `qwen3asr`-only build
    /// does not link voxora-whisper).
    #[error("build configuration error: {0}")]
    Build(String),

    /// The requested subcommand / feature is known but not yet
    /// implemented (closes [#112](https://github.com/airvzxf/voxora/issues/112)).
    /// Distinct from `InvalidInput` because a missing feature is not a
    /// usage error — the user invoked a valid subcommand whose
    /// implementation simply has not landed yet.
    #[error("not implemented: {feature}")]
    NotImplemented { feature: String },

    /// Underlying `voxora-hf` failure. The public `HuggingFaceSource`
    /// already maps its internal `HfError` to `AsrError`; we forward
    /// the [`AsrError`] here so the `?` operator works without a
    /// second `map_err` call at every call site.
    #[error("{0}")]
    Asr(#[source] AsrError),
}

impl CliError {
    /// Process exit code:
    ///
    /// - `0` = success (never returned here).
    /// - `1` = runtime / not-implemented failure.
    /// - `2` = usage / build configuration failure.
    ///
    /// Per `sysexits.h(3)`, exit `2` is reserved for command-line
    /// usage errors. A missing feature is a runtime failure and
    /// therefore uses exit `1`.
    pub fn exit_code(&self) -> u8 {
        match self {
            CliError::InvalidInput(_) | CliError::Build(_) => 2,
            CliError::NotImplemented { .. } | CliError::Asr(_) => 1,
        }
    }
}

impl From<AsrError> for CliError {
    fn from(value: AsrError) -> Self {
        CliError::Asr(value)
    }
}

impl From<voxora_hf::HfError> for CliError {
    fn from(value: voxora_hf::HfError) -> Self {
        CliError::Asr(voxora_traits::AsrError::from(value))
    }
}

#[cfg(test)]
mod tests;
