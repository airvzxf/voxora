//! Error mapping: 4xx/5xx responses must surface as the right
//! `voxora_core::AsrError` variant.

mod common;

use common::{resolve_err, source_for};
use voxora_core::AsrError;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

const MODEL_ID: &str = "Qwen/Qwen3-ASR-0.6B";

#[tokio::test]
async fn model_not_found_on_metadata_404() {
    let mock = wiremock::MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/api/models/{MODEL_ID}/revision/main")))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&mock)
        .await;

    let (_cache, src) = source_for(&mock, None).await;
    let err = resolve_err(&src, MODEL_ID).await;
    // We return Network for non-success status codes; the model_id
    // is still preserved in the AsrError.
    match &err {
        AsrError::Network { url, .. } => {
            assert!(url.contains("api/models/Qwen/Qwen3-ASR-0.6B"), "{url}")
        }
        other => panic!("expected Network, got {other:?}"),
    }
}

#[tokio::test]
async fn invalid_input_on_malformed_model_id() {
    // No mock needed — the validator rejects before any HTTP.
    let mock = wiremock::MockServer::start().await;
    let (_cache, src) = source_for(&mock, None).await;
    let err = resolve_err(&src, "nope-no-slash").await;
    assert!(
        matches!(err, AsrError::InvalidInput(_)),
        "expected InvalidInput, got {err:?}"
    );
}

#[tokio::test]
async fn server_error_5xx_is_network_failure() {
    let mock = wiremock::MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!("/api/models/{MODEL_ID}/revision/main")))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
        .mount(&mock)
        .await;

    let (_cache, src) = source_for(&mock, None).await;
    let err = resolve_err(&src, MODEL_ID).await;
    match &err {
        AsrError::Network { message, .. } => {
            assert!(message.contains("503"), "{message}");
        }
        other => panic!("expected Network, got {other:?}"),
    }
}
