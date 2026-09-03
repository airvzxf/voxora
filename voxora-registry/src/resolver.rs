//! The [`Registry`] itself: a list of [`EngineDescriptor`]s and a
//! resolver that maps a [`ModelId`] to (descriptor, [`voxora_core::ModelDir`]).

use std::sync::Arc;

use voxora_core::{ModelDir, ModelSource, ResolveOptions};

use crate::descriptor::EngineDescriptor;
use crate::error::RegistryError;
use crate::id::ModelId;

/// Resolved pair: which engine the model belongs to, and the on-disk
/// path to load it from (with `entry` already populated).
#[derive(Debug, Clone)]
pub struct ResolvedModel {
    /// Engine descriptor that accepted the resolved model id.
    pub descriptor: EngineDescriptor,
    /// On-disk directory returned by the underlying [`ModelSource`].
    pub model_dir: ModelDir,
}

/// A collection of [`EngineDescriptor`]s plus a [`ModelSource`] used
/// to download / locate the model on disk.
pub struct Registry {
    descriptors: Vec<EngineDescriptor>,
    source: Arc<dyn ModelSource>,
}

impl Registry {
    /// Construct a [`Registry`] backed by the given [`ModelSource`].
    pub fn new(source: Arc<dyn ModelSource>) -> Self {
        Self {
            descriptors: Vec::new(),
            source,
        }
    }

    /// Register an additional [`EngineDescriptor`].
    pub fn register(mut self, descriptor: EngineDescriptor) -> Self {
        self.descriptors.push(descriptor);
        self
    }

    /// List all registered descriptors.
    pub fn descriptors(&self) -> &[EngineDescriptor] {
        &self.descriptors
    }

    /// Mutable access to the descriptor list, used by feature-gated
    /// builders (e.g. the `hf` adapter's
    /// `Registry::with_builtin_descriptors`).
    pub fn descriptors_mut(&mut self) -> &mut Vec<EngineDescriptor> {
        &mut self.descriptors
    }

    /// Resolve a [`ModelId`] to the first matching descriptor and
    /// the on-disk [`ModelDir`].
    pub async fn resolve(
        &self,
        id: &ModelId,
        opts: &ResolveOptions,
    ) -> Result<ResolvedModel, RegistryError> {
        let descriptor = self
            .descriptors
            .iter()
            .find(|d| (d.accepts)(id))
            .cloned()
            .ok_or_else(|| RegistryError::NoMatchingDescriptor(id.canonical()))?;

        let canonical = id.canonical();
        let model_dir = self
            .source
            .resolve(&canonical, opts)
            .await
            .map_err(|e| RegistryError::Parse(format!("source resolve: {e}")))?;

        Ok(ResolvedModel {
            descriptor,
            model_dir,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::SourceKind;
    use async_trait::async_trait;
    use voxora_core::{AsrError, ModelCapabilities, ModelDir, ModelSourceKind, Quantization};
    use voxora_engine::EngineFamily;

    struct EchoSource;

    #[async_trait]
    impl ModelSource for EchoSource {
        fn name(&self) -> &'static str {
            "echo"
        }

        async fn resolve(
            &self,
            model_id: &str,
            _opts: &ResolveOptions,
        ) -> Result<ModelDir, AsrError> {
            Ok(ModelDir::with_entry(
                std::path::PathBuf::from(format!("/cache/{model_id}")),
                std::path::PathBuf::from(format!("/cache/{model_id}/model.bin")),
                ModelSourceKind::Local,
                Quantization::F16,
            ))
        }

        async fn capabilities_for(&self, _model_id: &str) -> Result<ModelCapabilities, AsrError> {
            Ok(ModelCapabilities::UNKNOWN)
        }
    }

    fn whisper_desc() -> EngineDescriptor {
        EngineDescriptor::new(
            EngineFamily::Whisper,
            "whisper",
            |id| {
                matches!(id.source, SourceKind::HuggingFace)
                    && id.repo.starts_with("ggerganov/whisper.cpp")
            },
            ModelCapabilities::UNKNOWN,
        )
    }

    #[tokio::test]
    async fn resolve_picks_first_matching_descriptor() {
        let registry = Registry::new(Arc::new(EchoSource)).register(whisper_desc());
        let id = ModelId::parse("ggerganov/whisper.cpp/ggml-tiny.bin").unwrap();
        let resolved = registry
            .resolve(&id, &ResolveOptions::default())
            .await
            .expect("resolve");
        assert_eq!(resolved.descriptor.family, EngineFamily::Whisper);
        assert!(resolved.model_dir.entry.is_some());
    }

    #[tokio::test]
    async fn resolve_errors_when_no_descriptor_matches() {
        let registry = Registry::new(Arc::new(EchoSource)).register(whisper_desc());
        let id = ModelId::parse("Qwen/Qwen3-ASR-0.6B").unwrap();
        let err = registry
            .resolve(&id, &ResolveOptions::default())
            .await
            .expect_err("no match");
        assert!(matches!(err, RegistryError::NoMatchingDescriptor(_)));
    }
}
