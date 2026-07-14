//! Wiremock test for the single-file download path
//! (`HuggingFaceSource::resolve` with a 3-segment model id).
//!
//! Verifies that when a caller passes `org/repo/file`, voxora-hf:
//!
//! - does NOT call the metadata endpoint
//!   (`GET /api/models/{org}/{repo}/revision/main`) — there is no
//!   multi-file repo to enumerate.
//! - downloads exactly one file:
//!   `GET /{org}/{repo}/resolve/main/{file}`.
//! - writes the `.complete` marker in the same directory layout as
//!   the whole-repo path.
//! - returns a `ModelDir` whose `kind == HuggingFace` and whose
//!   `quantization` is derived from the filename.
//!
//! Uses a synthetic ~1 KB payload for the ggml file (real ggml-tiny
//! is ~75 MB, same pattern as the safetensors tests).

mod common;

use common::{resolve_ok, source_for};
use voxora_core::{ModelSourceKind, Quantization};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

const ORG: &str = "ggerganov";
const REPO: &str = "whisper.cpp";
const FILE: &str = "ggml-tiny.bin";
const MODEL_ID: &str = "ggerganov/whisper.cpp/ggml-tiny.bin";

/// Synthetic 1 KB payload that stands in for a ggml model file.
fn synthetic_ggml(label: &str) -> Vec<u8> {
    let mut v = vec![0u8; 1024];
    let stamp = format!("voxora-ggml-{label}");
    let bytes = stamp.as_bytes();
    v[..bytes.len()].copy_from_slice(bytes);
    v
}

#[tokio::test]
async fn resolve_single_file_downloads_ggml_bin() {
    let mock = wiremock::MockServer::start().await;

    // The single-file path MUST NOT hit the metadata endpoint — that
    // call assumes a multi-file repo and would 404 on the
    // ggerganov/whisper.cpp layout. We mount it anyway with a 500 so
    // any accidental hit becomes a test failure rather than a silent
    // success.
    Mock::given(method("GET"))
        .and(path(format!("/api/models/{ORG}/{REPO}/revision/main")))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&mock)
        .await;

    // The single download we expect.
    Mock::given(method("GET"))
        .and(path(format!("/{ORG}/{REPO}/resolve/main/{FILE}")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(synthetic_ggml("tiny")))
        .expect(1)
        .mount(&mock)
        .await;

    let (_cache, src) = source_for(&mock, None).await;
    let dir = resolve_ok(&src, MODEL_ID).await;

    assert!(dir.path.exists(), "model dir must exist on disk");
    assert!(
        dir.path.join(".complete").is_file(),
        "marker missing at {}",
        dir.path.join(".complete").display()
    );
    assert!(
        dir.path.join(FILE).is_file(),
        "downloaded file missing at {}",
        dir.path.join(FILE).display()
    );

    // The file content should match the synthetic payload.
    let on_disk = std::fs::read(dir.path.join(FILE)).expect("read downloaded file");
    assert_eq!(on_disk, synthetic_ggml("tiny"));

    assert_eq!(ModelSourceKind::HuggingFace, dir.kind, "kind must be HF");
    // ggml-tiny.bin has no q4/q8 suffix — quantization defaults to
    // F16 per the from_gguf_filename rule.
    assert_eq!(
        Quantization::F16,
        dir.quantization,
        "expected F16 for an unquantized ggml filename"
    );
}

#[tokio::test]
async fn resolve_single_file_uses_cache_on_second_call() {
    let mock = wiremock::MockServer::start().await;

    // The download endpoint: mounted with expect(1) — a second
    // resolve() call MUST hit the cache instead of refetching.
    Mock::given(method("GET"))
        .and(path(format!("/{ORG}/{REPO}/resolve/main/{FILE}")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(synthetic_ggml("tiny")))
        .expect(1)
        .mount(&mock)
        .await;

    let (_cache, src) = source_for(&mock, None).await;
    let _ = resolve_ok(&src, MODEL_ID).await;
    let _ = resolve_ok(&src, MODEL_ID).await;

    // If the second call had refetched, wiremock would have made the
    // test fail because of the `expect(1)`. Reaching here means both
    // calls came from the cache.
}

#[tokio::test]
async fn resolve_single_file_detects_q4_quantization_from_filename() {
    let mock = wiremock::MockServer::start().await;

    let q4_file = "ggml-base.bin.q4_K_M";
    Mock::given(method("GET"))
        .and(path(format!("/{ORG}/{REPO}/resolve/main/{q4_file}")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(synthetic_ggml("q4")))
        .expect(1)
        .mount(&mock)
        .await;

    let (_cache, src) = source_for(&mock, None).await;
    let dir = resolve_ok(&src, "ggerganov/whisper.cpp/ggml-base.bin.q4_K_M").await;

    assert_eq!(
        Quantization::Q4K,
        dir.quantization,
        "Q4_K_M filename must surface as Q4K"
    );
}

#[tokio::test]
async fn capabilities_for_single_file_does_not_hit_config_json_endpoint() {
    let mock = wiremock::MockServer::start().await;

    // The single-file path MUST NOT hit the metadata endpoint or
    // fetch_file_text on `config.json`. Both would 404 because the
    // 3-segment id is not a real repo. We mount them with a 500
    // sentinel and `expect(0)` so any accidental hit fails the test.
    Mock::given(method("GET"))
        .and(path(format!("/api/models/{ORG}/{REPO}/revision/main")))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/{ORG}/{REPO}/resolve/main/config.json")))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&mock)
        .await;

    let (_cache, src) = source_for(&mock, None).await;
    let caps = voxora_core::ModelSource::capabilities_for(&src, MODEL_ID)
        .await
        .expect("capabilities_for should not hit the network");

    // Synthesised from the filename: ggml-tiny.bin is multilingual
    // Whisper (the canonical whisper.cpp tiny checkpoint).
    assert!(caps.multilingual);
    assert!(caps.word_timestamps);
    assert!(!caps.streaming);
}

#[tokio::test]
async fn capabilities_for_single_file_flags_english_only() {
    let mock = wiremock::MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&mock)
        .await;

    let (_cache, src) = source_for(&mock, None).await;
    let caps =
        voxora_core::ModelSource::capabilities_for(&src, "ggerganov/whisper.cpp/ggml-tiny.en.bin")
            .await
            .expect("capabilities_for");

    assert!(!caps.multilingual, ".en. file must be English-only");
    assert_eq!(caps.languages, vec!["en".to_string()]);
}
