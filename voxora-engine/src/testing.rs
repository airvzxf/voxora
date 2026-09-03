//! Test helpers — fake adapters that implement the [`EngineAdapter`]
//! contract for downstream tests.

use voxora_core::{AsrEngine, AsrError, ModelCapabilities, TranscribeOptions, TranscriptionResult};

use crate::adapter::EngineAdapter;
use crate::backend::{BackendDescriptor, BackendKind};
use crate::family::EngineFamily;
use crate::info::EngineInfo;

/// Minimal adapter used in tests. Delegates `AsrEngine` calls to a
/// deterministic in-memory implementation.
#[derive(Debug, Clone)]
pub struct MockAdapter {
    family: EngineFamily,
    capabilities: ModelCapabilities,
    backend: BackendDescriptor,
}

impl MockAdapter {
    /// Build a CPU-backed, English+Spanish, multilingual mock adapter
    /// for `family`. Use the builder helpers to override the defaults.
    pub fn new(family: EngineFamily) -> Self {
        let capabilities =
            ModelCapabilities::new(true, true, false, vec!["en".into(), "es".into()]);
        Self {
            family,
            capabilities,
            backend: BackendDescriptor::new(BackendKind::Cpu),
        }
    }

    /// Swap the backend kind (e.g. to exercise `cuda` code paths in
    /// the adapter contract).
    pub fn with_backend(mut self, backend: BackendKind) -> Self {
        self.backend = BackendDescriptor::new(backend);
        self
    }

    /// Override the [`ModelCapabilities`] advertised by the mock.
    pub fn with_capabilities(mut self, caps: ModelCapabilities) -> Self {
        self.capabilities = caps;
        self
    }
}

impl AsrEngine for MockAdapter {
    fn capabilities(&self) -> ModelCapabilities {
        self.capabilities.clone()
    }

    fn transcribe(
        &self,
        samples: &[f32],
        opts: &TranscribeOptions,
    ) -> Result<TranscriptionResult, AsrError> {
        Ok(TranscriptionResult::new(format!(
            "mock of {} samples ({:?})",
            samples.len(),
            opts.language
        )))
    }
}

impl EngineAdapter for MockAdapter {
    fn family(&self) -> EngineFamily {
        self.family
    }

    fn info(&self) -> EngineInfo {
        EngineInfo::new(self.family, self.capabilities.clone())
            .with_model_label(format!("mock-{}", self.family.as_config()))
    }

    fn backend(&self) -> BackendDescriptor {
        self.backend
    }

    fn as_asr_engine(&self) -> &dyn AsrEngine {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_reports_its_family() {
        let m = MockAdapter::new(EngineFamily::Whisper);
        assert_eq!(m.family(), EngineFamily::Whisper);
    }

    #[test]
    fn mock_transcribe_returns_deterministic_string() {
        let m = MockAdapter::new(EngineFamily::Qwen3Asr);
        let r = m
            .transcribe(&[0.0_f32; 8], &TranscribeOptions::default())
            .expect("ok");
        assert_eq!(r.text, "mock of 8 samples (None)");
    }

    #[test]
    fn mock_with_backend_reflects_in_adapter() {
        let m = MockAdapter::new(EngineFamily::Whisper).with_backend(BackendKind::Cuda);
        assert_eq!(m.backend().kind, BackendKind::Cuda);
    }
}
