//! Integration test: a single-file id resolves to a ModelDir with
//! `entry` populated — the structural fix for voxora#79.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use voxora_core::{
    AsrError, ModelCapabilities, ModelDir, ModelSource, ModelSourceKind, Quantization,
    ResolveOptions,
};
use voxora_engine::EngineFamily;
use voxora_registry::{Registry, builtin_whisper_descriptor};

struct FakeSource;

#[async_trait]
impl ModelSource for FakeSource {
    fn name(&self) -> &'static str {
        "fake"
    }

    async fn resolve(&self, model_id: &str, _opts: &ResolveOptions) -> Result<ModelDir, AsrError> {
        let parts: Vec<&str> = model_id.split('/').collect();
        let file = parts.last().copied().unwrap_or("");
        let dir_path = PathBuf::from(format!("/cache/{model_id}"));
        if file.is_empty() {
            Ok(ModelDir::new(
                dir_path,
                ModelSourceKind::HuggingFace,
                Quantization::F16,
            ))
        } else {
            Ok(ModelDir::with_entry(
                dir_path,
                PathBuf::from(format!("/cache/{model_id}/{file}")),
                ModelSourceKind::HuggingFace,
                Quantization::F16,
            ))
        }
    }

    async fn capabilities_for(&self, _model_id: &str) -> Result<ModelCapabilities, AsrError> {
        Ok(ModelCapabilities::UNKNOWN)
    }
}

#[tokio::test]
async fn single_file_id_resolves_with_entry_populated() {
    let registry = Registry::new(Arc::new(FakeSource)).register(builtin_whisper_descriptor());

    let id =
        voxora_registry::ModelId::parse("ggerganov/whisper.cpp/ggml-large-v3.bin").expect("parse");
    let resolved = registry
        .resolve(&id, &ResolveOptions::default())
        .await
        .expect("resolve");

    assert_eq!(resolved.descriptor.family, EngineFamily::Whisper);
    let entry = resolved.model_dir.entry.expect("entry populated");
    assert_eq!(
        entry,
        PathBuf::from("/cache/ggerganov/whisper.cpp/ggml-large-v3.bin/ggml-large-v3.bin"),
        "entry must name the exact file"
    );
}

#[tokio::test]
async fn single_file_id_does_not_lex_sort() {
    // The whole point of the registry: callers no longer have to
    // lex-sort over `*.bin`. The entry field encodes the exact file.
    let registry = Registry::new(Arc::new(FakeSource)).register(builtin_whisper_descriptor());

    let id =
        voxora_registry::ModelId::parse("ggerganov/whisper.cpp/ggml-large-v3.bin").expect("parse");
    let resolved = registry
        .resolve(&id, &ResolveOptions::default())
        .await
        .expect("resolve");

    let entry = resolved.model_dir.entry.expect("entry populated");
    let leaf = entry.file_name().and_then(|s| s.to_str()).expect("leaf");
    assert_eq!(
        leaf, "ggml-large-v3.bin",
        "no lex-sort: leaf must be the requested file"
    );
}

#[tokio::test]
async fn single_file_id_with_no_entry_errors_missing() {
    // A source that ignores single-file requests and only ever
    // returns ModelDir::new (entry: None).
    struct EntrylessSource;
    #[async_trait]
    impl voxora_core::ModelSource for EntrylessSource {
        fn name(&self) -> &'static str {
            "entryless"
        }
        async fn resolve(
            &self,
            model_id: &str,
            _opts: &voxora_core::ResolveOptions,
        ) -> Result<voxora_core::ModelDir, voxora_core::AsrError> {
            Ok(voxora_core::ModelDir::new(
                PathBuf::from(format!("/cache/{model_id}")),
                voxora_core::ModelSourceKind::Local,
                voxora_core::Quantization::F16,
            ))
        }
        async fn capabilities_for(
            &self,
            _model_id: &str,
        ) -> Result<voxora_core::ModelCapabilities, voxora_core::AsrError> {
            Ok(voxora_core::ModelCapabilities::UNKNOWN)
        }
    }

    let registry = Registry::new(Arc::new(EntrylessSource)).register(builtin_whisper_descriptor());
    let id = voxora_registry::ModelId::parse("ggerganov/whisper.cpp/ggml-large-v3.bin").unwrap();
    let err = registry
        .resolve(&id, &ResolveOptions::default())
        .await
        .expect_err("missing entry");
    assert!(matches!(
        err,
        voxora_registry::RegistryError::MissingModelFile(_)
    ));
}
