#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Core traits and types for the voxora ASR bridge.
//!
//! This crate defines the public surface every other voxora crate builds on:
//! [`AsrEngine`] for inference and [`ModelSource`] for model acquisition.
//! It has **no engine implementations and no network code** — keeping it
//! free of `reqwest` / `tokio` / `http` so it builds offline and pulls in
//! only `async-trait`, `thiserror`, and (optionally) `serde`.
//!
//! # Design
//!
//! The split between [`AsrEngine`] and [`ModelSource`] is intentional:
//!
//! - [`AsrEngine`] is **synchronous** because inference on a single audio
//!   buffer is a CPU/GPU task; wrapping it in `async` would only add cost.
//! - [`ModelSource`] is **asynchronous** because downloads are I/O-bound.
//!
//! Both traits require `Send + Sync` so engines can be shared across
//! threads behind an `Arc<dyn AsrEngine>` (the standard pattern).
//!
//! # Example
//!
//! ```rust
//! use std::sync::Arc;
//! use voxora_core::{AsrEngine, ModelCapabilities, TranscribeOptions, TranscriptionResult};
//!
//! struct DummyEngine;
//!
//! impl AsrEngine for DummyEngine {
//!     fn capabilities(&self) -> ModelCapabilities {
//!         ModelCapabilities::UNKNOWN
//!     }
//!
//!     fn transcribe(
//!         &self,
//!         _samples: &[f32],
//!         _opts: &TranscribeOptions,
//!     ) -> Result<TranscriptionResult, voxora_core::AsrError> {
//!         Ok(TranscriptionResult::new(""))
//!     }
//! }
//!
//! let engine: Arc<dyn AsrEngine> = Arc::new(DummyEngine);
//! let caps = engine.capabilities();
//! assert!(!caps.multilingual);
//! ```

pub mod engine;
pub mod error;
pub mod source;

pub use engine::{
    AsrEngine, ModelCapabilities, TranscribeOptions, TranscriptionResult, TranscriptionSegment,
};
pub use error::AsrError;
pub use source::{
    ModelDescriptor, ModelDir, ModelSource, ModelSourceKind, Quantization, QuantizationPreference,
    ResolveOptions,
};
