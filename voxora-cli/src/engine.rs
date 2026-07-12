//! Engine selection + dispatch.
//!
//! `select()` resolves the user's intent (explicit `--engine`, or
//! auto-detect from `config.json`) and returns a [`BackendKind`]
//! token. `run()` actually loads the engine and runs one
//! transcription. The two-step dance keeps selection pure (no model
//! load) so the `voxora run --engine=...` validation can fail fast
//! without needing to download anything.

use voxora_core::{AsrEngine, AsrError, ModelSource, TranscribeOptions, TranscriptionResult};

use crate::args::Cli;
use crate::error::CliError;

/// Which engine the CLI decided to load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    /// whisper.cpp via `voxora-whisper`.
    Whisper,
    /// Qwen3-ASR via `voxora-qwen3asr`.
    Qwen3Asr,
}

impl BackendKind {
    /// Stable string label for logs / output.
    pub fn label(self) -> &'static str {
        match self {
            BackendKind::Whisper => "voxora-whisper",
            BackendKind::Qwen3Asr => "voxora-qwen3asr",
        }
    }

    /// Parse the canonical CLI spelling (`whisper` / `qwen3-asr`).
    pub fn from_cli_label(label: &str) -> Result<Self, CliError> {
        match label.to_ascii_lowercase().as_str() {
            "whisper" => Ok(BackendKind::Whisper),
            "qwen3-asr" | "qwen3_asr" | "qwen3asr" => Ok(BackendKind::Qwen3Asr),
            other => Err(CliError::InvalidInput(format!(
                "unknown --engine value {other:?}; expected one of `whisper` or `qwen3-asr`"
            ))),
        }
    }
}

/// Decide which engine to use for `voxora run`.
///
/// Resolution order:
///
/// 1. `--engine <kind>` if provided → return it (after compiling-time
///    availability check below).
/// 2. Auto-detect from `config.json` via
///    [`voxora_hf::HuggingFaceSource::capabilities_for`].
///
/// `_force_redownload` is accepted but not yet wired through; voxora-hf
/// keeps its existing marker-file semantics for now.
pub async fn select(
    cli: &Cli,
    engine_flag: Option<&str>,
    source: &voxora_hf::HuggingFaceSource,
    model_id: &str,
    _force_redownload: bool,
) -> Result<BackendKind, CliError> {
    if let Some(flag) = engine_flag {
        let kind = BackendKind::from_cli_label(flag)?;
        ensure_available(kind, flag)?;
        return Ok(kind);
    }

    // Auto-detect path.
    let caps = source.capabilities_for(model_id).await?;
    let kind = infer_kind_from_capabilities(&caps).ok_or_else(|| {
        CliError::InvalidInput(format!(
            "cannot auto-detect engine for {model_id:?} from `config.json`; \
             pass `--engine whisper` or `--engine qwen3-asr` explicitly"
        ))
    })?;
    ensure_available(kind, "<auto>")?;
    let _ = cli;
    Ok(kind)
}

/// Best-effort classification from the `capabilities_for()` payload.
/// Because that payload only exposes flags (no `architectures[0]`),
/// we cannot always disambiguate. The CLI then asks the user to
/// supply `--engine` explicitly.
///
/// `voxora-hf` already includes a `config.json` field-detection layer
/// (`capabilities::ArchKey`) and the heuristic is wired into
/// [`voxora_hf::HuggingFaceSource::capabilities_for`] future-proof.
/// For now we fall back to: multilingual-only-with-word-timestamps →
/// Whisper; multilingual-no-word-timestamps → Qwen3-ASR.
fn infer_kind_from_capabilities(caps: &voxora_core::ModelCapabilities) -> Option<BackendKind> {
    if caps.word_timestamps {
        Some(BackendKind::Whisper)
    } else if caps.multilingual {
        Some(BackendKind::Qwen3Asr)
    } else {
        None
    }
}

/// Refuse `--engine foo` when the requested crate was feature-disabled
/// at build time.
pub fn ensure_available(kind: BackendKind, label: &str) -> Result<(), CliError> {
    match kind {
        BackendKind::Whisper => {
            if !cfg!(feature = "whisper") {
                return Err(CliError::Build(format!(
                    "--engine {label:?} requested but voxora-cli was built without the `whisper` feature"
                )));
            }
        }
        BackendKind::Qwen3Asr => {
            if !cfg!(feature = "qwen3asr") {
                return Err(CliError::Build(format!(
                    "--engine {label:?} requested but voxora-cli was built without the `qwen3asr` feature"
                )));
            }
        }
    }
    Ok(())
}

/// Load the chosen engine from the given `ModelSource`, then run one
/// transcription. Engine-specific loading is delegated to the
/// `dispatch` submodule so each backend can stay compile-gated by its
/// feature flag.
pub async fn run(
    kind: BackendKind,
    source: &voxora_hf::HuggingFaceSource,
    model_id: &str,
    resolve_opts: &voxora_core::ResolveOptions,
    samples: &[f32],
    transcribe_opts: &TranscribeOptions,
) -> Result<TranscriptionResult, AsrError> {
    let result: TranscriptionResult = match kind {
        BackendKind::Whisper => {
            #[cfg(feature = "whisper")]
            {
                let engine = voxora_whisper::WhisperEngine::from_hf(
                    source as &dyn voxora_core::ModelSource,
                    model_id,
                    resolve_opts,
                )
                .await?;
                engine.transcribe(samples, transcribe_opts)?
            }
            #[cfg(not(feature = "whisper"))]
            {
                let _ = (source, model_id, resolve_opts, samples, transcribe_opts);
                return Err(AsrError::Unsupported(
                    "voxora-whisper (build without `whisper` feature)",
                ));
            }
        }
        BackendKind::Qwen3Asr => {
            #[cfg(feature = "qwen3asr")]
            {
                let engine = voxora_qwen3asr::QwenAsrEngine::from_hf(
                    source as &dyn voxora_core::ModelSource,
                    model_id,
                    resolve_opts,
                )
                .await?;
                engine.transcribe(samples, transcribe_opts)?
            }
            #[cfg(not(feature = "qwen3asr"))]
            {
                let _ = (source, model_id, resolve_opts, samples, transcribe_opts);
                return Err(AsrError::Unsupported(
                    "voxora-qwen3asr (build without `qwen3asr` feature)",
                ));
            }
        }
    };
    Ok(result)
}

#[cfg(test)]
mod tests;
