//! Resolve a sharded model end-to-end against wiremock.
//!
//! Fixtures (real recordings from `huggingface.co`):
//! - `qwen3-asr-1.7b/_metadata.json`
//! - `qwen3-asr-1.7b/config.json`
//! - `qwen3-asr-1.7b/preprocessor_config.json`
//! - `qwen3-asr-1.7b/tokenizer_config.json`
//! - `qwen3-asr-1.7b/model.safetensors.index.json`
//! - `qwen3-asr-1.7b/vocab.json`, `merges.txt`
//!
//! The actual safetensors shards are too large to ship, so the
//! server returns synthetic bytes.

mod common;

use common::{read_fixture, resolve_ok, source_for, synthetic_shard};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

const MODEL_ID: &str = "Qwen/Qwen3-ASR-1.7B";
const FIXTURE_DIR: &str = "qwen3-asr-1.7b";

#[tokio::test]
async fn resolve_qwen3_asr_1_7b_sharded() {
    let mock = wiremock::MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/api/models/{MODEL_ID}/revision/main")))
        .respond_with(
            ResponseTemplate::new(200).set_body_bytes(read_fixture(FIXTURE_DIR, "_metadata.json")),
        )
        .mount(&mock)
        .await;

    for fname in [
        "config.json",
        "preprocessor_config.json",
        "tokenizer_config.json",
        "model.safetensors.index.json",
        "vocab.json",
        "merges.txt",
    ] {
        Mock::given(method("GET"))
            .and(path(format!("/{MODEL_ID}/resolve/main/{fname}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_bytes(read_fixture(FIXTURE_DIR, fname)),
            )
            .mount(&mock)
            .await;
    }

    // The sharded index points to two shards. Read the index fixture
    // so we know which filenames to mock.
    let index_bytes = read_fixture(FIXTURE_DIR, "model.safetensors.index.json");
    let index: serde_json::Value = serde_json::from_slice(&index_bytes).unwrap();
    let shards: Vec<String> = index["weight_map"]
        .as_object()
        .unwrap()
        .values()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    let mut unique = shards.clone();
    unique.sort();
    unique.dedup();
    assert!(
        unique.len() >= 2,
        "expected ≥2 shards, got {}",
        unique.len()
    );

    for shard in &unique {
        Mock::given(method("GET"))
            .and(path(format!("/{MODEL_ID}/resolve/main/{shard}")))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(synthetic_shard(shard)))
            .expect(1)
            .mount(&mock)
            .await;
    }

    let (_cache, src) = source_for(&mock, None).await;
    let dir = resolve_ok(&src, MODEL_ID).await;

    assert!(dir.path.join(".complete").is_file());
    assert!(dir.path.join("model.safetensors.index.json").is_file());
    for shard in &unique {
        let dest = dir.path.join(shard);
        assert!(dest.is_file(), "shard {shard} missing");
        let bytes = std::fs::read(&dest).unwrap();
        assert_eq!(bytes.len(), 1024, "shard {shard} not synthetic");
    }
    assert_eq!(
        voxora_core::Quantization::Bf16,
        dir.quantization,
        "qwen3 1.7b is BF16"
    );
}
