//! The [`MiniMaxEngine`] type, a [`voxora_traits::AsrEngine`]
//! backed by the MiniMax hosted `/v1/speech_to_text` API.
//!
//! Wraps an HTTP client (the `ureq::Agent`, bearer-token header,
//! endpoint, and model) behind an `Arc` so the engine is
//! `Send` plus `Sync` — the standard voxora pattern for
//! `Arc<dyn AsrEngine>`. The bearer token and the `ureq::Agent`
//! live in the private client struct; the engine owns a clone
//! for each `transcribe()` call (cheap, since the client is just
//! three `String`s and an `Agent`). The client itself is
//! intentionally kept crate-private; callers build engines, not
//! clients.

use std::sync::Arc;

use voxora_traits::{AsrEngine, AsrError, ModelCapabilities, TranscribeOptions, TranscriptionResult};

use crate::client::MiniMaxClient;
use crate::config::MiniMaxConfig;
use crate::language::known_languages_bcp47;
use crate::params;
use crate::wav;

/// Default sample rate the engine assumes when converting
/// segment boundaries from seconds to samples. Documented in the
/// crate root's "Sample-rate contract" section.
const ASSUMED_SAMPLE_RATE: u32 = 16_000;

/// Default channel count for the WAV payload.
const ASSUMED_CHANNELS: u16 = 1;

/// MiniMax hosted ASR engine — implements [`voxora_traits::AsrEngine`]
/// against the crate-private HTTP client wrapper.
///
/// `Send + Sync` via the inner `Arc<MiniMaxClient>` so the engine
/// can be shared across threads behind the standard
/// `Arc<dyn AsrEngine>` pattern.
#[derive(Clone)]
pub struct MiniMaxEngine {
    // The client type is `pub` inside the `client` module but
    // the module itself is private, so `MiniMaxClient` is not
    // reachable from outside this crate. We hold it via `Arc`
    // for `Send + Sync` sharing — `Arc<dyn Any>` would require
    // an extra trait, and the cost of holding a typed `Arc` is
    // negligible (the inner client is three `String`s + an
    // `Agent`).
    client: Arc<MiniMaxClient>,
    config: Arc<MiniMaxConfig>,
    capabilities: ModelCapabilities,
}

impl std::fmt::Debug for MiniMaxEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MiniMaxEngine")
            .field("client", &self.client)
            .field("capabilities", &self.capabilities)
            .finish_non_exhaustive()
    }
}

impl MiniMaxEngine {
    /// Build a [`MiniMaxEngine`] from a [`MiniMaxConfig`]. Pure
    /// — no round-trip; the first HTTP call happens on
    /// [`AsrEngine::transcribe`].
    pub fn new(config: MiniMaxConfig) -> Result<Self, AsrError> {
        let client = MiniMaxClient::new(&config);
        let capabilities = build_capabilities();
        Ok(Self {
            client: Arc::new(client),
            config: Arc::new(config),
            capabilities,
        })
    }

    /// Borrow the underlying [`MiniMaxConfig`].
    pub fn config(&self) -> &MiniMaxConfig {
        &self.config
    }

    /// Wrap this engine in a [`crate::MiniMaxAdapter`]. Built via
    /// the `engine-adapter` feature.
    #[cfg(feature = "engine-adapter")]
    #[cfg_attr(docsrs, doc(cfg(feature = "engine-adapter")))]
    pub fn adapter(self) -> crate::MiniMaxAdapter {
        let arc = Arc::new(self);
        crate::MiniMaxAdapter::new(arc)
    }
}

impl AsrEngine for MiniMaxEngine {
    fn capabilities(&self) -> ModelCapabilities {
        self.capabilities.clone()
    }

    fn transcribe(
        &self,
        samples: &[f32],
        opts: &TranscribeOptions,
    ) -> Result<TranscriptionResult, AsrError> {
        let params = params::apply(opts)?;
        let wav_bytes = wav::write_wav_pcm_f32(samples, ASSUMED_SAMPLE_RATE, ASSUMED_CHANNELS);
        let resp = self.client.transcribe(&wav_bytes, &params)?;
        Ok(params::collect_result(resp, opts))
    }
}

