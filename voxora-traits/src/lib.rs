//! Canonical traits and value types for the voxora model-agnostic ASR
//! bridge.
//!
//! This crate is the canonical home of the public API surface. It has
//! zero runtime dependencies (no `tokio`, no `reqwest`, no `async-trait`
//! requirement beyond the trait-level use of `async-trait`).
//!
//! ## Relationship with `voxora-core`
//!
//! `voxora-core` (since 0.3.0) is a thin shim that re-exports this
//! crate for backwards compatibility. New code should depend on
//! `voxora-traits` directly.
//!
//! ## ASR-specific
//!
//! Per operator direction, the design is ASR-specific. The
//! [`AsrEngine`] trait is purpose-built for automatic speech
//! recognition. New domains (LLM, vision, multimodal) are out of
//! scope per the architecture handoff.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod engine;
pub mod error;
pub mod source;
pub mod streaming;

pub use engine::{
    AsrEngine, ModelCapabilities, TranscribeOptions, TranscriptionResult, TranscriptionSegment,
};
pub use error::AsrError;
pub use source::{
    ModelDescriptor, ModelDir, ModelSource, ModelSourceKind, Quantization, QuantizationPreference,
    ResolveOptions,
};
pub use streaming::{StreamingAsrEngine, StreamingOptions, StreamingResult, StreamingSession};

#[cfg(test)]
mod tests {
    #[test]
    fn public_surface_round_trip() {
        let _: crate::ModelCapabilities = crate::ModelCapabilities::default();
    }
}
