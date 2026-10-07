//! Offline tests for `QwenAsrEngine::from_hf_with_device` and
//! `QwenAsrEngine::prepare_model_dir`. A fake `ModelSource` resolves to
//! a temp directory, so no network or model weights are needed.

use std::path::PathBuf;

use async_trait::async_trait;
use voxora_qwen3asr::{Device, QwenAsrEngine};
use voxora_traits::{
    AsrError, ModelCapabilities, ModelDir, ModelSource, ModelSourceKind, Quantization,
    ResolveOptions,
};

struct FixedDirSource(PathBuf);

#[async_trait]
impl ModelSource for FixedDirSource {
    fn name(&self) -> &'static str {
        "fixed-dir"
    }

    async fn resolve(&self, _model_id: &str, _opts: &ResolveOptions) -> Result<ModelDir, AsrError> {
        Ok(ModelDir::new(
            self.0.clone(),
            ModelSourceKind::Local,
            Quantization::F16,
        ))
    }

    async fn capabilities_for(&self, _model_id: &str) -> Result<ModelCapabilities, AsrError> {
        Ok(ModelCapabilities::default())
    }
}

fn write_tokenizer_trio(dir: &std::path::Path) {
    std::fs::write(dir.join("vocab.json"), br#"{"a":0,"b":1}"#).unwrap();
    std::fs::write(dir.join("merges.txt"), b"a b\n").unwrap();
    std::fs::write(
        dir.join("tokenizer_config.json"),
        br#"{"added_tokens_decoder":{}}"#,
    )
    .unwrap();
}

#[tokio::test]
async fn missing_tokenizer_sources_fail_before_loading() {
    let dir = tempfile::tempdir().unwrap();
    let source = FixedDirSource(dir.path().to_path_buf());

    let err = QwenAsrEngine::from_hf_with_device(
        &source,
        "Qwen/Qwen3-ASR-0.6B",
        &ResolveOptions::default(),
        Device::Cpu,
    )
    .await
    .expect_err("empty dir cannot load");

    assert!(
        matches!(err, AsrError::ModelNotFound(_)),
        "expected ModelNotFound from the tokenizer step, got {err:?}"
    );
}

#[tokio::test]
async fn tokenizer_is_prepared_then_load_is_attempted_on_the_given_device() {
    let dir = tempfile::tempdir().unwrap();
    write_tokenizer_trio(dir.path());
    let source = FixedDirSource(dir.path().to_path_buf());

    let err = QwenAsrEngine::from_hf_with_device(
        &source,
        "Qwen/Qwen3-ASR-0.6B",
        &ResolveOptions::default(),
        Device::Cpu,
    )
    .await
    .expect_err("no config.json / weights, so the load itself must fail");

    assert!(
        dir.path().join("tokenizer.json").is_file(),
        "tokenizer.json must be synthesised before the load"
    );
    assert!(
        !matches!(err, AsrError::ModelNotFound(_)),
        "failure must come from the load, not the tokenizer step: {err:?}"
    );
}

#[test]
fn prepare_model_dir_synthesises_and_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    write_tokenizer_trio(dir.path());

    QwenAsrEngine::prepare_model_dir(dir.path()).expect("first call synthesises");
    let first = std::fs::read(dir.path().join("tokenizer.json")).unwrap();
    QwenAsrEngine::prepare_model_dir(dir.path()).expect("second call is a no-op");
    let second = std::fs::read(dir.path().join("tokenizer.json")).unwrap();

    assert_eq!(first, second);
}
