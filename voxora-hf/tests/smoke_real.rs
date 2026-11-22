//! Live smoke tests against `huggingface.co`. Gated by `#[ignore]` so
//! they do not run in CI; enable with `cargo test -p voxora-hf
//! --test smoke_real -- --ignored`.
//!
//! These exist to verify that the real HF API contract still matches
//! the recordings baked into `tests/fixtures/`. They are slow (one
//! round-trip per model) and depend on HF being reachable, so they
//! stay opt-in.

mod common;

use voxora_traits::ModelSource;

const QWEN_ID: &str = "Qwen/Qwen3-ASR-0.6B";

#[tokio::test]
#[ignore = "hits live HF API; run with --ignored"]
async fn smoke_qwen3_capabilities() {
    let src = voxora_hf::HuggingFaceSource::new().expect("source build should succeed");
    let caps = src.capabilities_for(QWEN_ID).await.expect("caps ok");
    assert!(caps.multilingual, "qwen3 is multilingual");
    assert!(!caps.languages.is_empty());
    println!(
        "qwen3 languages (first 5): {:?}",
        &caps.languages[..5.min(caps.languages.len())]
    );
}

#[tokio::test]
#[ignore = "hits live HF API; run with --ignored"]
async fn smoke_qwen3_metadata() {
    let src = voxora_hf::HuggingFaceSource::new().expect("source build should succeed");
    let list = src.list_available().await.expect("list ok");
    assert!(!list.is_empty());
    println!("curated models: {}", list.len());
}

/// The Hub only declares file sizes for `?blobs=true` metadata
/// requests; without them `max_bytes` cannot stop a download before
/// it starts.
#[tokio::test]
#[ignore = "hits live HF API; run with --ignored"]
async fn smoke_max_bytes_rejects_before_downloading_weights() {
    let cache = tempfile::tempdir().expect("tempdir");
    let src = voxora_hf::HuggingFaceSource::builder()
        .cache_dir(cache.path().to_path_buf())
        .build()
        .expect("source build should succeed");
    let opts = voxora_traits::ResolveOptions::with_max_bytes(1024 * 1024);

    let err = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        src.resolve(QWEN_ID, &opts),
    )
    .await
    .expect("must fail fast, not download ~1.9 GB")
    .expect_err("a ~1.9 GB model must exceed a 1 MiB cap");

    assert!(
        matches!(err, voxora_traits::AsrError::InvalidInput(_)),
        "{err:?}"
    );
    assert!(
        !cache
            .path()
            .join(QWEN_ID)
            .join("main/model.safetensors")
            .exists()
    );
}
