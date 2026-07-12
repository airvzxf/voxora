//! Wiremock-backed integration tests for `voxora download`.

mod common;

use std::process::Command;

use common::{read_fixture_bytes, synthetic_safetensors, voxora_bin};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const MODEL_ID: &str = "Qwen/Qwen3-ASR-0.6B";
const FIXTURE_DIR: &str = "qwen3-asr-0.6b";

async fn setup_qwen3_asr_single_file_mock(mock: &MockServer) {
    Mock::given(method("GET"))
        .and(path(format!("/api/models/{MODEL_ID}/revision/main")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(read_fixture_bytes(FIXTURE_DIR, "_metadata.json")),
        )
        .expect(1)
        .mount(mock)
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
                ResponseTemplate::new(200).set_body_bytes(read_fixture_bytes(FIXTURE_DIR, fname)),
            )
            .expect(1)
            .mount(mock)
            .await;
    }
    Mock::given(method("GET"))
        .and(path(format!("/{MODEL_ID}/resolve/main/model.safetensors")))
        .respond_with(
            ResponseTemplate::new(200).set_body_bytes(synthetic_safetensors("voxora-cli")),
        )
        .expect(1)
        .mount(mock)
        .await;
}

#[tokio::test]
async fn download_resolves_to_cache_dir_and_marks_complete() {
    let mock = MockServer::start().await;
    setup_qwen3_asr_single_file_mock(&mock).await;

    let tmp = tempfile::tempdir().unwrap();
    let cache_root = tmp.path().join("voxora-cli-test");
    std::fs::create_dir_all(&cache_root).unwrap();

    let out = Command::new(voxora_bin())
        .args([
            "download",
            MODEL_ID,
            "--base-url",
            &mock.uri(),
            "--cache",
            cache_root.to_str().unwrap(),
        ])
        .output()
        .expect("voxora download");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}; stdout: {stdout}");

    // stdout is the resolved path; stderr is the progress summary.
    let model_dir = std::path::PathBuf::from(stdout.trim());
    assert!(model_dir.is_dir(), "resolved path must exist: {stdout}");
    assert!(
        model_dir.join(".complete").is_file(),
        ".complete marker must be present"
    );
    assert!(model_dir.join("config.json").is_file());
    assert!(model_dir.join("model.safetensors").is_file());
}

#[tokio::test]
async fn download_second_call_is_idempotent() {
    let mock = MockServer::start().await;
    setup_qwen3_asr_single_file_mock(&mock).await;

    let tmp = tempfile::tempdir().unwrap();
    let cache_root = tmp.path().join("voxora-cli-test");
    std::fs::create_dir_all(&cache_root).unwrap();

    // First call: full download.
    let out = Command::new(voxora_bin())
        .args([
            "download",
            MODEL_ID,
            "--base-url",
            &mock.uri(),
            "--cache",
            cache_root.to_str().unwrap(),
        ])
        .output()
        .expect("first voxora download");
    assert!(out.status.success());

    // Second call: point at a SECOND mock server that 503s every
    // request. If the cache marker is honoured, voxora never makes
    // any HTTP call, so the second mock never sees traffic. If the
    // cache is broken, the 503 propagates as a runtime failure.
    let mock2 = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&mock2)
        .await;

    let out = Command::new(voxora_bin())
        .args([
            "download",
            MODEL_ID,
            "--base-url",
            &mock2.uri(),
            "--cache",
            cache_root.to_str().unwrap(),
        ])
        .output()
        .expect("second voxora download");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "second call must hit cache and succeed; stderr: {stderr}; stdout: {stdout}"
    );
}

#[tokio::test]
async fn download_with_unknown_quantization_exits_two() {
    // No mock server needed — quantization validation runs before any
    // network call.
    let out = Command::new(voxora_bin())
        .args(["download", MODEL_ID, "--quantization", "int8fictional"])
        .output()
        .expect("voxora download");
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("int8fictional"), "stderr: {stderr}");
}
