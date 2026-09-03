//! `voxora-core` — shim around [`voxora-traits`] for backwards
//! compatibility.
//!
//! Since voxora 0.3.0, the canonical trait surface lives in
//! [`voxora-traits`]. This crate re-exports everything so callers
//! that still depend on `voxora_core::*` keep compiling unchanged.
//!
//! New code should depend on [`voxora-traits`] directly.
//!
//! [`voxora-traits`]: https://docs.rs/voxora-traits

#![forbid(unsafe_code)]
#![warn(missing_docs)]

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
