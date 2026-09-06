//! HF-backed [`Registry`] builder.
//!
//! Gated behind the `hf` feature so a downstream consumer that
//! wants to use voxora-registry with a non-HF source can disable
//! the dependency on `voxora-hf`.

use std::sync::Arc;

#[cfg(feature = "local")]
use std::path::PathBuf;

use voxora_hf::HuggingFaceSource;
use voxora_traits::ModelSource;

use crate::resolver::Registry;

#[cfg(feature = "local")]
use crate::builtin::builtin_local_descriptor;
#[cfg(feature = "local")]
use voxora_local::{ChainedSource, LocalSource};

/// Extension trait: `Registry::with_builtin_descriptors()`.
pub trait RegistryHfExt {
    /// Register Whisper and Qwen3-ASR descriptors and return the
    /// [`Registry`] for chaining.
    fn with_builtin_descriptors(self) -> Self;

    /// Register the same built-in descriptors as
    /// [`Self::with_builtin_descriptors`] AND wrap the registry's
    /// underlying [`voxora_traits::ModelSource`] with a
    /// `voxora_local::ChainedSource::new(LocalSource::new(local_root), HuggingFaceSource::new()?)`.
    ///
    /// The accepted arm gains a Local-id fallback descriptor
    /// ([`crate::builtin::builtin_local_descriptor`] with
    /// [`voxora_engine::EngineFamily::Whisper`]) registered AFTER
    /// the HF built-ins, so the existing first-match-wins order is
    /// preserved: a Whisper HF id still hits the Whisper HF
    /// descriptor, a Qwen HF id still hits the Qwen descriptor, and
    /// a `SourceKind::Local` id is accepted by the Local fallback
    /// and resolved against `local_root` first, falling through to
    /// HF on `AsrError::ModelNotFound`.
    ///
    /// # Engine-family routing
    ///
    /// The new Local descriptor's `family` is
    /// [`voxora_engine::EngineFamily::Whisper`] by default. A
    /// consumer dispatching on `resolved.descriptor.family` will
    /// therefore route any `SourceKind::Local` id through the
    /// Whisper engine loader (which consumes
    /// [`voxora_traits::ModelDir::entry`] directly — the shape
    /// `LocalSource::resolve` returns). Consumers needing
    /// Local-as-Qwen must register their own
    /// [`crate::descriptor::EngineDescriptor`] with
    /// `family = EngineFamily::Qwen3Asr` **before** calling this
    /// helper, so the registered descriptor wins over the Local
    /// fallback for matching ids.
    ///
    /// # Path-joining semantics
    ///
    /// `LocalSource::resolve` does `root.join(model_id)`. Rust's
    /// `PathBuf::join` semantics: an absolute `model_id` replaces
    /// the prefix, so absolute Local ids (e.g. `/srv/models/qwen.bin`)
    /// resolve verbatim; relative Local ids (e.g. `./org/repo/file.bin`)
    /// join under the configured `local_root`. Both shapes work
    /// without changes to this helper.
    ///
    /// # Errors
    ///
    /// Returns `voxora_traits::AsrError` if the underlying
    /// [`HuggingFaceSource::new`] fails (env-cascade error building
    /// the HTTP client). The `LocalSource` itself has no failure
    /// mode at construction time — the directory's contents are
    /// inspected lazily at resolve time.
    #[cfg(feature = "local")]
    #[cfg_attr(docsrs, doc(cfg(feature = "local")))]
    fn with_builtin_descriptors_and_chained_source(
        self,
        local_root: impl Into<PathBuf>,
    ) -> Result<Self, voxora_traits::AsrError>
    where
        Self: Sized;
}

impl RegistryHfExt for Registry {
    fn with_builtin_descriptors(mut self) -> Self {
        use crate::builtin::{builtin_qwen3asr_descriptor, builtin_whisper_descriptor};
        self.descriptors_mut().push(builtin_whisper_descriptor());
        self.descriptors_mut().push(builtin_qwen3asr_descriptor());
        self
    }

    #[cfg(feature = "local")]
    fn with_builtin_descriptors_and_chained_source(
        mut self,
        local_root: impl Into<PathBuf>,
    ) -> Result<Self, voxora_traits::AsrError> {
        use voxora_engine::EngineFamily;

        // Register the existing HF built-ins first (preserves the
        // first-match-wins accept order) and the Local fallback
        // descriptor LAST. A registered consumer descriptor wins
        // over the Local fallback for matching ids.
        self = self.with_builtin_descriptors();
        self.descriptors_mut()
            .push(builtin_local_descriptor(EngineFamily::Whisper));

        // Wrap the registry's `source` with the canonical
        // "local first, HF on miss" chain. The registry already
        // holds the HF source as `source`; we read it out via the
        // descriptor list ordering rather than a public accessor
        // because Registry does not expose `source` directly
        // (encapsulation — `with_source` is the only writer).
        //
        // Build the chain with a fresh `HuggingFaceSource` so the
        // caller's existing `Registry::new(source)` value is not
        // moved out from under them. The two sources are
        // independent: both read the same env cascade and end up
        // pointing at the same cache root.
        let local = Arc::new(LocalSource::new(local_root));
        let hf: Arc<dyn ModelSource> = Arc::new(HuggingFaceSource::new()?);
        let chain: Arc<dyn ModelSource> = Arc::new(ChainedSource::new(local, hf));
        Ok(self.with_source(chain))
    }
}

/// Convenience: build a HF-backed registry with all built-in descriptors.
pub fn hf_registry() -> Result<Registry, voxora_traits::AsrError> {
    let source: Arc<dyn ModelSource> = Arc::new(HuggingFaceSource::new()?);
    Ok(Registry::new(source).with_builtin_descriptors())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::ModelId;

    #[test]
    fn extension_trait_registers_builtins() {
        let src: Arc<dyn ModelSource> = Arc::new(voxora_hf::HuggingFaceSource::new().unwrap());
        let registry = Registry::new(src).with_builtin_descriptors();
        assert_eq!(registry.descriptors().len(), 2);

        let id = ModelId::parse("ggerganov/whisper.cpp/ggml-large-v3.bin").unwrap();
        assert!(registry.descriptors().iter().any(|d| (d.accepts)(&id)));
    }

    /// The chained-source helper (when both `hf` and `local`
    /// features are on) must register the Local fallback descriptor
    /// after the HF built-ins. This pins the descriptor ordering
    /// that the integration test in `tests/registry.rs` relies on.
    #[cfg(feature = "local")]
    #[test]
    fn chained_source_helper_registers_local_fallback_last() {
        let registry = Registry::new(Arc::new(voxora_hf::HuggingFaceSource::new().unwrap()))
            .with_builtin_descriptors_and_chained_source(tempfile::tempdir().unwrap().path())
            .expect("chained helper");
        // Whisper HF + Qwen HF + Local fallback = 3 descriptors.
        assert_eq!(registry.descriptors().len(), 3);
        // The Local fallback's accept predicate matches any Local
        // id — the last descriptor in the list.
        let local_id = ModelId::parse("/srv/models/anything.bin").unwrap();
        let last = registry.descriptors().last().expect("last descriptor");
        assert!((last.accepts)(&local_id));
    }
}
