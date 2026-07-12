//! Resolve a single-file model end-to-end against a wiremock server
//! that replays real HF recordings.
//!
//! Fixtures:
//! - `qwen3-asr-0.6b/_metadata.json` (siblings list, real)
//! - `qwen3-asr-0.6b/config.json` (real)
//! - `qwen3-asr-0.6b/preprocessor_config.json` (real)
//! - `qwen3-asr-0.6b/tokenizer_config.json` (real)
//! - `qwen3-asr-0.6b/vocab.json`, `merges.txt` (real, large text files)
//!
//! The model.safetensors file is served as 1 KB of synthetic bytes
//! because the real one is ~600 MB.

mod common;

use common::{read_fixture, resolve_ok, source_for, synthetic_safetensors};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

const MODEL_ID: &str = "Qwen/Qwen3-ASR-0.6B";
const FIXTURE_DIR: &str = "qwen3-asr-0.6b";

#[tokio::test]
async fn resolve_qwen3_asr_0_6b_single_file() {
    let mock = wiremock::MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/api/models/{MODEL_ID}/revision/main")))
        .respond_with(
            ResponseTemplate::new(200).set_body_bytes(read_fixture(FIXTURE_DIR, "_metadata.json")),
        )
        .mount(&mock)
        .await;

    // Single-file safetensors.
    Mock::given(method("GET"))
        .and(path(format!("/{MODEL_ID}/resolve/main/model.safetensors")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(synthetic_safetensors("single")))
        .expect(1)
        .mount(&mock)
        .await;

    for fname in [
        "config.json",
        "preprocessor_config.json",
        "tokenizer_config.json",
        "vocab.json",
        "merges.txt",
    ] {
        Mock::given(method("GET"))
            .and(path(format!("/{MODEL_ID}/resolve/main/{fname}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_bytes(read_fixture(FIXTURE_DIR, fname)),
            )
            .expect(1)
            .mount(&mock)
            .await;
    }

    let (_cache, src) = source_for(&mock, None).await;
    let dir = resolve_ok(&src, MODEL_ID).await;

    assert!(dir.path.exists(), "model dir must exist on disk");
    assert!(dir.path.join(".complete").is_file(), "marker missing");
    assert!(dir.path.join("config.json").is_file());
    assert!(dir.path.join("preprocessor_config.json").is_file());
    assert!(dir.path.join("tokenizer_config.json").is_file());
    assert!(dir.path.join("vocab.json").is_file());
    assert!(dir.path.join("merges.txt").is_file());
    assert!(dir.path.join("model.safetensors").is_file());
    assert_eq!(
        voxora_core::ModelSourceKind::HuggingFace,
        dir.kind,
        "kind must be HF"
    );
    // Qwen3-ASR 0.6B official release ships as BF16.
    assert_eq!(
        voxora_core::Quantization::Bf16,
        dir.quantization,
        "expected Bf16 from config.json arch"
    );

    // Files match what the server sent.
    let on_disk_safetensors = std::fs::read(dir.path.join("model.safetensors")).unwrap();
    assert_eq!(on_disk_safetensors.len(), 1024);
    let on_disk_config = std::fs::read_to_string(dir.path.join("config.json")).unwrap();
    assert!(
        on_disk_config.contains("Qwen3ASRForConditionalGeneration"),
        "config.json must contain real arch string"
    );
    assert!(
        on_disk_config.contains("\"qwen3_asr\""),
        "model_type must be qwen3_asr"
    );
}

#[tokio::test]
async fn second_resolve_skips_downloads_when_cached() {
    let mock = wiremock::MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/api/models/{MODEL_ID}/revision/main")))
        .respond_with(
            ResponseTemplate::new(200).set_body_bytes(read_fixture(FIXTURE_DIR, "_metadata.json")),
        )
        // Exactly ONE siblings call across both resolves.
        .expect(1)
        .mount(&mock)
        .await;

    for fname in [
        "config.json",
        "preprocessor_config.json",
        "tokenizer_config.json",
        "vocab.json",
        "merges.txt",
    ] {
        Mock::given(method("GET"))
            .and(path(format!("/{MODEL_ID}/resolve/main/{fname}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_bytes(read_fixture(FIXTURE_DIR, fname)),
            )
            .expect(1)
            .mount(&mock)
            .await;
    }
    Mock::given(method("GET"))
        .and(path(format!("/{MODEL_ID}/resolve/main/model.safetensors")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(synthetic_safetensors("cached")))
        .expect(1)
        .mount(&mock)
        .await;

    let (_cache, src) = source_for(&mock, None).await;
    let _ = resolve_ok(&src, MODEL_ID).await;
    let again = resolve_ok(&src, MODEL_ID).await;

    assert!(again.path.join(".complete").is_file());
    assert_eq!(again.kind, voxora_core::ModelSourceKind::HuggingFace);
}
