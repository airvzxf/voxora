//! In-memory mocks used by voxora test suites.

use std::path::PathBuf;
use std::sync::Mutex;

use async_trait::async_trait;
use voxora_core::{
    AsrEngine, AsrError, ModelCapabilities, ModelDir, ModelSource, ModelSourceKind, Quantization,
    ResolveOptions, TranscribeOptions, TranscriptionResult,
};

/// In-memory [`ModelSource`] used by tests that don't need real HF.
///
/// `resolve` returns a `ModelDir` whose `path` and `entry` are
/// synthesised from the requested `model_id`. The optional
/// `missing_files` list lets a test simulate "the requested file is
/// not in the cache" (the voxora#79 scenario).
pub struct InMemorySource {
    /// Suffix list: if the synthesised entry filename ends with one
    /// of these strings, [`ModelSource::resolve`] returns
    /// [`AsrError::ModelNotFound`].
    pub missing_files: Mutex<Vec<String>>,
}

impl InMemorySource {
    /// Construct a fresh `InMemorySource` with an empty missing-file
    /// list.
    pub fn new() -> Self {
        Self {
            missing_files: Mutex::new(Vec::new()),
        }
    }

    /// Builder: add `file` to the missing-file list. Any `resolve`
    /// call whose synthesised entry filename ends with `file` will
    /// fail with [`AsrError::ModelNotFound`].
    pub fn with_missing(mut self, file: impl Into<String>) -> Self {
        self.missing_files.get_mut().unwrap().push(file.into());
        self
    }
}

impl Default for InMemorySource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModelSource for InMemorySource {
    fn name(&self) -> &'static str {
        "in-memory"
    }

    async fn resolve(&self, model_id: &str, _opts: &ResolveOptions) -> Result<ModelDir, AsrError> {
        let parts: Vec<&str> = model_id.split('/').collect();
        let leaf = parts.last().copied().unwrap_or("");
        let dir_path = if parts.len() >= 2 {
            let without_leaf = parts[..parts.len() - 1].join("/");
            format!("/fake-cache/{without_leaf}")
        } else {
            format!("/fake-cache/{model_id}")
        };
        let entry_path = if leaf.is_empty() || parts.len() < 2 {
            None
        } else {
            Some(format!("{dir_path}/{leaf}"))
        };
        let missing = self.missing_files.lock().unwrap().clone();
        let entry = entry_path
            .map(PathBuf::from)
            .filter(|p| !missing.iter().any(|m| p.to_string_lossy().ends_with(m)));
        if entry.is_none() && !missing.is_empty() && !leaf.is_empty() {
            return Err(AsrError::ModelNotFound(format!(
                "simulated missing file in cache for {model_id:?}"
            )));
        }
        let path = PathBuf::from(&dir_path);
        let kind = ModelSourceKind::Local;
        let quant = Quantization::F16;
        Ok(match entry {
            Some(e) => ModelDir::with_entry(path, e, kind, quant),
            None => ModelDir::new(path, kind, quant),
        })
    }

    async fn capabilities_for(&self, _model_id: &str) -> Result<ModelCapabilities, AsrError> {
        Ok(ModelCapabilities::UNKNOWN)
    }
}

/// Deterministic [`AsrEngine`] used by tests that need a stable
/// contract without loading a real model.
///
/// `transcribe` returns a fixed string built from the sample count
/// and (optionally) the language option.
pub struct EchoEngine {
    /// Reported [`ModelCapabilities`] (multilingual, word-timestamps,
    /// English language).
    pub capabilities: ModelCapabilities,
    /// Prefix text returned by [`AsrEngine::transcribe`]. Default
    /// is `"echo"`; override via [`EchoEngine::with_text`].
    pub transcribe_text: String,
}

impl EchoEngine {
    /// Build a default `EchoEngine` that reports English +
    /// multilingual + word-timestamps and returns `"echo (N samples)"`
    /// from `transcribe`.
    pub fn new() -> Self {
        Self {
            capabilities: ModelCapabilities::new(true, true, false, vec!["en".into()]),
            transcribe_text: "echo".to_string(),
        }
    }

    /// Builder: override the prefix returned by [`AsrEngine::transcribe`].
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.transcribe_text = text.into();
        self
    }
}

impl Default for EchoEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AsrEngine for EchoEngine {
    fn capabilities(&self) -> ModelCapabilities {
        self.capabilities.clone()
    }

    fn transcribe(
        &self,
        samples: &[f32],
        _opts: &TranscribeOptions,
    ) -> Result<TranscriptionResult, AsrError> {
        Ok(TranscriptionResult::new(format!(
            "{} ({} samples)",
            self.transcribe_text,
            samples.len()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};

    /// Spin-block a future to completion using a no-op waker.
    ///
    /// `InMemorySource::resolve` is `async` only via `async_trait`;
    /// its body has no real await points, so it completes on the
    /// first poll. A real executor is overkill — this keeps the
    /// testkit free of `tokio` / `futures` runtime deps.
    fn block_on<F: Future>(fut: F) -> F::Output {
        struct Noop;
        impl Wake for Noop {
            fn wake(self: Arc<Self>) {}
            fn wake_by_ref(self: &Arc<Self>) {}
        }
        let waker: Waker = Arc::new(Noop).into();
        let mut fut: Pin<Box<F>> = Box::pin(fut);
        let mut ctx = Context::from_waker(&waker);
        loop {
            if let Poll::Ready(v) = fut.as_mut().poll(&mut ctx) {
                return v;
            }
            std::hint::spin_loop();
        }
    }

    #[test]
    fn in_memory_source_synthesises_path_and_entry() {
        let src = InMemorySource::new();
        let dir = block_on(src.resolve("org/repo/file.bin", &ResolveOptions::default())).unwrap();
        assert_eq!(dir.path, PathBuf::from("/fake-cache/org/repo"));
        assert_eq!(
            dir.entry,
            Some(PathBuf::from("/fake-cache/org/repo/file.bin"))
        );
    }

    #[test]
    fn in_memory_source_simulates_missing_file() {
        let src = InMemorySource::new().with_missing("ggml-large-v3.bin");
        let err = block_on(src.resolve(
            "ggerganov/whisper.cpp/ggml-large-v3.bin",
            &ResolveOptions::default(),
        ))
        .expect_err("simulated missing file");
        assert!(matches!(err, AsrError::ModelNotFound(_)));
    }

    #[test]
    fn echo_engine_reports_deterministic_text() {
        let e = EchoEngine::new();
        let r = e
            .transcribe(&[0.0_f32; 8], &TranscribeOptions::default())
            .unwrap();
        assert_eq!(r.text, "echo (8 samples)");
    }

    #[test]
    fn echo_engine_supports_custom_text() {
        let e = EchoEngine::new().with_text("hi");
        let r = e.transcribe(&[], &TranscribeOptions::default()).unwrap();
        assert_eq!(r.text, "hi (0 samples)");
    }
}
