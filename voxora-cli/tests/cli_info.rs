//! Wiremock-backed integration tests for `voxora info`. The CLI is
//! launched as a subprocess with the hidden `--base-url` flag
//! pointing at a wiremock `MockServer`.

mod common;

use std::process::Command;

use common::{read_fixture_bytes, voxora_bin};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const MODEL_ID: &str = "Qwen/Qwen3-ASR-0.6B";

#[tokio::test]
async fn info_prints_capabilities_for_known_model() {
    let mock = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/{MODEL_ID}/resolve/main/config.json")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(read_fixture_bytes("qwen3-asr-0.6b", "config.json")),
        )
        .expect(1)
        .mount(&mock)
        .await;

    let out = Command::new(voxora_bin())
        .args([
            "info",
            MODEL_ID,
            "--base-url",
            &mock.uri(),
            "--cache",
            "/tmp/voxora-cli-no-cache-used",
        ])
        .output()
        .expect("voxora info");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}; stdout: {stdout}");
    assert!(stdout.contains("Qwen/Qwen3-ASR-0.6B"), "stdout: {stdout}");
    assert!(stdout.contains("multilingual  : true"), "stdout: {stdout}");
    // The wiremock served the same `config.json` as `voxora-hf`'s
    // own tests, so the language list should contain `english`.
    assert!(
        stdout.contains("english"),
        "stdout should list english: {stdout}"
    );
}

#[tokio::test]
async fn info_reports_404_for_unknown_model() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/nosuch/modelid/resolve/main/config.json"))
        .respond_with(ResponseTemplate::new(404).set_body_bytes(b"not found"))
        .expect(1)
        .mount(&mock)
        .await;

    let out = Command::new(voxora_bin())
        .args(["info", "nosuch/modelid", "--base-url", &mock.uri()])
        .output()
        .expect("voxora info");

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "should be a runtime failure");
    assert_eq!(
        out.status.code(),
        Some(1),
        "404 from HF is a runtime failure (exit 1), stderr: {stderr}"
    );
}

#[tokio::test]
async fn info_without_slash_exits_two() {
    // We don't even spin up a mock server — the input validation
    // runs before any network call.
    let out = Command::new(voxora_bin())
        .args(["info", "noslash"])
        .output()
        .expect("voxora info");
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("org/name"));
}
