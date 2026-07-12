//! `list_available` is curated (no HTTP) in Phase 2.

mod common;

use common::source_for;
use voxora_core::ModelSource;

#[tokio::test]
async fn list_available_returns_curated_models() {
    let mock = wiremock::MockServer::start().await;
    let (_cache, src) = source_for(&mock, None).await;

    let list = ModelSource::list_available(&src)
        .await
        .expect("list_available should succeed");

    assert!(!list.is_empty(), "curated list is non-empty");
    let ids: Vec<&str> = list.iter().map(|d| d.id.as_str()).collect();
    assert!(ids.contains(&"Qwen/Qwen3-ASR-0.6B"));
    assert!(ids.contains(&"Qwen/Qwen3-ASR-1.7B"));
    assert!(ids.contains(&"openai/whisper-tiny"));
    assert!(ids.contains(&"ggerganov/whisper.cpp"));

    // No HTTP traffic: wiremock's received_requests must be empty.
    let received = mock.received_requests().await.unwrap_or_default();
    assert!(
        received.is_empty(),
        "list_available should not hit the network (saw {} calls)",
        received.len()
    );
}

#[tokio::test]
async fn list_available_descriptors_carry_capabilities() {
    let mock = wiremock::MockServer::start().await;
    let (_cache, src) = source_for(&mock, None).await;

    let list = ModelSource::list_available(&src).await.unwrap();
    let qwen = list
        .iter()
        .find(|d| d.id == "Qwen/Qwen3-ASR-0.6B")
        .expect("qwen entry");
    let caps = qwen.capabilities.as_ref().expect("qwen has capabilities");
    assert!(caps.multilingual);
    assert!(!caps.languages.is_empty());
}
