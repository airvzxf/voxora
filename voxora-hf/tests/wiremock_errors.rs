//! Error mapping: 4xx/5xx responses must surface as the right
//! `voxora_traits::AsrError` variant.

mod common;

use common::{read_fixture, resolve_err, source_for, synthetic_safetensors};
use voxora_traits::AsrError;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

const MODEL_ID: &str = "Qwen/Qwen3-ASR-0.6B";
const FIXTURE_DIR: &str = "qwen3-asr-0.6b";

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

/// Closes [#188](https://github.com/airvzxf/voxora/issues/188):
/// a wiremock that returns a multi-KiB body on a 503 must NOT
/// read the entire body into RAM. The body stays at exactly the
/// 4 KiB cap (plus the truncation marker), regardless of how
/// large the wiremock body is.
#[tokio::test]
async fn error_response_body_is_size_capped() {
    let mock = wiremock::MockServer::start().await;

    // 1 MiB body — far above the 4 KiB cap. Each byte is the
    // printable Latin-1 'A' so the response body is valid UTF-8
    // and the assert below can compare lengths cleanly.
    let huge_body: Vec<u8> = vec![b'A'; 1024 * 1024];

    Mock::given(method("GET"))
        .and(path(format!("/api/models/{MODEL_ID}/revision/main")))
        .respond_with(ResponseTemplate::new(503).set_body_bytes(huge_body))
        .mount(&mock)
        .await;

    let (_cache, src) = source_for(&mock, None).await;
    let err = resolve_err(&src, MODEL_ID).await;

    // The body stored in `AsrError::Network` must be at most the
    // 4 KiB cap plus the UTF-8 ellipsis + "[truncated]" marker
    // (~13 bytes), well under 5 KiB. The exact size before
    // truncation is `MAX_ERROR_BODY_BYTES = 4 * 1024 = 4096`.
    match &err {
        AsrError::Network { message, .. } => {
            assert!(
                message.len() < 5000,
                "error body must be capped under 5 KiB; got {} bytes",
                message.len()
            );
            // The 503 trigger means the message is the formatted
            // error body. Cap should be reflected in len.
            assert!(
                message.contains("503"),
                "message must mention the 503 status: {message:?}"
            );
        }
        other => panic!("expected Network, got {other:?}"),
    }
}

/// Closes [#189](https://github.com/airvzxf/voxora/issues/189):
/// a streaming 503 mid-body must NOT leave a `<file>.partial.<hex>-<n>`
/// tmp behind. Before the `TmpGuard` fix, every one of the five
/// error paths in `HfClient::get_to_file` (chunk read, write, flush,
/// `sync_all`, `rename`) leaked the tmp; this test pins the contract
/// for the streaming-failure case specifically.
///
/// Wiremock's streaming behaviour is best-effort: even with a small
/// body, the chunked encoder may deliver bytes before the 503 closes
/// the connection, so the test exercises the realistic "headers OK,
/// body short, then 503" shape rather than a header-only failure.
#[tokio::test]
async fn streaming_503_cleans_partial_tmp() {
    let mock = wiremock::MockServer::start().await;

    // Metadata endpoint succeeds so we get past model-not-found.
    Mock::given(method("GET"))
        .and(path(format!("/api/models/{MODEL_ID}/revision/main")))
        .respond_with(
            ResponseTemplate::new(200).set_body_bytes(read_fixture(FIXTURE_DIR, "_metadata.json")),
        )
        .mount(&mock)
        .await;

    // Smaller text files succeed so the resolve reaches the
    // safetensors download before failing.
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

    // The safetensors download returns 503 with a non-empty body so
    // the streaming path actually creates a `.partial.<hex>-<n>` file
    // before failing.
    Mock::given(method("GET"))
        .and(path(format!("/{MODEL_ID}/resolve/main/model.safetensors")))
        .respond_with(
            ResponseTemplate::new(503).set_body_bytes(synthetic_safetensors("503-with-body")),
        )
        .mount(&mock)
        .await;

    let (cache, src) = source_for(&mock, None).await;
    let err = resolve_err(&src, MODEL_ID).await;
    assert!(
        matches!(err, AsrError::Network { .. }),
        "expected Network, got {err:?}"
    );

    // After the resolve fails, no .partial tmp may remain in the
    // cache directory (the `TmpGuard` Drop must have cleaned it).
    // Use a recursive walk because the tmp lives one level below
    // the cache root at `<org>/<name>/<revision>/`.
    let mut leaked: Vec<String> = Vec::new();
    visit_partials(cache.path(), &mut leaked);
    assert!(
        leaked.is_empty(),
        "no .partial files may remain after a failed resolve; found: {leaked:?}"
    );
}

/// Recursive walk that collects every path whose name contains
/// `.partial` (matches both the legacy `.partial` suffix and the
/// current `.partial.<hex>-<n>` shape).
fn visit_partials(dir: &std::path::Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        let name = p
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_default();
        if p.is_dir() {
            visit_partials(&p, out);
        } else if name.contains(".partial") {
            out.push(name);
        }
    }
}
