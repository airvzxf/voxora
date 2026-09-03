//! Integration test: a whole-repo id resolves to a ModelDir with
//! `entry = None`.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use voxora_core::{
    AsrError, ModelCapabilities, ModelDir, ModelSource, ModelSourceKind, Quantization,
    ResolveOptions,
};
use voxora_registry::{Registry, builtin_qwen3asr_descriptor, builtin_whisper_descriptor};

struct WholeRepoSource;

#[async_trait]
impl ModelSource for WholeRepoSource {
    fn name(&self) -> &'static str {
        "whole-repo"
    }

    async fn resolve(&self, model_id: &str, _opts: &ResolveOptions) -> Result<ModelDir, AsrError> {
        Ok(ModelDir::new(
            PathBuf::from(format!("/cache/{model_id}")),
            ModelSourceKind::HuggingFace,
            Quantization::Bf16,
        ))
    }

    async fn capabilities_for(&self, _model_id: &str) -> Result<ModelCapabilities, AsrError> {
        Ok(ModelCapabilities::UNKNOWN)
    }
}

#[tokio::test]
async fn whole_repo_id_has_no_entry() {
    let registry = Registry::new(Arc::new(WholeRepoSource))
        .register(builtin_whisper_descriptor())
        .register(builtin_qwen3asr_descriptor());

    let id = voxora_registry::ModelId::parse("Qwen/Qwen3-ASR-0.6B").expect("parse");
    let resolved = registry
        .resolve(&id, &ResolveOptions::default())
        .await
        .expect("resolve");

    assert!(
        resolved.model_dir.entry.is_none(),
        "whole-repo: entry stays None"
    );
    assert_eq!(
        resolved.descriptor.family,
        voxora_engine::EngineFamily::Qwen3Asr
    );
}

#[tokio::test]
async fn no_matching_descriptor_errors() {
    let registry = Registry::new(Arc::new(WholeRepoSource)).register(builtin_whisper_descriptor());

    let id = voxora_registry::ModelId::parse("Qwen/Qwen3-ASR-0.6B").expect("parse");
    let err = registry
        .resolve(&id, &ResolveOptions::default())
        .await
        .expect_err("no match");
    assert!(matches!(
        err,
        voxora_registry::RegistryError::NoMatchingDescriptor(_)
    ));
}
