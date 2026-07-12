//! `capabilities_for` must read `config.json` directly without
//! downloading any weights and without hitting the metadata endpoint.

mod common;

use common::{read_fixture, source_for};
use voxora_core::ModelSource;
use voxora_hf::HuggingFaceSource;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

const QWEN_ID: &str = "Qwen/Qwen3-ASR-0.6B";
const WHISPER_ID: &str = "openai/whisper-tiny";

#[tokio::test]
async fn capabilities_for_qwen3_returns_multilingual_languages() {
    let mock = wiremock::MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/{QWEN_ID}/resolve/main/config.json")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(read_fixture("qwen3-asr-0.6b", "config.json")),
        )
        .expect(1)
        .mount(&mock)
        .await;

    let (_cache, src) = source_for(&mock, None).await;
    let caps = ModelSource::capabilities_for(&src, QWEN_ID)
        .await
        .expect("capabilities_for should succeed");

    assert!(caps.multilingual, "qwen3 is multilingual");
    assert!(!caps.languages.is_empty(), "languages must be filled");
    assert!(
        caps.languages.iter().any(|l| l == "english"),
        "got {:?}",
        caps.languages
    );
}

#[tokio::test]
async fn capabilities_for_whisper_enables_word_timestamps() {
    let mock = wiremock::MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/{WHISPER_ID}/resolve/main/config.json")))
        .respond_with(
            ResponseTemplate::new(200).set_body_bytes(read_fixture("whisper-tiny", "config.json")),
        )
        .expect(1)
        .mount(&mock)
        .await;

    let (_cache, src) = source_for(&mock, None).await;
    let caps = ModelSource::capabilities_for(&src, WHISPER_ID)
        .await
        .expect("capabilities_for should succeed");

    assert!(caps.multilingual);
    assert!(caps.word_timestamps, "whisper has word timestamps");
    assert!(!caps.languages.is_empty());
}

#[tokio::test]
async fn capabilities_for_does_not_fetch_weights_or_metadata() {
    // We only mount /resolve/main/config.json. Any other request would
    // hit wiremock's default 404, but `received_requests` will let us
    // see what *was* called.
    let mock = wiremock::MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/{QWEN_ID}/resolve/main/config.json")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(read_fixture("qwen3-asr-0.6b", "config.json")),
        )
        .mount(&mock)
        .await;

    let (_cache, src): (tempfile::TempDir, HuggingFaceSource) = source_for(&mock, None).await;
    let _ = ModelSource::capabilities_for(&src, QWEN_ID)
        .await
        .expect("capabilities_for should succeed");

    let received = mock.received_requests().await.unwrap_or_default();
    assert_eq!(
        received.len(),
        1,
        "exactly one request expected (config.json), got {}",
        received.len()
    );
    assert!(received[0].url.path().ends_with("/config.json"));
}
