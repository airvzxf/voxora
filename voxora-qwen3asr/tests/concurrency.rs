//! `Send + Sync` end-to-end test: spawn N threads, each calling
//! [`AsrEngine::transcribe`] on a clone of the same engine behind an
//! `Arc<dyn AsrEngine>`. Mirrors `voxora-core`'s
//! `engine::tests::engine_works_across_threads` and `voxora-whisper`'s
//! `tests/concurrency.rs` but exercises the qwen3-asr code path.
//!
//! Gated by `#[ignore]` because it requires `Qwen/Qwen3-ASR-0.6B`
//! to be downloaded once via the parity test.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use voxora_core::{AsrEngine, ModelSource, ResolveOptions, TranscribeOptions};
use voxora_hf::HuggingFaceSource;
use voxora_qwen3asr::QwenAsrEngine;

const MODEL_ID: &str = "Qwen/Qwen3-ASR-0.6B";

/// Ensure the model is resolved once, then return the directory
/// path. We keep this lazy because `HuggingFaceSource::resolve` does
/// the actual HF download and we want the concurrency test to be a
/// pure `Arc<dyn AsrEngine>` test, not a network test.
fn qwen_dir() -> &'static PathBuf {
    static CACHE: OnceLock<PathBuf> = OnceLock::new();
    CACHE.get_or_init(|| {
        let source = tokio::runtime::Runtime::new().expect("rt").block_on(async {
            HuggingFaceSource::new()
                .expect("HF source")
                .resolve(MODEL_ID, &ResolveOptions::default())
                .await
                .expect("resolve Qwen3-ASR-0.6B")
        });
        source.path
    })
}

#[test]
#[ignore = "requires Qwen/Qwen3-ASR-0.6B (~1.7 GB); run with --ignored"]
fn engine_is_send_sync_and_transcribes_across_threads() {
    let engine = QwenAsrEngine::load(qwen_dir()).expect("load Qwen3-ASR-0.6B");
    let engine: Arc<dyn AsrEngine> = Arc::new(engine);

    let opts = TranscribeOptions::default();
    // 1 second of silence at 16 kHz. Qwen3-ASR will produce some
    // text — perhaps empty or near-empty — but the point is that the
    // threads all complete without panic and the engine proves
    // Send + Sync via the `Arc<dyn AsrEngine>` round-trip.
    let samples = vec![0.0_f32; 16_000];

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let e = Arc::clone(&engine);
            let s = samples.clone();
            let o = opts.clone();
            std::thread::spawn(move || e.transcribe(&s, &o).map(|_r| ()))
        })
        .collect();

    for h in handles {
        h.join()
            .expect("thread did not panic")
            .expect("transcribe ok");
    }
}
