//! whisper.cpp engine adapter for voxora.
//!
//! This crate implements [`voxora_core::AsrEngine`] against the
//! [`whisper-rs`](https://crates.io/crates/whisper-rs) bindings for
//! [`whisper.cpp`](https://github.com/ggerganov/whisper.cpp). It loads
//! a Whisper GGML model file and exposes a synchronous
//! `Send + Sync` engine that any downstream application can hold
//! behind an `Arc<dyn AsrEngine>`.
//!
//! # Loading a model
//!
//! The primary constructor is [`WhisperEngine::load`], which takes a
//! path to a `.bin` model file on disk:
//!
//! ```no_run
//! use std::path::Path;
//! use voxora_core::{AsrEngine, TranscribeOptions};
//! use voxora_whisper::WhisperEngine;
//!
//! # fn run() -> Result<(), voxora_core::AsrError> {
//! let engine = WhisperEngine::load(Path::new("models/ggml-tiny.bin"))?;
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
//! Mirrors the upstream `whisper-rs` features, one per hardware
//! backend. Exactly one is implicitly active in any given build.
//!
//! | Flag | Backend | Default |
//! |---|---|---|
//! | `cpu` | CPU (no GPU acceleration) | yes |
//! | `metal` | Apple Metal (macOS) | no |
//! | `cuda` | NVIDIA CUDA | no |
//! | `vulkan` | Vulkan (AMD / Intel GPU) | no |
//!
//! Pick one with `cargo build -p voxora-whisper --features metal`
//! (or `cuda` / `vulkan`). The default `cpu` keeps the build portable
//! for CI and for users without GPU drivers installed.
//!
//! # Hugging Face integration (optional)
//!
//! With the `hf` feature enabled, `WhisperEngine::from_hf` resolves a
//! Hugging Face model id (e.g. `ggerganov/whisper.cpp`) via
//! `voxora_hf::HuggingFaceSource` and loads the first `.bin` file in
//! the resolved directory:
//!
//! <details><summary>Show example (requires `hf` feature)</summary>
//!
//! ```ignore
//! # #[cfg(feature = "hf")]
//! # async fn run() -> Result<(), voxora_core::AsrError> {
//! use voxora_core::ResolveOptions;
//! use voxora_hf::HuggingFaceSource;
//! use voxora_whisper::WhisperEngine;
//!
//! let source = HuggingFaceSource::new()?;
//! let engine = WhisperEngine::from_hf(
//!     &source,
//!     "ggerganov/whisper.cpp",
//!     &ResolveOptions::default(),
//! ).await?;
//! # let _ = engine; Ok(()) }
//! ```
//!
//! </details>

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod engine;
mod language;
mod params;

pub use engine::WhisperEngine;
pub use language::{iso_code_from_id, known_languages, validate_lang};
