//! Concurrent resolves to the same model against a single wiremock
//! server. We use `expect(N)` to confirm each file is downloaded
//! the expected number of times. Multiple tasks racing the same
//! `(model_id, revision)` directory is the realistic case from a
//! shared CLI session.

mod common;

use common::{read_fixture, source_for, synthetic_safetensors};
use std::sync::Arc;
use voxora_traits::{ModelSource, ResolveOptions};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

const MODEL_ID: &str = "Qwen/Qwen3-ASR-0.6B";
const FIXTURE_DIR: &str = "qwen3-asr-0.6b";
const CONCURRENCY: usize = 4;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parallel_resolves_complete_without_corruption() {
    let mock = wiremock::MockServer::start().await;

    // Allow enough invocations: at most CONCURRENCY per file.
    Mock::given(method("GET"))
        .and(path(format!("/api/models/{MODEL_ID}/revision/main")))
        .respond_with(
            ResponseTemplate::new(200).set_body_bytes(read_fixture(FIXTURE_DIR, "_metadata.json")),
        )
        .expect(1..=CONCURRENCY as u64)
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
            .expect(1..=CONCURRENCY as u64)
            .mount(&mock)
            .await;
    }
    Mock::given(method("GET"))
        .and(path(format!("/{MODEL_ID}/resolve/main/model.safetensors")))
        .respond_with(
            ResponseTemplate::new(200).set_body_bytes(synthetic_safetensors("concurrent")),
        )
        .expect(1..=CONCURRENCY as u64)
        .mount(&mock)
        .await;

    let (_cache, src) = source_for(&mock, None).await;
    let src = Arc::new(src);

    let mut handles = Vec::new();
    for _ in 0..CONCURRENCY {
        let s = Arc::clone(&src);
        handles.push(tokio::spawn(async move {
            s.resolve(MODEL_ID, &ResolveOptions::default())
                .await
                .map_err(|e| e.to_string())
        }));
    }

    for h in handles {
        let dir = h.await.expect("task did not panic").expect("resolve ok");
        assert!(dir.path.join(".complete").is_file(), "marker present");
        assert!(dir.path.join("config.json").is_file());
        assert!(dir.path.join("model.safetensors").is_file());
    }
}

/// Regression for #103 — `HfClient::get_to_file` used to derive its
/// tmp-file suffix from `SystemTime::now().as_nanos()`, which is not
/// unique across two concurrent calls. The single-file resolve path
/// funnels straight into `get_to_file`, so two parallel resolves
/// against a fresh cache (a) must both return `Ok`, (b) must leave
/// the destination file on disk, and (c) must leave its bytes equal
/// to the wiremock body — i.e. one writer's truncated bytes do not
/// race over the other's in-flight bytes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn parallel_resolves_concurrent_tmp_suffix_race() {
    const ORG: &str = "ggerganov";
    const REPO: &str = "whisper.cpp";
    const FILE: &str = "ggml-tiny.bin";
    const MODEL_ID_3SEG: &str = "ggerganov/whisper.cpp/ggml-tiny.bin";

    let mock = wiremock::MockServer::start().await;

    let body: Vec<u8> = (0..2048u32).map(|i| (i & 0xff) as u8).collect();
    Mock::given(method("GET"))
        .and(path(format!("/{ORG}/{REPO}/resolve/main/{FILE}")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
        // Two parallel resolves both reach the download — at most 2
        // hits; a third would mean a writer resurrected after a fail
        // and we want the test to flag that.
        .expect(1..=2)
        .mount(&mock)
        .await;

    let (_cache, src) = source_for(&mock, None).await;
    let src = Arc::new(src);

    let (a, b) = tokio::try_join!(
        async {
            src.resolve(MODEL_ID_3SEG, &ResolveOptions::default())
                .await
        },
        async {
            src.resolve(MODEL_ID_3SEG, &ResolveOptions::default())
                .await
        },
    )
    .expect("both resolves returned Ok");

    let dir_a = a.path.clone();
    let dir_b = b.path.clone();
    assert_eq!(dir_a, dir_b, "both resolves share the cache dir");

    let dest = dir_a.join(FILE);
    assert!(dest.is_file(), "dest file missing after parallel resolves");

    let on_disk = std::fs::read(&dest).expect("read dest");
    assert_eq!(
        on_disk, body,
        "dest bytes must equal the wiremock body (no interleaved writer)"
    );

    assert!(
        dir_a.join(".complete").is_file(),
        "complete marker must be present"
    );
}
