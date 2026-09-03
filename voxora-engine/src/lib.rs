//! Canonical adapter contract for voxora engine crates (ASR-specific).
//!
//! Every `voxora-<engine>` crate (voxora-whisper, voxora-qwen3asr,
//! future voxora-parakeet, voxora-voxtral, …) builds on top of this
//! crate so consumers see a uniform shape.
//!
//! Owns nothing engine-specific; only the dispatch and metadata glue
//! that is identical across engines.
//!
//! ASR-specific: the adapter wraps an [`voxora_core::AsrEngine`].
//! There is no generic `Model` trait at this layer.
//!
//! The next planned engine family (parakeet, voxtral, …) lands as a
//! new variant on [`EngineFamily`] in 0.2.0; consumers that
//! exhaustively match today must add a wildcard arm.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod adapter;
pub mod backend;
pub mod family;
pub mod info;
pub mod testing;

pub use adapter::{AnyEngine, EngineAdapter};
pub use backend::{BackendDescriptor, BackendKind};
pub use family::{EngineFamily, InvalidEngineFamily};
pub use info::EngineInfo;
pub use testing::MockAdapter;
pub use voxora_core::ModelCapabilities;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_surface_lists_core_types() {
        // Compile-time check that the re-exports stay in sync.
        let _: EngineFamily = EngineFamily::Whisper;
        let _: BackendKind = BackendKind::Cpu;
        let _: BackendDescriptor = BackendDescriptor::CPU;
    }
}
