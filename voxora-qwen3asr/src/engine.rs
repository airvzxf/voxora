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
        // Qwen3-ASR's official HF release ships `vocab.json` +
        // `merges.txt` + `tokenizer_config.json` but NOT
        // `tokenizer.json`. Upstream `qwen3_asr::AsrInference::load`
        // expects a `tokenizer.json` file, so we synthesise one here
        // when it's missing. This mirrors what
        // `qwen3_asr::from_pretrained` does internally; we re-do it
        // on the consumer side so callers using voxora-hf directly
        // (i.e. not qwen3-asr's own downloader) get the same
        // treatment.
        ensure_qwen3_tokenizer_json(&dir.path)?;
        Self::load(&dir.path)
    }

    /// Return the directory the engine was loaded from.
    pub fn model_dir(&self) -> &Path {
        &self.model_dir
    }

    /// Wrap this engine in a [`crate::QwenAsrAdapter`]. The default
    /// backend is CPU; consumers can rebuild with
    /// [`crate::QwenAsrAdapter::new`] if they need cuda/metal.
    #[cfg(feature = "engine-adapter")]
    pub fn adapter(self) -> crate::QwenAsrAdapter {
        let arc = Arc::new(self);
        crate::QwenAsrAdapter::new(
            arc,
            voxora_engine::BackendDescriptor::new(voxora_engine::BackendKind::Cpu),
        )
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

/// [`voxora_engine::EngineAdapter`] wrapper around a [`QwenAsrEngine`].
///
/// Built via [`QwenAsrEngine::adapter`] when the `engine-adapter`
/// feature is enabled. The adapter reports the same capabilities as
/// the engine, plus the family and backend metadata.
#[cfg(feature = "engine-adapter")]
pub struct QwenAsrAdapter {
    engine: Arc<QwenAsrEngine>,
    info: voxora_engine::EngineInfo,
    backend: voxora_engine::BackendDescriptor,
}

#[cfg(feature = "engine-adapter")]
impl QwenAsrAdapter {
    /// Wrap an existing [`QwenAsrEngine`] in an adapter. The
    /// `backend` describes the device the engine was loaded with
    /// (typically [`voxora_engine::BackendKind::Cpu`]; `cuda` /
    /// `metal` if the matching Cargo feature was enabled).
    pub fn new(engine: Arc<QwenAsrEngine>, backend: voxora_engine::BackendDescriptor) -> Self {
        let capabilities = engine.capabilities();
        let info =
            voxora_engine::EngineInfo::new(voxora_engine::EngineFamily::Qwen3Asr, capabilities)
                .with_source_path(engine.model_dir());
        Self {
            engine,
            info,
            backend,
        }
    }

    /// Borrow the wrapped [`QwenAsrEngine`].
    pub fn engine(&self) -> &QwenAsrEngine {
        &self.engine
    }
}

#[cfg(feature = "engine-adapter")]
impl voxora_engine::EngineAdapter for QwenAsrAdapter {
    fn family(&self) -> voxora_engine::EngineFamily {
        voxora_engine::EngineFamily::Qwen3Asr
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
impl AsrEngine for QwenAsrAdapter {
    fn capabilities(&self) -> voxora_core::ModelCapabilities {
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
impl std::fmt::Debug for QwenAsrAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QwenAsrAdapter")
            .field("family", &voxora_engine::EngineFamily::Qwen3Asr)
            .field("backend", &self.backend)
            .field("info", &self.info)
            .finish_non_exhaustive()
    }
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

    // The "Debug impl skips the `inference` field" property is
    // structural: the manual `Debug for QwenAsrAdapter` in this module
    // calls `finish_non_exhaustive()`, and `qwen3_asr::AsrInference`
    // does not implement `Debug`. Adding `.field("inference", ...)`
    // to the Debug impl would fail compilation — the invariant is
    // enforced at build time, so no runtime test is necessary.
}

/// Synthesise a `tokenizer.json` inside `model_dir` if it is
/// missing. Qwen3-ASR's official HF release ships
/// `vocab.json` + `merges.txt` + `tokenizer_config.json` but NOT
/// `tokenizer.json`. Upstream `qwen3_asr::AsrInference::load`
/// requires the latter, so we build it here. The logic mirrors
/// what `qwen3_asr::from_pretrained` does internally
/// (`build_qwen3_tokenizer_json` in the upstream crate); we re-do
/// it on the consumer side so callers going through voxora-hf get
/// the same treatment.
#[cfg(feature = "hf")]
fn ensure_qwen3_tokenizer_json(model_dir: &Path) -> Result<(), AsrError> {
    use std::fs;

    let tok_json_path = model_dir.join("tokenizer.json");
    if tok_json_path.is_file() {
        return Ok(());
    }

    let vocab_path = model_dir.join("vocab.json");
    let merges_path = model_dir.join("merges.txt");
    let tok_config_path = model_dir.join("tokenizer_config.json");
    if !vocab_path.is_file() || !merges_path.is_file() || !tok_config_path.is_file() {
        return Err(AsrError::ModelNotFound(format!(
            "Qwen3-ASR cache at {} is missing vocab.json / merges.txt / \
             tokenizer_config.json; cannot synthesise tokenizer.json",
            model_dir.display()
        )));
    }

    let vocab = fs::read_to_string(&vocab_path)
        .map_err(|e| AsrError::Config(format!("failed to read {}: {e}", vocab_path.display())))?;
    let merges = fs::read_to_string(&merges_path)
        .map_err(|e| AsrError::Config(format!("failed to read {}: {e}", merges_path.display())))?;
    let tok_config = fs::read_to_string(&tok_config_path).map_err(|e| {
        AsrError::Config(format!("failed to read {}: {e}", tok_config_path.display()))
    })?;

    let bytes = build_qwen3_tokenizer_json(&vocab, &merges, &tok_config)
        .map_err(|e| AsrError::Config(format!("tokenizer synthesis: {e}")))?;
    fs::write(&tok_json_path, &bytes).map_err(|e| {
        AsrError::Config(format!(
            "failed to write synthesised {}: {e}",
            tok_json_path.display()
        ))
    })?;
    Ok(())
}

/// Mirror of `qwen3_asr::hub::build_qwen3_tokenizer_json`. Kept
/// private inside that crate, so we re-implement it here verbatim.
/// The JSON shape is what `tokenizers::Tokenizer::from_file` expects
/// when reading a BPE-style Qwen3 model. Any drift in the upstream
/// helper will surface as a tokeniser load failure at engine-load
/// time — surfaced via the existing inference-error path.
#[cfg(feature = "hf")]
fn build_qwen3_tokenizer_json(
    vocab: &str,
    merges: &str,
    tok_config: &str,
) -> anyhow::Result<Vec<u8>> {
    use anyhow::Context;

    let vocab_val: serde_json::Value = serde_json::from_str(vocab).context("parse vocab.json")?;
    let merges_vec: Vec<&str> = merges
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .collect();

    let tok_cfg: serde_json::Value =
        serde_json::from_str(tok_config).context("parse tokenizer_config.json")?;
    let mut added_tokens: Vec<serde_json::Value> = Vec::new();
    if let Some(decoder_map) = tok_cfg["added_tokens_decoder"].as_object() {
        let mut entries: Vec<(u64, &serde_json::Value)> = decoder_map
            .iter()
            .filter_map(|(k, v)| k.parse::<u64>().ok().map(|id| (id, v)))
            .collect();
        entries.sort_by_key(|(id, _)| *id);
        for (id, v) in &entries {
            added_tokens.push(serde_json::json!({
                "id": id,
                "content": v["content"],
                "single_word": false,
                "lstrip": false,
                "rstrip": false,
                "normalized": false,
                "special": v["special"],
            }));
        }
    }

    let tokenizer_json = serde_json::json!({
        "version": "1.0",
        "truncation": null,
        "padding": null,
        "added_tokens": added_tokens,
        "normalizer": { "type": "NFC" },
        "pre_tokenizer": {
            "type": "Sequence",
            "pretokenizers": [
                {
                    "type": "Split",
                    "pattern": { "Regex": "(?i:'s|'t|'re|'ve|'m|'ll|'d)|[^\\r\\n\\p{L}\\p{N}]?\\p{L}+|\\p{N}| ?[^\\s\\p{L}\\p{N}]+[\\r\\n]*|\\s*[\\r\\n]+|\\s+(?!\\S)|\\s+" },
                    "behavior": "Isolated",
                    "invert": false,
                },
                {
                    "type": "ByteLevel",
                    "add_prefix_space": false,
                    "trim_offsets": false,
                    "use_regex": false,
                }
            ]
        },
        "post_processor": {
            "type": "ByteLevel",
            "add_prefix_space": false,
            "trim_offsets": false,
            "use_regex": false,
        },
        "decoder": {
            "type": "ByteLevel",
            "add_prefix_space": false,
            "trim_offsets": false,
            "use_regex": false,
        },
        "model": {
            "type": "BPE",
            "dropout": null,
            "unk_token": null,
            "continuing_subword_prefix": "",
            "end_of_word_suffix": "",
            "fuse_unk": false,
            "byte_fallback": false,
            "ignore_merges": false,
            "vocab": vocab_val,
            "merges": merges_vec,
        }
    });

    serde_json::to_vec(&tokenizer_json).context("serialize synthesised tokenizer.json")
}

#[cfg(all(test, feature = "hf"))]
mod synth_tests {
    use super::*;

    #[test]
    fn build_qwen3_tokenizer_json_smoke() {
        let vocab = r#"{"hello":0,"world":1}"#;
        let merges = "#version: 0.2\nh ello\nwor ld\n";
        let tok_config =
            r#"{"added_tokens_decoder":{"151643":{"content":"<|endoftext|>","special":true}}}"#;
        let bytes = build_qwen3_tokenizer_json(vocab, merges, tok_config).expect("synthesise");
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).expect("parse");
        assert_eq!(parsed["version"], "1.0");
        assert_eq!(parsed["model"]["type"], "BPE");
        assert_eq!(parsed["model"]["vocab"]["hello"], 0);
        assert_eq!(parsed["model"]["vocab"]["world"], 1);
        let merges_arr = parsed["model"]["merges"].as_array().expect("merges array");
        assert_eq!(
            merges_arr.len(),
            2,
            "comment + empty lines must be filtered"
        );
        assert_eq!(merges_arr[0], "h ello");
        assert_eq!(merges_arr[1], "wor ld");
        let added = parsed["added_tokens"]
            .as_array()
            .expect("added_tokens array");
        assert_eq!(added.len(), 1);
        assert_eq!(added[0]["content"], "<|endoftext|>");
        assert_eq!(added[0]["special"], true);
        assert_eq!(added[0]["id"], 151643);
    }

    #[test]
    fn ensure_qwen3_tokenizer_json_noop_when_present() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("tokenizer.json"), b"{}").expect("write");
        ensure_qwen3_tokenizer_json(dir.path()).expect("no-op");
        let bytes = std::fs::read(dir.path().join("tokenizer.json")).expect("read");
        assert_eq!(
            bytes, b"{}",
            "existing tokenizer.json must not be overwritten"
        );
    }

    #[test]
    fn ensure_qwen3_tokenizer_json_errors_when_trio_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = ensure_qwen3_tokenizer_json(dir.path()).expect_err("must fail");
        match err {
            AsrError::ModelNotFound(msg) => assert!(
                msg.contains("missing"),
                "expected missing-files message, got: {msg}"
            ),
            other => panic!("expected ModelNotFound, got: {other:?}"),
        }
    }

    #[test]
    fn ensure_qwen3_tokenizer_json_synthesises_from_trio() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("vocab.json"), br#"{"a":0,"b":1}"#).expect("vocab");
        std::fs::write(dir.path().join("merges.txt"), b"a b\n").expect("merges");
        std::fs::write(
            dir.path().join("tokenizer_config.json"),
            br#"{"added_tokens_decoder":{}}"#,
        )
        .expect("tok_config");
        ensure_qwen3_tokenizer_json(dir.path()).expect("synth");
        assert!(dir.path().join("tokenizer.json").is_file());
        let bytes = std::fs::read(dir.path().join("tokenizer.json")).expect("read");
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).expect("parse");
        assert_eq!(parsed["model"]["type"], "BPE");
        assert_eq!(parsed["model"]["vocab"]["a"], 0);
    }
}
