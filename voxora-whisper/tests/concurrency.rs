//! `Send + Sync` end-to-end test: spawn N threads, each calling
//! [`AsrEngine::transcribe`] on a clone of the same engine behind an
//! `Arc<dyn AsrEngine>`. Mirrors `voxora-core`'s
//! `engine::tests::engine_works_across_threads` but exercises the
//! actual whisper-rs code path.
//!
//! Gated by `#[ignore]` because it requires `ggml-tiny.bin`.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use voxora_core::{AsrEngine, TranscribeOptions};
use voxora_whisper::WhisperEngine;

const TINY_MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin";

fn tiny_model() -> &'static PathBuf {
    static CACHE: OnceLock<PathBuf> = OnceLock::new();
    CACHE.get_or_init(|| {
        let dir = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
            .map(|p| p.join("voxora").join("whisper-fixtures"))
            .expect("HOME or XDG_CACHE_HOME");
        std::fs::create_dir_all(&dir).expect("create fixtures dir");
        let path = dir.join("ggml-tiny.bin");
        if !path.exists() {
            eprintln!("downloading {TINY_MODEL_URL} -> {}", path.display());
            let resp = ureq::get(TINY_MODEL_URL)
                .call()
                .expect("download ggml-tiny.bin");
            let mut body = resp.into_body();
            let mut file = std::fs::File::create(&path).expect("create model file");
            let mut reader = body.as_reader();
            std::io::copy(&mut reader, &mut file).expect("write model");
        }
        path
    })
}

#[test]
#[ignore = "requires ggml-tiny.bin (~75 MB); run with --ignored"]
fn engine_is_send_sync_and_transcribes_across_threads() {
    let engine = WhisperEngine::load(tiny_model()).expect("load ggml-tiny.bin");
    let engine: Arc<dyn AsrEngine> = Arc::new(engine);

    let opts = TranscribeOptions::default();
    // 1 second of silence at 16 kHz. Whisper will produce something
    // — perhaps an empty or near-empty transcript — but the point is
    // that the threads all complete without panic and the engine
    // proves Send + Sync.
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
