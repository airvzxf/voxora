//! Live smoke tests against `huggingface.co`. Gated by `#[ignore]` so
//! they do not run in CI; enable with `cargo test -p voxora-hf
//! --test smoke_real -- --ignored`.
//!
//! These exist to verify that the real HF API contract still matches
//! the recordings baked into `tests/fixtures/`. They are slow (one
//! round-trip per model) and depend on HF being reachable, so they
//! stay opt-in.

mod common;

use voxora_core::ModelSource;

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
