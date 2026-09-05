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
//! (`voxora-traits`, `voxora-hf`, `voxora-whisper`, `voxora-qwen3asr`)
//! behind a single import path so consumers do not have to depend on
//! four separate crates with four separate feature lists.
//!
//! ## Features
//!
//! | Feature | Re-exports | Enables |
//! |---|---|---|
//! | (none) | `voxora-traits` + `voxora-hf` | traits, types, HF resolver |
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
//! 2. Call `WhisperEngine::from_hf` or `QwenAsrEngine::from_hf`
//!    (each lives behind its engine feature) to resolve a model id
//!    (e.g. `ggerganov/whisper.cpp/ggml-tiny.bin`) into an on-disk
//!    directory and load the engine.
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

// voxora-traits is always re-exported: every consumer needs the
// `AsrEngine` / `ModelSource` traits, regardless of which engine
// adapter they pick.
pub use voxora_traits::{
    AsrEngine, AsrError, ModelCapabilities, ModelDescriptor, ModelDir, ModelSource,
    ModelSourceKind, Quantization, QuantizationPreference, ResolveOptions, TranscribeOptions,
    TranscriptionResult, TranscriptionSegment,
};

// voxora-hf is always re-exported too: the canonical
// `HuggingFaceSource` is the only `ModelSource` implementation we ship
// today, and the bridge constructors (`*Engine::from_hf`) all take it
// as an argument.
pub use voxora_hf::HuggingFaceSource;

// voxora-engine owns the canonical `EngineFamily` enum. Re-export it
// here so consumers that pull in `voxora-bridge` get a single import
// path for the "which ASR engine" decision.
pub use voxora_engine::EngineFamily;

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

/// Library version (matches the workspace).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
