//! Engine selection + dispatch.
//!
//! `select()` resolves the user's intent (explicit `--engine`, or
//! auto-detect from `config.json`) and returns an [`EngineFamily`]
//! token. `run()` actually loads the engine and runs one
//! transcription. The two-step dance keeps selection pure (no model
//! load) so the `voxora run --engine=...` validation can fail fast
//! without needing to download anything.

use voxora_core::{AsrEngine, AsrError, ModelSource, TranscribeOptions, TranscriptionResult};
use voxora_engine::EngineFamily;

use crate::args::Cli;
use crate::error::CliError;

/// Parse a `--engine` flag into an [`EngineFamily`].
///
/// Mirrors [`EngineFamily::from_config`] but maps the `None` case to a
/// [`CliError::InvalidInput`] with the same wording the deprecated
/// `BackendKind::from_cli_label` produced, so error messages are
/// stable across the 0.2.x → 0.3.0 migration.
pub fn from_cli_label(label: &str) -> Result<EngineFamily, CliError> {
    EngineFamily::from_config(label).ok_or_else(|| {
        CliError::InvalidInput(format!(
            "unknown --engine value {label:?}; expected one of `whisper` or `qwen3-asr`"
        ))
    })
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
) -> Result<EngineFamily, CliError> {
    if let Some(flag) = engine_flag {
        let kind = from_cli_label(flag)?;
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
fn infer_kind_from_capabilities(caps: &voxora_core::ModelCapabilities) -> Option<EngineFamily> {
    if caps.word_timestamps {
        Some(EngineFamily::Whisper)
    } else if caps.multilingual {
        Some(EngineFamily::Qwen3Asr)
    } else {
        None
    }
}

/// Refuse `--engine foo` when the requested crate was feature-disabled
/// at build time.
pub fn ensure_available(kind: EngineFamily, label: &str) -> Result<(), CliError> {
    match kind {
        EngineFamily::Whisper => {
            if !cfg!(feature = "whisper") {
                return Err(CliError::Build(format!(
                    "--engine {label:?} requested but voxora-cli was built without the `whisper` feature"
                )));
            }
        }
        EngineFamily::Qwen3Asr => {
            if !cfg!(feature = "qwen3asr") {
                return Err(CliError::Build(format!(
                    "--engine {label:?} requested but voxora-cli was built without the `qwen3asr` feature"
                )));
            }
        }
        // `EngineFamily` is `#[non_exhaustive]`; an unknown variant
        // cannot be feature-gated, so it is treated as not-built.
        _ => {
            return Err(CliError::Build(format!(
                "--engine {label:?} requested but no voxora-cli backend is wired for that family"
            )));
        }
    }
    Ok(())
}

/// Load the chosen engine from the given `ModelSource`, then run one
/// transcription. Engine-specific loading is delegated to the
/// `dispatch` submodule so each backend can stay compile-gated by its
/// feature flag.
pub async fn run(
    kind: EngineFamily,
    source: &voxora_hf::HuggingFaceSource,
    model_id: &str,
    resolve_opts: &voxora_core::ResolveOptions,
    samples: &[f32],
    transcribe_opts: &TranscribeOptions,
) -> Result<TranscriptionResult, AsrError> {
    let result: TranscriptionResult = match kind {
        EngineFamily::Whisper => {
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
        EngineFamily::Qwen3Asr => {
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
        // `EngineFamily` is `#[non_exhaustive]`; future families have
        // no voxora-cli backend yet, so refuse with a clear error.
        _ => {
            let _ = (source, model_id, resolve_opts, samples, transcribe_opts);
            return Err(AsrError::Unsupported(
                "engine family has no voxora-cli backend wired in this build",
            ));
        }
    };
    Ok(result)
}

#[cfg(test)]
mod tests;
