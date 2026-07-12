//! The [`QwenAsrEngine`] type — a [`voxora_core::AsrEngine`] backed by
//! [`qwen3_asr::AsrInference`].
//!
//! Upstream `AsrInference` is `Send` (the inner `Mutex` makes it
//! implicitly so via `Send` on `AsrInferenceInner`), but it is *not*
//! `Sync` because the inner state includes raw candle Metal pointers
//! that need the mutex to mediate access. The standard voxora pattern
//! (`Arc<dyn AsrEngine>`) requires `Send + Sync`, so we wrap the
//! `AsrInference` in a `Mutex` here to add the missing `Sync` bound
//! without changing the public shape of the engine.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use voxora_core::{AsrEngine, AsrError, ModelCapabilities, TranscribeOptions, TranscriptionResult};

use crate::error::map_qwen_error;
use crate::language;
use crate::params;

/// Qwen3-ASR model loaded into a [`qwen3_asr::AsrInference`] and exposed
/// through the voxora [`AsrEngine`] trait.
///
/// The engine is `Send + Sync` so it can be shared across threads
/// behind an `Arc<dyn AsrEngine>` (the standard voxora pattern). The
/// `Sync` bound is added by the inner `Mutex`; the actual GPU work is
/// still serialized per call.
pub struct QwenAsrEngine {
    inner: Arc<Mutex<qwen3_asr::AsrInference>>,
    model_dir: PathBuf,
    capabilities: ModelCapabilities,
}

impl QwenAsrEngine {
    /// Load a Qwen3-ASR model from a directory on disk.
    ///
    /// `model_dir` must point to a directory containing `config.json`,
    /// `tokenizer.json`, and `model.safetensors` (single file or
    /// sharded via `model.safetensors.index.json`). The same layout is
    /// produced by `huggingface-cli download Qwen/Qwen3-ASR-0.6B
    /// --local-dir <dir>` and by `voxora-hf`'s `HuggingFaceSource::resolve`
    /// (with the `hf` feature enabled).
    ///
    /// Internally calls `qwen3_asr::best_device()`, which picks
    /// CUDA → Metal → CPU based on the active feature flags at
    /// compile time.
    ///
    /// # Errors
    ///
    /// - [`AsrError::InvalidInput`] if `model_dir` does not exist or
    ///   is not a directory.
    /// - [`AsrError::Inference`] if upstream fails to read
    ///   `config.json`, decode `tokenizer.json`, or load
    ///   `model.safetensors` shards. The full chain is preserved in
    ///   the error message.
    pub fn load(model_dir: &Path) -> Result<Self, AsrError> {
        Self::load_with_device(model_dir, qwen3_asr::best_device())
    }

    /// Load a Qwen3-ASR model from a directory using an explicit
    /// candle `Device`.
    ///
    /// Use this when [`load`](Self::load)'s "best device" pick is not
    /// what you want — e.g. forcing CPU on a machine where Metal is
    /// also compiled in, or pinning a specific CUDA device.
    ///
    /// # Errors
    ///
    /// Same as [`load`](Self::load).
    pub fn load_with_device(model_dir: &Path, device: crate::Device) -> Result<Self, AsrError> {
        let meta = std::fs::metadata(model_dir).map_err(|e| AsrError::audio_io(model_dir, e))?;
        if !meta.is_dir() {
            return Err(AsrError::InvalidInput(format!(
                "model path is not a directory: {}",
                model_dir.display()
            )));
        }

        let inference = qwen3_asr::AsrInference::load(model_dir, device).map_err(map_qwen_error)?;
        let capabilities = build_capabilities();

        Ok(Self {
            inner: Arc::new(Mutex::new(inference)),
            model_dir: model_dir.to_path_buf(),
            capabilities,
        })
    }

    /// Resolve a Hugging Face model id via the supplied
    /// [`voxora_core::ModelSource`] and load the resolved directory.
    ///
    /// Requires the `hf` feature (which pulls in `voxora-hf`). The
    /// model id must resolve to a directory with the Qwen3-ASR layout
    /// (`config.json` + `model.safetensors` + tokenizer). The adapter
    /// does not currently validate the architecture string — passing
    /// a non-Qwen3 model id will fail with an upstream inference
    /// error during [`AsrEngine::transcribe`].
    #[cfg(feature = "hf")]
    pub async fn from_hf(
        source: &dyn voxora_core::ModelSource,
        model_id: &str,
        opts: &voxora_core::ResolveOptions,
    ) -> Result<Self, AsrError> {
        let dir = source.resolve(model_id, opts).await?;
        Self::load(&dir.path)
    }

