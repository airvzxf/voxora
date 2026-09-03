//! [`EngineAdapter`] — the canonical contract every voxora engine
//! crate implements.
//!
//! ## Streaming
//!
//! [`EngineAdapter::as_streaming_engine`] lets an adapter opt in to
//! [`voxora_traits::StreamingAsrEngine`]. The default implementation
//! returns `None` because no engine in the workspace implements
//! streaming yet — both `voxora-whisper` and `voxora-qwen3asr` are
//! whole-buffer only. Future engines (parakeet, voxtral, …) will
//! override the method to expose incremental decoding once their
//! upstream stacks (whisper-rs, candle) gain streaming APIs.

use std::sync::Arc;

use voxora_traits::{AsrEngine, StreamingAsrEngine};

use crate::backend::BackendDescriptor;
use crate::family::EngineFamily;
use crate::info::EngineInfo;

/// Adapter trait implemented by every voxora engine crate.
///
/// Wraps a concrete [`AsrEngine`] with metadata (`info`) and a
/// [`BackendDescriptor`] so consumers can introspect the engine
/// without per-engine special-casing.
///
/// ASR-specific: the adapter wraps an `AsrEngine`. There is no
/// generic `Model` trait at this layer.
pub trait EngineAdapter: Send + Sync {
    /// Which engine family this adapter represents.
    fn family(&self) -> EngineFamily;

    /// Static metadata about the loaded engine.
    fn info(&self) -> EngineInfo;

    /// Hardware backend the engine was loaded with.
    fn backend(&self) -> BackendDescriptor;

    /// Borrow the underlying [`AsrEngine`] as a trait object.
    ///
    /// Implementors should return `&self.inner` where `inner`
    /// already implements `AsrEngine + Send + Sync`.
    fn as_asr_engine(&self) -> &dyn AsrEngine;

    /// Returns `Some(&dyn StreamingAsrEngine)` if the underlying
    /// engine supports streaming, `None` otherwise.
    ///
    /// Default implementation returns `None` because no engine in
    /// the workspace implements streaming today. Engines that do
    /// support incremental decoding override this to expose their
    /// [`StreamingAsrEngine`] implementation.
    fn as_streaming_engine(&self) -> Option<&dyn StreamingAsrEngine> {
        None
    }
}

/// Type-erased wrapper around an [`EngineAdapter`].
///
/// Lets `voxora-registry` and `voxora-bridge` store heterogeneous
/// engines behind a single type. The wrapper is `Clone` because the
/// inner `Arc` is.
#[derive(Clone)]
pub struct AnyEngine {
    inner: Arc<dyn EngineAdapter>,
}

impl AnyEngine {
    /// Wrap an adapter behind `Arc` and return a `Clone`-able handle.
    pub fn new<A: EngineAdapter + 'static>(adapter: A) -> Self {
        Self {
            inner: Arc::new(adapter),
        }
    }

    /// Engine family of the wrapped adapter.
    pub fn family(&self) -> EngineFamily {
        self.inner.family()
    }

    /// Static metadata for the wrapped engine.
    pub fn info(&self) -> EngineInfo {
        self.inner.info()
    }

    /// Backend the wrapped engine was loaded with.
    pub fn backend(&self) -> BackendDescriptor {
        self.inner.backend()
    }

    /// Borrow the underlying ASR engine trait object.
    pub fn as_asr_engine(&self) -> &dyn AsrEngine {
        self.inner.as_asr_engine()
    }

    /// Borrow the streaming engine trait object when the underlying
    /// adapter advertises one. Returns `None` for whole-buffer-only
    /// engines (today: every engine in the workspace).
    pub fn as_streaming_engine(&self) -> Option<&dyn StreamingAsrEngine> {
        self.inner.as_streaming_engine()
    }

    /// Borrow the inner adapter trait object.
    pub fn as_engine_adapter(&self) -> &dyn EngineAdapter {
        &*self.inner
    }
}

impl std::fmt::Debug for AnyEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnyEngine")
            .field("family", &self.inner.family())
            .field("backend", &self.inner.backend())
            .finish_non_exhaustive()
    }
}

impl AsrEngine for AnyEngine {
    fn capabilities(&self) -> voxora_traits::ModelCapabilities {
        self.inner.as_asr_engine().capabilities()
    }

    fn transcribe(
        &self,
        samples: &[f32],
        opts: &voxora_traits::TranscribeOptions,
    ) -> Result<voxora_traits::TranscriptionResult, voxora_traits::AsrError> {
        self.inner.as_asr_engine().transcribe(samples, opts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::MockAdapter;
    use voxora_traits::{AsrEngine, TranscribeOptions};

    #[test]
    fn any_engine_dispatches_to_inner() {
        let adapter = MockAdapter::new(EngineFamily::Whisper);
        let any = AnyEngine::new(adapter);
        assert_eq!(any.family(), EngineFamily::Whisper);

        let result = any
            .transcribe(&[0.0_f32; 4], &TranscribeOptions::default())
            .expect("transcribe");
        assert!(result.text.contains("mock"));
    }

    #[test]
    fn any_engine_capabilities_match_inner() {
        let adapter = MockAdapter::new(EngineFamily::Qwen3Asr);
        let any = AnyEngine::new(adapter);
        let caps = any.capabilities();
        assert!(caps.multilingual);
    }

    #[test]
    fn any_engine_is_clone() {
        let any = AnyEngine::new(MockAdapter::new(EngineFamily::Whisper));
        let any2 = any.clone();
        assert_eq!(any.family(), any2.family());
    }

    #[test]
    fn as_engine_adapter_borrows_inner() {
        let any = AnyEngine::new(MockAdapter::new(EngineFamily::Whisper));
        let _adapter: &dyn EngineAdapter = any.as_engine_adapter();
    }

    #[test]
    fn as_streaming_engine_defaults_to_none() {
        let any = AnyEngine::new(MockAdapter::new(EngineFamily::Whisper));
        assert!(
            any.as_streaming_engine().is_none(),
            "no engine in the workspace implements streaming yet"
        );
    }

    #[test]
    fn mock_adapter_does_not_advertise_streaming() {
        let adapter = MockAdapter::new(EngineFamily::Qwen3Asr);
        assert!(adapter.as_streaming_engine().is_none());
    }
}
