//! Bearer-token semantics: when a token is configured, it must reach
//! the server as `Authorization: Bearer …`; when it isn't, the header
//! must be absent.

mod common;

use common::{read_fixture, resolve_ok, source_for, synthetic_safetensors};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, ResponseTemplate};

const MODEL_ID: &str = "Qwen/Qwen3-ASR-0.6B";
const FIXTURE_DIR: &str = "qwen3-asr-0.6b";

#[tokio::test]
async fn token_is_sent_in_authorization_header() {
    let mock = wiremock::MockServer::start().await;

    // Mock that matches ONLY when the bearer header is present.
    Mock::given(method("GET"))
        .and(path(format!("/api/models/{MODEL_ID}/revision/main")))
        .and(header("Authorization", "Bearer hf_secret"))
        .respond_with(
            ResponseTemplate::new(200).set_body_bytes(read_fixture(FIXTURE_DIR, "_metadata.json")),
        )
        // If the bearer header isn't sent, this mock never matches,
        // and wiremock panics on shutdown because `expect(1)` fails.
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
            .mount(&mock)
            .await;
    }
    Mock::given(method("GET"))
        .and(path(format!("/{MODEL_ID}/resolve/main/model.safetensors")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(synthetic_safetensors("token")))
        .mount(&mock)
        .await;

    let (_cache, src) = source_for(&mock, Some("hf_secret")).await;
    let _ = resolve_ok(&src, MODEL_ID).await;
}

#[tokio::test]
async fn no_token_means_no_authorization_header() {
    let mock = wiremock::MockServer::start().await;

    // Mock that 401s ONLY when a bearer header is present.
    Mock::given(method("GET"))
        .and(path(format!("/api/models/{MODEL_ID}/revision/main")))
        .and(header("Authorization", "Bearer X"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock)
        .await;

    Mock::given(method("GET"))
        .and(path(format!("/api/models/{MODEL_ID}/revision/main")))
        .respond_with(
            ResponseTemplate::new(200).set_body_bytes(read_fixture(FIXTURE_DIR, "_metadata.json")),
        )
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
            .mount(&mock)
            .await;
    }
    Mock::given(method("GET"))
        .and(path(format!("/{MODEL_ID}/resolve/main/model.safetensors")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(synthetic_safetensors("anon")))
        .mount(&mock)
        .await;

    let (_cache, src) = source_for(&mock, None).await;
    let _ = resolve_ok(&src, MODEL_ID).await;
}