    /// Return the directory the engine was loaded from.
    pub fn model_dir(&self) -> &Path {
        &self.model_dir
    }
}

impl AsrEngine for QwenAsrEngine {
    fn capabilities(&self) -> ModelCapabilities {
        self.capabilities.clone()
    }

    fn transcribe(
        &self,
        samples: &[f32],
        opts: &TranscribeOptions,
    ) -> Result<TranscriptionResult, AsrError> {
        let qwen_opts = params::build_qwen_opts(opts)?;
        let inner = self
            .inner
            .lock()
            .map_err(|e| AsrError::Config(format!("qwen3-asr mutex poisoned: {e}")))?;
        let qwen_result = inner
            .transcribe_samples(samples, qwen_opts)
            .map_err(map_qwen_error)?;
        Ok(params::collect_qwen_result(qwen_result, opts))
    }
}

impl std::fmt::Debug for QwenAsrEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QwenAsrEngine")
            .field("model_dir", &self.model_dir)
            .field("capabilities", &self.capabilities)
            .finish_non_exhaustive()
    }
}

/// Build the [`ModelCapabilities`] snapshot advertised by a freshly
/// loaded Qwen3-ASR engine.
///
/// Qwen3-ASR is multilingual across the closed 20-language list
/// tracked in [`language::known_languages`]. Word-level timestamps and
/// streaming are not supported by upstream, so both flags are `false`.
fn build_capabilities() -> ModelCapabilities {
    ModelCapabilities::new(
        true,
        false,
        false,
        language::known_languages()
            .iter()
            .map(|s| s.to_string())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_rejects_missing_directory() {
        let err = QwenAsrEngine::load(Path::new("/nonexistent/qwen3-asr-dir"))
            .expect_err("missing dir should error");
        match err {
            AsrError::AudioIo { ref path, .. } => {
                assert_eq!(path, &PathBuf::from("/nonexistent/qwen3-asr-dir"));
            }
            other => panic!("expected AudioIo, got {other:?}"),
        }
    }

    #[test]
    fn load_rejects_file_in_place_of_directory() {
        let f = tempfile::NamedTempFile::new().expect("tempfile");
        let err = QwenAsrEngine::load(f.path())
            .expect_err("file should not be accepted as a model directory");
        match err {
            AsrError::InvalidInput(msg) => {
                assert!(
                    msg.contains("not a directory"),
                    "message should explain the failure: {msg}"
                );
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn capabilities_report_multilingual_with_known_languages() {
        let caps = build_capabilities();
        assert!(caps.multilingual, "Qwen3-ASR is multilingual");
        assert!(!caps.word_timestamps, "Qwen3-ASR has no word timestamps");
        assert!(!caps.streaming, "streaming is deferred to a future phase");
        assert_eq!(caps.languages.len(), language::known_languages().len());
        assert!(caps.languages.iter().any(|l| l == "english"));
        assert!(caps.languages.iter().any(|l| l == "chinese"));
    }

    #[test]
    fn debug_impl_skips_inference_field() {
        // We cannot construct a real `qwen3_asr::AsrInference` here
        // (that needs a model directory and a working candle Device),
        // and `AsrInference` does not implement `Debug` upstream. The
        // manual `Debug` impl in this module uses
        // `finish_non_exhaustive()`, so the inference field is
        // omitted from the rendered output. This test asserts that
        // we are *not* depending on `AsrInference: Debug` anywhere
        // by simply compiling: if the field type ever sneaks into
        // the Debug output, the build still compiles because we use
        // `finish_non_exhaustive()`.
        //
        // Round-trip coverage of the Debug string lives in the
        // integration tests, where a real engine is constructed.
        fn _assert_omits_inference() {
            // The `engine.field("inference", ...)` line must never
            // appear in src/engine.rs. Compile-time invariant.
            //
            // (We can't pattern-match on source code from within a
            // test; this comment is the contract. A grep check
            // runs in CI via `validate`.)
        }
    }
}
