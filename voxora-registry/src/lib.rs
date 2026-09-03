//! Central model registry for voxora (ASR-specific).
//!
//! Owns the answer to "given a model id, give me the engine + the
//! exact on-disk file". Before this crate existed, that answer was
//! distributed across `voxora-hf` (cache key), `voxora-whisper`
//! (lex-sort over `*.bin`), and `voxora-cli` (heuristic). Now it
//! lives here.
//!
//! ## Phase 0 surface
//!
//! - [`ModelId::parse`] — strict parser for `org/repo` /
//!   `org/repo/file` / `/local/path` / `./local-path`.
//! - [`Registry`] — a list of [`EngineDescriptor`]s + a
//!   [`voxora_core::ModelSource`] used to download/locate the model.
//! - [`builtin_whisper_descriptor`] / [`builtin_qwen3asr_descriptor`]
//!   — default descriptors. [`Registry::with_builtin_descriptors`]
//!   builds a registry that knows about both engines out of the box.
//! - [`CacheManifest`] — `.voxora-manifest.json` written next to
//!   cached weights so future runs can answer "which engine?" without
//!   re-parsing the directory.
//!
//! ASR-specific: the descriptors model only ASR engines (Whisper,
//! Qwen3-Asr). Adding `parakeet`, `voxtral`, or `granite-speech` is
//! a 0.2.x change.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod builtin;
pub mod descriptor;
pub mod error;
pub mod id;
pub mod manifest;
pub mod resolver;

#[cfg(feature = "hf")]
pub mod hf;

pub use builtin::{builtin_qwen3asr_descriptor, builtin_whisper_descriptor};
pub use descriptor::{EngineCapabilities, EngineDescriptor};
pub use error::RegistryError;
pub use id::{ModelId, SourceKind};
pub use manifest::{CacheManifest, MANIFEST_FILENAME, MANIFEST_VERSION};
pub use resolver::{Registry, ResolvedModel};

#[cfg(feature = "hf")]
pub use hf::{RegistryHfExt, hf_registry};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_surface_lists_core_types() {
        // Compile-time check that the re-exports stay in sync.
        let _id: ModelId = ModelId::parse("Qwen/Qwen3-ASR-0.6B").unwrap();
        let _src = SourceKind::HuggingFace;
    }
}
