//! The [`WhisperEngine`] type — a [`voxora_traits::AsrEngine`] backed by
//! [`whisper-rs`] (Rust bindings for whisper.cpp).
//!
//! Holds a [`whisper_rs::WhisperContext`] behind an `Arc` so the
//! underlying model file is mmap'd once and shared across every
//! transcription call. Each call creates a fresh
//! [`whisper_rs::WhisperState`] (per-call decode state) — `whisper-rs`
//! supports concurrent states per context out of the box.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use voxora_traits::{
    AsrEngine, AsrError, ModelCapabilities, TranscribeOptions, TranscriptionResult,
};

use crate::language;
use crate::params;

/// Whisper GGML model loaded into a [`whisper_rs::WhisperContext`] and
/// exposed through the voxora [`AsrEngine`] trait.
///
/// The engine is `Send + Sync` so it can be shared across threads
/// behind an `Arc<dyn AsrEngine>` (the standard voxora pattern).
pub struct WhisperEngine {
    ctx: Arc<whisper_rs::WhisperContext>,
    model_path: PathBuf,
    capabilities: ModelCapabilities,
}

impl WhisperEngine {
    /// Load a Whisper GGML model from a `.bin` file on disk.
    ///
    /// `model_path` must point to a file produced by `whisper.cpp`'s
    /// download / convert scripts (e.g. `ggml-tiny.bin`,
    /// `ggml-base.en.bin`, `ggml-large-v3.bin`). The file is opened
    /// once and held in memory for the lifetime of the engine.
    ///
    /// # Errors
    ///
    /// - [`AsrError::InvalidInput`] if `model_path` does not exist or
    ///   is not a regular file.
    /// - [`AsrError::Inference`] if whisper.cpp rejects the model
    ///   (corrupt file, unsupported quantization, etc.).
    pub fn load(model_path: &Path) -> Result<Self, AsrError> {
        let meta = std::fs::metadata(model_path)
            .map_err(|e| AsrError::audio_io(model_path.to_path_buf(), e))?;
        if !meta.is_file() {
            return Err(AsrError::InvalidInput(format!(
                "model path is not a regular file: {}",
                model_path.display()
            )));
        }

        let ctx_params = whisper_rs::WhisperContextParameters::default();
        let ctx = whisper_rs::WhisperContext::new_with_params(model_path, ctx_params)
            .map_err(|e| AsrError::Inference(format!("whisper init: {e}")))?;
        let ctx = Arc::new(ctx);

        let capabilities = build_capabilities(&ctx);

        Ok(Self {
            ctx,
            model_path: model_path.to_path_buf(),
            capabilities,
        })
    }

    /// Resolve a Hugging Face model id via the supplied
    /// [`voxora_traits::ModelSource`] and load the file the source
    /// actually requested. When the source supplies an `entry`
    /// (single-file requests via 3-segment `org/repo/file` ids), that
    /// exact file is loaded; otherwise the engine falls back to
    /// `locate_model_file`, which lex-sorts the directory listing.
    ///
    /// Requires the `hf` feature (which pulls in `voxora-hf`).
    #[cfg(feature = "hf")]
    pub async fn from_hf(
        source: &dyn voxora_traits::ModelSource,
        model_id: &str,
        opts: &voxora_traits::ResolveOptions,
    ) -> Result<Self, AsrError> {
        let dir = source.resolve(model_id, opts).await?;
        let model_path = pick_model_path(&dir)?;
        Self::load(&model_path)
    }

    /// Return the path the engine was loaded from.
    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    /// Return a human-readable model type identifier (as reported by
    /// whisper.cpp's `whisper_model_type_readable`), or `None` if
    /// whisper.cpp returned a non-UTF-8 label.
    pub fn model_type(&self) -> Option<String> {
        self.ctx
            .model_type_readable_str_lossy()
            .ok()
            .map(|cow| cow.into_owned())
    }

    /// Wrap this engine in a [`crate::WhisperAdapter`]. The default
    /// backend is CPU; consumers can rebuild with
    /// [`crate::WhisperAdapter::new`] if they need cuda/metal/vulkan.
    #[cfg(feature = "engine-adapter")]
    pub fn adapter(self) -> crate::WhisperAdapter {
        let arc = std::sync::Arc::new(self);
        crate::WhisperAdapter::new(
            arc,
            voxora_engine::BackendDescriptor::new(voxora_engine::BackendKind::Cpu),
        )
    }
}

