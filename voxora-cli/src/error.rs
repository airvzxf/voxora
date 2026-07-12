//! CLI error type and exit-code mapping.
//!
//! The CLI differentiates two failure modes:
//!
//! - **Usage / bad input** → exit `2` (conventional Unix "usage error").
//! - **Runtime / underlying failure** → exit `1`.
//!
//! `voxora run` keys off `CliError::exit_code()` so the underlying
//! `AsrError` and HF plumbing don't have to know about process state.

use voxora_core::AsrError;

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
    /// - `1` = runtime failure.
    /// - `2` = usage / build configuration failure.
    pub fn exit_code(&self) -> u8 {
        match self {
            CliError::InvalidInput(_) | CliError::Build(_) => 2,
            CliError::Asr(_) => 1,
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
        CliError::Asr(voxora_core::AsrError::from(value))
    }
}

#[cfg(test)]
mod tests;
