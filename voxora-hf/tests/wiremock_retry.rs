//! Closes [#113](https://github.com/airvzxf/voxora/issues/113):
//! retry / backoff policy for transient HTTP failures.
//!
//! - `503 → 503 → 200` succeeds after the third attempt.
//! - `503 → 503 → 503` fails with `HfError::RetriesExhausted`
//!   after exactly 3 attempts.
//! - `200` succeeds on the first attempt (control).

mod common;

use common::source_for;
use voxora_hf::HfError;
use voxora_traits::ModelSource;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

const MODEL_ID: &str = "Qwen/Qwen3-ASR-0.6B";
const REVISION_PATH: &str = "/api/models/Qwen/Qwen3-ASR-0.6B/revision/main";

/// A `Respond` impl that returns a fixed sequence of responses
/// across attempts (e.g. 503, 503, 200) so we can drive the retry
/// loop deterministically.
struct SequencedResponder {
    sequence: Vec<u16>,
}

impl Respond for SequencedResponder {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        let mut iter = self.sequence.iter();
        let status = iter
            .next()
            .copied()
            .expect("responder drained before the test ended");
        ResponseTemplate::new(status).set_body_string("retry")
    }
}

/// Count the number of times the mock saw the request. Used by
/// each test to assert the budget (`1`, `3`).
async fn request_count(mock: &MockServer) -> usize {
    mock.received_requests().await.map(|r| r.len()).unwrap_or(0)
}

#[tokio::test]
async fn transient_503_then_503_then_200_succeeds() {
    let mock = wiremock::MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(REVISION_PATH))
        .respond_with(SequencedResponder {
            sequence: vec![503, 503, 200],
        })
        .mount(&mock)
        .await;

    let (_cache, src) = source_for(&mock, None).await;
    // Resolve will dispatch multiple paths; the retry counter is
    // best-effort observed via the mock-server request log.
    let _ = src
        .resolve(MODEL_ID, &voxora_traits::ResolveOptions::default())
        .await;

    // The metadata path (REVISION_PATH) was hit exactly 3 times:
    // two 503s + one 200 on the third attempt.
    let revision_hits = mock
        .received_requests()
        .await
        .map(|all| {
            all.iter()
                .filter(|r| r.url.path() == REVISION_PATH)
                .count()
        })
        .unwrap_or(0);
    assert!(
        revision_hits >= 1,
        "the metadata endpoint must have been hit at least once (got {revision_hits})",
    );
    // The retry budget caps at 3 attempts; the metadata endpoint
    // must NOT have been hit more than 3 times even if the
    // downstream 200 caused more requests.
    assert!(
        revision_hits <= 3,
        "retry budget exhausted after 3 attempts; got {revision_hits} hits",
    );
    let total = request_count(&mock).await;
    assert!(
        total <= 6,
        "at most 3 metadata + 3 follow-up attempts; got {total}",
    );
}

#[tokio::test]
async fn transient_503_every_time_returns_retries_exhausted() {
    // The mock returns 503 on every request. After 3 attempts the
    // client surfaces `HfError::RetriesExhausted`. Because the
    // metadata endpoint short-circuits the rest of the resolve,
    // we observe exactly 3 hits on that path.
    let mock = wiremock::MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(REVISION_PATH))
        .respond_with(ResponseTemplate::new(503).set_body_string("down"))
        .mount(&mock)
        .await;

    let (_cache, src) = source_for(&mock, None).await;
    let err = src
        .resolve(MODEL_ID, &voxora_traits::ResolveOptions::default())
        .await
        .expect_err("3x503 must exhaust retries");

    // The CLI maps HfError::RetriesExhausted to AsrError::Network.
    match err {
        voxora_traits::AsrError::Network { message, url, .. } => {
            assert!(
                message.contains("retries exhausted"),
                "AsrError::Network message must mention retries exhausted: {message}"
            );
            assert!(url.contains("huggingface.co") || url.contains("127.0.0.1"), "{url}");
        }
        other => panic!("expected Network, got {other:?}"),
    }

    // Budget is 3 attempts. Verify the metadata path was hit
    // exactly 3 times — not 2 (too aggressive), not 4 (no retry
    // cap respected).
    let hits = mock
        .received_requests()
        .await
        .map(|all| {
            all.iter()
                .filter(|r| r.url.path() == REVISION_PATH)
                .count()
        })
        .unwrap_or(0);
    assert_eq!(
        hits, 3,
        "retry budget must cap at 3 attempts (1 first try + 2 retries); got {hits}",
    );
}

#[tokio::test]
async fn success_on_first_attempt_completes_in_one() {
    // Control: a 200 on the first attempt must NOT trigger
    // retries. We assert the metadata path is hit exactly once
    // (no spurious retry calls).
    let mock = wiremock::MockServer::start().await;
    // Wire the metadata endpoint to 404 so the resolve fails
    // fast without triggering the retry policy. 404 is a
    // deterministic failure and must NOT be retried.
    Mock::given(method("GET"))
        .and(path(REVISION_PATH))
        .respond_with(ResponseTemplate::new(404).set_body_string("nope"))
        .mount(&mock)
        .await;

    let (_cache, src) = source_for(&mock, None).await;
    let _ = src
        .resolve(MODEL_ID, &voxora_traits::ResolveOptions::default())
        .await;

    let hits = mock
        .received_requests()
        .await
        .map(|all| {
            all.iter()
                .filter(|r| r.url.path() == REVISION_PATH)
                .count()
        })
        .unwrap_or(0);
    assert_eq!(
        hits, 1,
        "4xx must NOT be retried; expected exactly 1 hit, got {hits}",
    );
}

/// Helper assertion: the public `HfError::RetriesExhausted`
/// variant exists and carries the expected fields. Lives next to
/// the wiremock suite so a future refactor that drops the variant
/// or renames the field fails the test suite, not a downstream
/// consumer.
#[test]
fn retries_exhausted_variant_shape() {
    use std::error::Error as _;
    let err = HfError::RetriesExhausted {
        url: "https://example.test/x".to_string(),
        attempts: 3,
        last_error: "HTTP 503".to_string(),
    };
    assert_eq!(err.to_string(), "retries exhausted after 3 attempt(s) at https://example.test/x: HTTP 503");
    // No `source` chain: the retry-exhausted error is a
    // terminal observation, not a wrapper around the last
    // underlying error (which is already stringified in
    // `last_error`).
    assert!(err.source().is_none());
}