impl AsrEngine for WhisperEngine {
    fn capabilities(&self) -> ModelCapabilities {
        self.capabilities.clone()
    }

    fn transcribe(
        &self,
        samples: &[f32],
        opts: &TranscribeOptions,
    ) -> Result<TranscriptionResult, AsrError> {
        let mut state = self
            .ctx
            .create_state()
            .map_err(|e| AsrError::Inference(format!("create_state: {e}")))?;

        let mut params =
            whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
        params::apply(&mut params, opts, self.ctx.is_multilingual())?;

        // `state.full` consumes `params`, so we cannot reuse it after.
        state
            .full(params, samples)
            .map_err(|e| AsrError::Inference(format!("whisper full: {e}")))?;

        // After full(), capture the detected language if the caller
        // asked for auto-detection. whisper-rs returns the language
        // id (or -1 when no detection ran) via full_lang_id_from_state.
        let detected_language = if opts.language.is_none() {
            let id = state.full_lang_id_from_state();
            if id >= 0 {
                language::iso_code_from_id(id)
            } else {
                None
            }
        } else {
            None
        };

        params::collect_result(&state, opts, detected_language)
    }
}

impl std::fmt::Debug for WhisperEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WhisperEngine")
            .field("model_path", &self.model_path)
            .field("model_type", &self.model_type())
            .field("capabilities", &self.capabilities)
            .finish()
    }
}

/// Build the [`ModelCapabilities`] snapshot advertised by a freshly
/// loaded context. Pulled out so [`WhisperEngine::load`] stays short.
fn build_capabilities(ctx: &whisper_rs::WhisperContext) -> ModelCapabilities {
    let multilingual = ctx.is_multilingual();
    ModelCapabilities::new(
        multilingual,
        // whisper.cpp supports token-level (word) timestamps via
        // `set_token_timestamps(true)` on every multilingual model.
        // English-only checkpoints can technically do it too but
        // it's less useful, so we report `false` for those.
        multilingual,
        false, // streaming is not yet wired into voxora-core's trait
        if multilingual {
            language::known_languages()
        } else {
            vec!["en".to_string()]
        },
    )
}

/// Locate a Whisper model file (`.bin` or `.gguf`) inside a resolved
/// HF model directory. Picks the lexicographically smallest name when
/// multiple candidates exist, so callers get a deterministic result.
#[cfg(feature = "hf")]
fn locate_model_file(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut candidates: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .is_some_and(|ext| ext == "bin" || ext == "gguf")
        })
        .collect();
    candidates.sort();
    candidates.into_iter().next()
}

/// Choose which file inside a [`voxora_traits::ModelDir`] the engine
/// should mmap. Prefers the explicit `entry` recorded by the source
/// (single-file HF requests), falling back to directory scanning via
/// [`locate_model_file`] for whole-repo resolves. Encoding the
/// requested filename into [`voxora_traits::ModelDir::entry`] fixes
/// `airvzxf/telora#79` where lex-sort picked `ggml-base.bin` for a
/// request that asked for `ggml-large-v3.bin`.
#[cfg(feature = "hf")]
fn pick_model_path(dir: &voxora_traits::ModelDir) -> Result<PathBuf, AsrError> {
    if let Some(p) = dir.entry.clone() {
        return Ok(p);
    }
    locate_model_file(&dir.path).ok_or_else(|| {
        AsrError::ModelNotFound(format!(
            "no .bin or .gguf file found in resolved model directory: {}",
            dir.path.display()
        ))
    })
}

/// [`voxora_engine::EngineAdapter`] wrapper around a [`WhisperEngine`].
///
/// Built via [`WhisperEngine::adapter`] when the `engine-adapter`
/// feature is enabled. The adapter reports the same capabilities as
/// the engine, plus the family and backend metadata.
#[cfg(feature = "engine-adapter")]
pub struct WhisperAdapter {
    engine: Arc<WhisperEngine>,
    info: voxora_engine::EngineInfo,
    backend: voxora_engine::BackendDescriptor,
}

