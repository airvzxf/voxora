//! `ResolveOptions::max_bytes` when the Hub declares no file sizes
//! (single-file ids never ask for metadata; legacy or private repos
//! may omit `size`): the download itself must stop at the cap.

mod common;

use common::{read_fixture, source_for, synthetic_safetensors};
use voxora_traits::{AsrError, ModelSource, ResolveOptions};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const MODEL_ID: &str = "Qwen/Qwen3-ASR-0.6B";
const FIXTURE_DIR: &str = "qwen3-asr-0.6b";
const TEXT_FILES: [&str; 5] = [
    "config.json",
    "preprocessor_config.json",
    "tokenizer_config.json",
    "vocab.json",
    "merges.txt",
];

fn leftovers(dir: &std::path::Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n != ".lock")
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn single_file_larger_than_max_bytes_is_rejected() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![7u8; 2048]))
        .mount(&mock)
        .await;
    let (cache, src) = source_for(&mock, None).await;

    let err = src
        .resolve(
            "ggerganov/whisper.cpp/ggml-tiny.bin",
            &ResolveOptions::with_max_bytes(1024),
        )
        .await
        .expect_err("2048 bytes must exceed a 1024-byte cap");

    assert!(matches!(err, AsrError::InvalidInput(_)), "{err:?}");
    let dir = cache.path().join("ggerganov/whisper.cpp/main");
    assert_eq!(leftovers(&dir), Vec::<String>::new());
}

/// Mounts the recorded metadata (every `size` is `null`) and the
/// files of a whole-repo resolve.
async fn mount_sizeless_repo(mock: &MockServer) {
    Mock::given(method("GET"))
        .and(path(format!("/api/models/{MODEL_ID}/revision/main")))
        .respond_with(
            ResponseTemplate::new(200).set_body_bytes(read_fixture(FIXTURE_DIR, "_metadata.json")),
        )
        .mount(mock)
        .await;
    for fname in TEXT_FILES {
        Mock::given(method("GET"))
            .and(path(format!("/{MODEL_ID}/resolve/main/{fname}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_bytes(read_fixture(FIXTURE_DIR, fname)),
            )
            .mount(mock)
            .await;
    }
    Mock::given(method("GET"))
        .and(path(format!("/{MODEL_ID}/resolve/main/model.safetensors")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(synthetic_safetensors("w")))
        .mount(mock)
        .await;
}

fn repo_sizes() -> Vec<u64> {
    let mut sizes: Vec<u64> = TEXT_FILES
        .iter()
        .map(|f| read_fixture(FIXTURE_DIR, f).len() as u64)
        .collect();
    sizes.push(synthetic_safetensors("w").len() as u64);
    sizes
}

#[tokio::test]
async fn repo_file_larger_than_max_bytes_is_rejected() {
    let mock = MockServer::start().await;
    mount_sizeless_repo(&mock).await;
    let (cache, src) = source_for(&mock, None).await;
    let largest = *repo_sizes().iter().max().unwrap();

    let err = src
        .resolve(MODEL_ID, &ResolveOptions::with_max_bytes(largest - 1))
        .await
        .expect_err("the largest file exceeds the cap");

    assert!(matches!(err, AsrError::InvalidInput(_)), "{err:?}");
    assert!(!cache.path().join(MODEL_ID).join("main/.complete").exists());
}

#[tokio::test]
async fn repo_whose_total_exceeds_max_bytes_is_rejected() {
    let mock = MockServer::start().await;
    mount_sizeless_repo(&mock).await;
    let (cache, src) = source_for(&mock, None).await;
    let sizes = repo_sizes();
    let cap = *sizes.iter().max().unwrap();
    assert!(sizes.iter().sum::<u64>() > cap);

    let err = src
        .resolve(MODEL_ID, &ResolveOptions::with_max_bytes(cap))
        .await
        .expect_err("every file fits but the total does not");

    assert!(matches!(err, AsrError::InvalidInput(_)), "{err:?}");
    assert!(!cache.path().join(MODEL_ID).join("main/.complete").exists());
}
