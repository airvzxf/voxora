//! qwen3-asr-rs engine adapter for voxora.
//!
//! This crate implements [`voxora_core::AsrEngine`] against the
//! [`qwen3-asr`](https://crates.io/crates/qwen3-asr) bindings for
//! [Qwen3-ASR](https://huggingface.co/Qwen/Qwen3-ASR-0.6B), a
//! candle-native multilingual ASR model that supports English, Chinese,
//! and code-switched audio. It loads a model directory produced by
//! `huggingface-cli download` (or by `voxora-hf`'s `HuggingFaceSource`
//! with the `hf` feature) and exposes a synchronous `Send + Sync`
//! engine that any downstream application can hold behind an
//! `Arc<dyn AsrEngine>`.
//!
//! # Loading a model
//!
//! The primary constructor is [`QwenAsrEngine::load`], which takes a
//! path to a directory containing `config.json`, `tokenizer.json`, and
//! `model.safetensors` (single file or sharded via
//! `model.safetensors.index.json`):
//!
//! ```no_run
//! use std::path::Path;
//! use voxora_core::{AsrEngine, TranscribeOptions};
//! use voxora_qwen3asr::QwenAsrEngine;
//!
//! # fn run() -> Result<(), voxora_core::AsrError> {
//! let engine = QwenAsrEngine::load(Path::new("models/Qwen3-ASR-0.6B"))?;
//! let caps = engine.capabilities();
//! println!("multilingual: {}", caps.multilingual);
//!
//! let samples: Vec<f32> = vec![0.0; 16_000]; // 1 s of silence @ 16 kHz
//! let result = engine.transcribe(&samples, &TranscribeOptions::default())?;
//! println!("{}", result.text);
//! # Ok(()) }
//! ```
//!
//! # Feature flags
//!
//! Mirrors the upstream `qwen3-asr` features, one per hardware backend.
//!
//! | Flag | Backend | Default |
//! |---|---|---|
//! | `cpu` | CPU (no GPU acceleration) | yes |
//! | `metal` | Apple Metal (macOS) | no |
//! | `cuda` | NVIDIA CUDA | no |
//!
//! Pick one with `cargo build -p voxora-qwen3asr --features metal`
//! (or `cuda`). The default `cpu` keeps the build portable for CI and
//! for users without GPU drivers installed.
//!
//! # Device selection
//!
//! [`QwenAsrEngine::load`] calls `qwen3_asr::best_device()` internally,
//! which at compile time picks CUDA → Metal → CPU in that order based
//! on the active feature flags. For explicit control, use
//! [`QwenAsrEngine::load_with_device`], which accepts a
//! `candle_core::Device` re-exported as [`Device`].
//!
//! # CUDA BF16 passthrough
//!
//! Upstream `qwen3-asr` reads the `QWEN3_ASR_CUDA_NATIVE_BF16` env var
//! at load time to decide whether to keep BF16 weights on CUDA (useful
//! on `sm_80+` hardware for benchmarking). voxora does not touch this
//! variable; whatever the process env says when [`QwenAsrEngine::load`]
//! is called is what `qwen3-asr` will see. Set it before invoking the
//! adapter to opt in.
//!
//! # Language codes
//!
//! Unlike [`voxora_whisper`](crate) (which uses ISO 639-1 codes like
//! `"en"`), Qwen3-ASR expects full English language names — e.g.
//! `"english"`, `"chinese"`, `"cantonese"`. The list is closed; see
//! [`language::known_languages`] for the canonical 20 entries.
//! [`validate_lang`] rejects anything outside that set as
//! [`voxora_core::AsrError::InvalidInput`].
//!
//! # Hugging Face integration (optional)
//!
//! With the `hf` feature enabled, `QwenAsrEngine::from_hf` resolves a
//! Hugging Face model id (e.g. `Qwen/Qwen3-ASR-0.6B`) via
//! `voxora-hf`'s `HuggingFaceSource` and loads the resolved directory:
//!
//! <details><summary>Show example (requires `hf` feature)</summary>
//!
//! ```ignore
//! # #[cfg(feature = "hf")]
//! # async fn run() -> Result<(), voxora_core::AsrError> {
//! use voxora_core::ResolveOptions;
//! # #[cfg(feature = "hf")]
//! use voxora_hf::HuggingFaceSource;
//! use voxora_qwen3asr::QwenAsrEngine;
//!
//! # #[cfg(feature = "hf")]
//! let source = HuggingFaceSource::new()?;
//! # #[cfg(feature = "hf")]
//! let engine = QwenAsrEngine::from_hf(
//!     &source,
//!     "Qwen/Qwen3-ASR-0.6B",
//!     &ResolveOptions::default(),
//! ).await?;
//! # let _ = engine; Ok(()) }
//! ```
//!
//! </details>

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod engine;
mod error;
mod language;
mod params;

pub use engine::QwenAsrEngine;
pub use error::map_qwen_error;
pub use language::{known_languages, validate_lang};

/// Re-export of `candle_core::Device` for callers that want explicit
/// device control via [`QwenAsrEngine::load_with_device`].
pub use candle_core::Device;