#[cfg(feature = "engine-adapter")]
impl WhisperAdapter {
    /// Wrap an existing [`WhisperEngine`] in an adapter. The
    /// `backend` describes the device the engine was loaded with
    /// (typically [`voxora_engine::BackendKind::Cpu`]; `cuda` /
    /// `metal` / `vulkan` if the matching Cargo feature was enabled).
    pub fn new(engine: Arc<WhisperEngine>, backend: voxora_engine::BackendDescriptor) -> Self {
        let capabilities = engine.capabilities();
        let mut info =
            voxora_engine::EngineInfo::new(voxora_engine::EngineFamily::Whisper, capabilities)
                .with_source_path(engine.model_path());
        if let Some(label) = engine.model_type() {
            info = info.with_model_label(label);
        }
        Self {
            engine,
            info,
            backend,
        }
    }

    /// Borrow the wrapped [`WhisperEngine`].
    pub fn engine(&self) -> &WhisperEngine {
        &self.engine
    }
}

#[cfg(feature = "engine-adapter")]
impl voxora_engine::EngineAdapter for WhisperAdapter {
    fn family(&self) -> voxora_engine::EngineFamily {
        voxora_engine::EngineFamily::Whisper
    }

    fn info(&self) -> voxora_engine::EngineInfo {
        self.info.clone()
    }

    fn backend(&self) -> voxora_engine::BackendDescriptor {
        self.backend
    }

    fn as_asr_engine(&self) -> &dyn AsrEngine {
        &*self.engine
    }
}

#[cfg(feature = "engine-adapter")]
impl AsrEngine for WhisperAdapter {
    fn capabilities(&self) -> voxora_traits::ModelCapabilities {
        self.engine.capabilities()
    }

    fn transcribe(
        &self,
        samples: &[f32],
        opts: &TranscribeOptions,
    ) -> Result<TranscriptionResult, AsrError> {
        self.engine.transcribe(samples, opts)
    }
}

#[cfg(feature = "engine-adapter")]
impl std::fmt::Debug for WhisperAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WhisperAdapter")
            .field("family", &voxora_engine::EngineFamily::Whisper)
            .field("backend", &self.backend)
            .field("info", &self.info)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_rejects_missing_file() {
        let err = WhisperEngine::load(Path::new("/nonexistent/ggml-tiny.bin"))
            .expect_err("missing file should error");
        match err {
            AsrError::AudioIo { ref path, .. } => {
                assert_eq!(path, &PathBuf::from("/nonexistent/ggml-tiny.bin"));
            }
            other => panic!("expected AudioIo, got {other:?}"),
        }
    }

    #[test]
    fn load_rejects_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = WhisperEngine::load(dir.path())
            .expect_err("directory should not be accepted as a model file");
        match err {
            AsrError::InvalidInput(msg) => {
                assert!(
                    msg.contains("not a regular file"),
                    "message should explain the failure: {msg}"
                );
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn load_rejects_empty_file() {
        let f = tempfile::NamedTempFile::new().expect("tempfile");
        let err = WhisperEngine::load(f.path())
            .expect_err("empty file should not load as a whisper model");
        // whisper.cpp rejects empty files with an inference error.
        assert!(
            matches!(err, AsrError::Inference(_)),
            "expected Inference error from whisper.cpp, got {err:?}"
        );
    }

    #[cfg(feature = "hf")]
    #[test]
    fn pick_model_path_prefers_entry_over_locate() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path().join("ggml-base.bin");
        let large = dir.path().join("ggml-large-v3.bin");
        std::fs::write(&base, b"base").unwrap();
        std::fs::write(&large, b"large").unwrap();

        let model_dir = voxora_traits::ModelDir::with_entry(
            dir.path().to_path_buf(),
            large.clone(),
            voxora_traits::ModelSourceKind::Local,
            voxora_traits::Quantization::F16,
        );
        assert_eq!(pick_model_path(&model_dir).unwrap(), large);
    }

    #[cfg(feature = "hf")]
    #[test]
    fn pick_model_path_falls_back_to_locate_when_entry_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path().join("ggml-base.bin");
        std::fs::write(&base, b"base").unwrap();

        let model_dir = voxora_traits::ModelDir::new(
            dir.path().to_path_buf(),
            voxora_traits::ModelSourceKind::Local,
            voxora_traits::Quantization::F16,
        );
        assert_eq!(pick_model_path(&model_dir).unwrap(), base);
    }
}
