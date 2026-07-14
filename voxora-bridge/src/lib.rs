//! `voxora-bridge` — the umbrella crate for the voxora model-agnostic
//! ASR bridge.
//!
//! Downstream consumers (Telora's daemon, third-party STT applications)
//! depend on this single crate and pick which engines they want through
//! Cargo features:
//!
//! ```toml
//! [dependencies]
//! voxora-bridge = { path = "../voxora-bridge", default-features = false, features = ["whisper", "qwen3asr"] }
//! ```
//!
//! This crate is a **pure re-exporter**. It owns no logic; it just
//! glues the four upstream crates
//! (`voxora-core`, `voxora-hf`, `voxora-whisper`, `voxora-qwen3asr`)
//! behind a single import path so consumers do not have to depend on
//! four separate crates with four separate feature lists.
//!
//! ## Features
//!
//! | Feature | Re-exports | Enables |
//! |---|---|---|
//! | (none) | `voxora-core` + `voxora-hf` | traits, types, HF resolver |
//! | `whisper` (default) | `voxora-whisper` | `WhisperEngine`, ggml models |
//! | `qwen3asr` (default) | `voxora-qwen3asr` | `QwenAsrEngine`, candle-native Qwen3-ASR |
//!
//! Defaults are `["whisper", "qwen3asr"]` so the happy-path consumer
//! (`telora-daemon`) gets both engines wired up with one line. Set
//! `default-features = false` to slim down to a single backend.
//!
//! ## Example: load a Whisper model from Hugging Face
//!
//! The flow every consumer (Telora included) follows:
//!
//! 1. Build a [`HuggingFaceSource`] pointing at a cache directory.
//! 2. Call [`WhisperEngine::from_hf`] or [`QwenAsrEngine::from_hf`]
//!    to resolve a model id (e.g. `ggerganov/whisper.cpp/ggml-tiny.bin`)
//!    into an on-disk directory and load the engine.
//! 3. Hold the engine behind `Arc<dyn AsrEngine>` and call
//!    [`AsrEngine::transcribe`] on incoming audio.
//!
//! See `examples/bridge_demo.rs` for the full code.
//!
//! ## License compatibility
//!
//! AGPL-3 downstream consumers (Telora) depend on this Apache-2.0
//! crate. AGPL-3 §5 explicitly permits AGPL works to depend on
//! non-copyleft libraries without propagating copyleft to those
//! libraries.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

// voxora-core is always re-exported: every consumer needs the
// `AsrEngine` / `ModelSource` traits, regardless of which engine
// adapter they pick.
pub use voxora_core::{
    AsrEngine, AsrError, ModelCapabilities, ModelDescriptor, ModelDir, ModelSource,
    ModelSourceKind, Quantization, QuantizationPreference, ResolveOptions, TranscribeOptions,
    TranscriptionResult, TranscriptionSegment,
};

// voxora-hf is always re-exported too: the canonical
// `HuggingFaceSource` is the only `ModelSource` implementation we ship
// today, and the bridge constructors (`*Engine::from_hf`) all take it
// as an argument.
pub use voxora_hf::HuggingFaceSource;

// Engine adapters are gated. Each block keeps its re-exports behind
// `#[cfg(feature = "...")]` so a single-engine binary does not pull
// the other engine's transitive deps (candle vs. whisper.cpp).
#[cfg(feature = "whisper")]
pub use voxora_whisper::WhisperEngine;

#[cfg(feature = "qwen3asr")]
pub use voxora_qwen3asr::{QwenAsrEngine, known_languages, validate_lang};

/// Re-export of `candle_core::Device`, available only with the
/// `qwen3asr` feature (it comes from `qwen3-asr`'s transitive deps).
#[cfg(feature = "qwen3asr")]
pub use voxora_qwen3asr::Device;

/// Engine family the bridge loads.
///
/// Consumers pick one at construction time; the bridge then resolves
/// the appropriate engine adapter behind `Arc<dyn AsrEngine>` and
/// applies engine-specific language-code translation. The
/// string forms are the canonical spellings used in config files:
///
/// | Variant | Config spelling | Engine adapter |
/// |---|---|---|
/// | `Whisper` | `"whisper"` | `voxora-whisper` (whisper.cpp) |
/// | `Qwen3Asr` | `"qwen3-asr"` | `voxora-qwen3asr` (candle) |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelKind {
    /// whisper.cpp family (`voxora-whisper`).
    Whisper,
    /// Qwen3-ASR (`voxora-qwen3asr`).
    Qwen3Asr,
}

impl ModelKind {
    /// Parse the canonical config spelling. Accepts both
    /// `"qwen3-asr"` and the legacy `"qwen3asr"` /
    /// `"qwen3_asr"` aliases for ergonomics.
    pub fn from_config(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "whisper" => Some(Self::Whisper),
            "qwen3-asr" | "qwen3asr" | "qwen3_asr" => Some(Self::Qwen3Asr),
            _ => None,
        }
    }

    /// Canonical config spelling (matches `voxora-cli --engine`).
    pub fn as_config(self) -> &'static str {
        match self {
            Self::Whisper => "whisper",
            Self::Qwen3Asr => "qwen3-asr",
        }
    }
}

impl std::fmt::Display for ModelKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_config())
    }
}

impl std::str::FromStr for ModelKind {
    type Err = InvalidModelKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_config(s).ok_or_else(|| InvalidModelKind(s.to_string()))
    }
}

/// Error returned by the [`std::str::FromStr`] impl on [`ModelKind`]
/// when the input does not match a known engine family.
#[derive(Debug, thiserror::Error)]
#[error("unknown model_kind {0:?}; expected one of `whisper` or `qwen3-asr`")]
pub struct InvalidModelKind(pub String);

/// Library version (matches the workspace).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