/// Build the [`ModelCapabilities`] snapshot advertised by
/// [`MiniMaxEngine::capabilities`]. Pulled out so the constructor
/// stays short.
fn build_capabilities() -> ModelCapabilities {
    ModelCapabilities::new(
        // MiniMax supports 20 BCP-47 tags.
        true,
        // `verbose_json` returns `segments[]` with start/end
        // timestamps; the engine surfaces them as
        // `TranscriptionSegment`s.
        true,
        // Streaming (`StreamingAsrEngine`) is deferred — SSE
        // streaming is well-defined upstream but adding it
        // requires `async-trait`. The capability table flags
        // `false` until a future release wires it in.
        false,
        known_languages_bcp47()
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    )
}

/// [`voxora_engine::EngineAdapter`] wrapper around a
/// [`MiniMaxEngine`].
///
/// Built via [`MiniMaxEngine::adapter`] when the `engine-adapter`
/// feature is enabled. The adapter reports the same capabilities
/// as the engine, plus the family metadata.
#[cfg(feature = "engine-adapter")]
#[cfg_attr(docsrs, doc(cfg(feature = "engine-adapter")))]
pub struct MiniMaxAdapter {
    engine: Arc<MiniMaxEngine>,
    info: voxora_engine::EngineInfo,
    backend: voxora_engine::BackendDescriptor,
}

#[cfg(feature = "engine-adapter")]
impl MiniMaxAdapter {
    /// Wrap an existing [`MiniMaxEngine`] in an adapter. The
    /// `backend` describes the device the request is processed on
    /// — MiniMax compute happens server-side, so the descriptor
    /// is informational (a CPU marker on the consumer side).
    pub fn new(engine: Arc<MiniMaxEngine>) -> Self {
        let capabilities = engine.capabilities();
        let info =
            voxora_engine::EngineInfo::new(voxora_engine::EngineFamily::MiniMax, capabilities)
                // No `with_source_path` — MiniMax is hosted, not local.
                .with_model_label(engine.config().model().to_string());
        Self {
            engine,
            info,
            backend: voxora_engine::BackendDescriptor::CPU,
        }
    }

    /// Borrow the wrapped [`MiniMaxEngine`].
    pub fn engine(&self) -> &MiniMaxEngine {
        &self.engine
    }
}

#[cfg(feature = "engine-adapter")]
impl voxora_engine::EngineAdapter for MiniMaxAdapter {
    fn family(&self) -> voxora_engine::EngineFamily {
        voxora_engine::EngineFamily::MiniMax
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
impl AsrEngine for MiniMaxAdapter {
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
impl std::fmt::Debug for MiniMaxAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MiniMaxAdapter")
            .field("family", &voxora_engine::EngineFamily::MiniMax)
            .field("backend", &self.backend)
            .field("info", &self.info)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_report_multilingual_with_known_languages() {
        let caps = build_capabilities();
        assert!(caps.multilingual, "MiniMax is multilingual");
        assert!(caps.word_timestamps, "verbose_json returns segments[]");
        assert!(!caps.streaming, "streaming is deferred to a future release");
        assert_eq!(caps.languages.len(), known_languages_bcp47().len());
        assert!(caps.languages.iter().any(|l| l == "en"));
        assert!(caps.languages.iter().any(|l| l == "zh"));
    }

    #[test]
    fn new_rejects_empty_token_via_config() {
        // `MiniMaxConfig::new("")` already errors; the engine
        // forwards that error through.
        let cfg_err = MiniMaxConfig::new("").expect_err("empty token rejected");
        assert!(matches!(
            cfg_err,
            crate::config::MiniMaxConfigError::EmptyApiKey
        ));
    }

    #[test]
    fn engine_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MiniMaxEngine>();
    }

    #[test]
    fn engine_is_clone() {
        let cfg = MiniMaxConfig::new("sk-test").unwrap();
        let engine = MiniMaxEngine::new(cfg).unwrap();
        let _clone = engine.clone();
    }
}
