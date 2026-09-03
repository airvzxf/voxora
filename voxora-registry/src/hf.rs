//! HF-backed [`Registry`] builder.
//!
//! Gated behind the `hf` feature so a downstream consumer that
//! wants to use voxora-registry with a non-HF source can disable
//! the dependency on `voxora-hf`.

use std::sync::Arc;

use voxora_traits::ModelSource;
use voxora_hf::HuggingFaceSource;

use crate::resolver::Registry;

/// Extension trait: `Registry::with_builtin_descriptors()`.
pub trait RegistryHfExt {
    /// Register Whisper and Qwen3-ASR descriptors and return the
    /// [`Registry`] for chaining.
    fn with_builtin_descriptors(self) -> Self;
}

impl RegistryHfExt for Registry {
    fn with_builtin_descriptors(mut self) -> Self {
        use crate::builtin::{builtin_qwen3asr_descriptor, builtin_whisper_descriptor};
        self.descriptors_mut().push(builtin_whisper_descriptor());
        self.descriptors_mut().push(builtin_qwen3asr_descriptor());
        self
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
}
