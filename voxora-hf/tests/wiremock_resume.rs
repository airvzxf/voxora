//! Resuming an interrupted download: files already in the cache
//! directory (no `.complete` marker) are reused only when they are
//! the size the Hub declares; anything else is downloaded again.

mod common;

use common::{read_fixture, source_for, synthetic_safetensors};
use wiremock::matchers::{method, path, query_param};
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

/// The recorded metadata, with the `size` the Hub only reports for
/// `?blobs=true` requests filled in from the bytes the mock serves.
fn metadata_with_sizes() -> Vec<u8> {
    let mut meta: serde_json::Value =
        serde_json::from_slice(&read_fixture(FIXTURE_DIR, "_metadata.json")).unwrap();
    for sibling in meta["siblings"].as_array_mut().unwrap() {
        let name = sibling["rfilename"].as_str().unwrap().to_string();
        let size = if name == "model.safetensors" {
            synthetic_safetensors("full").len()
        } else if TEXT_FILES.contains(&name.as_str()) {
            read_fixture(FIXTURE_DIR, &name).len()
        } else {
            1
        };
        sibling["size"] = size.into();
    }
    serde_json::to_vec(&meta).unwrap()
}

#[tokio::test]
async fn resume_downloads_again_a_file_of_the_wrong_size() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/api/models/{MODEL_ID}/revision/main")))
        .and(query_param("blobs", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(metadata_with_sizes()))
        .mount(&mock)
        .await;
    // Already on disk at the declared size: must not be fetched again.
    for fname in TEXT_FILES {
        Mock::given(method("GET"))
            .and(path(format!("/{MODEL_ID}/resolve/main/{fname}")))
            .respond_with(
                ResponseTemplate::new(200).set_body_bytes(read_fixture(FIXTURE_DIR, fname)),
            )
            .expect(0)
            .mount(&mock)
            .await;
    }
    Mock::given(method("GET"))
        .and(path(format!("/{MODEL_ID}/resolve/main/model.safetensors")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(synthetic_safetensors("full")))
        .expect(1)
        .mount(&mock)
        .await;

    let (cache, src) = source_for(&mock, None).await;
    let dir = cache.path().join(MODEL_ID).join("main");
    std::fs::create_dir_all(&dir).unwrap();
    for fname in TEXT_FILES {
        std::fs::write(dir.join(fname), read_fixture(FIXTURE_DIR, fname)).unwrap();
    }
    std::fs::write(
        dir.join("model.safetensors"),
        &synthetic_safetensors("full")[..256],
    )
    .unwrap();

    let resolved = common::resolve_ok(&src, MODEL_ID).await;

    let on_disk = std::fs::read(resolved.path.join("model.safetensors")).unwrap();
    assert_eq!(on_disk.len(), 1024, "the truncated file must be replaced");
    assert!(on_disk == synthetic_safetensors("full"));
    assert!(resolved.path.join(".complete").is_file());
}

#[tokio::test]
async fn single_file_resume_replaces_a_leftover_file() {
    const FILE_ID: &str = "ggerganov/whisper.cpp/ggml-tiny.bin";
    let full = synthetic_safetensors("ggml");
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(full.clone()))
        .expect(1)
        .mount(&mock)
        .await;

    let (cache, src) = source_for(&mock, None).await;
    let dir = cache.path().join("ggerganov/whisper.cpp/main");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("ggml-tiny.bin"), &full[..256]).unwrap();

    let resolved = common::resolve_ok(&src, FILE_ID).await;

    let on_disk = std::fs::read(resolved.entry.unwrap()).unwrap();
    assert_eq!(on_disk.len(), 1024, "the leftover file must be replaced");
    assert!(on_disk == full);
}
