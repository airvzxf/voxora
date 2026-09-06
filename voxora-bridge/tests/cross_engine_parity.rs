//! Cross-engine parity matrix (EPIC #133, closes #59).
//!
//! Loads the same audio fixture (`sample1.wav`) into
//! [`voxora_bridge::WhisperEngine`] and
//! [`voxora_bridge::QwenAsrEngine`], transcribes it through both,
//! normalises the transcripts, and asserts the Word Error Rate
//! (WER) between them is below `0.3`.
//!
//! ## Why this lives in `voxora-bridge` and not `voxora-testkit`
//!
//! `voxora-testkit` is **offline-only** by manifest
//! (`voxora-testkit/Cargo.toml:18` `publish = false`, `lib
//! bench = false`, no network, no engine deps). Pulling both
//! `voxora-whisper` and `voxora-qwen3asr` in as dev-deps would
//! flip testkit into a hybrid online/offline crate and break
//! every consumer that uses it as a hermetic dependency.
//!
//! `voxora-bridge` already re-exports both engines behind
//! feature flags (closes #49 + EPIC #117), so the parity test
//! becomes a published contract: a downstream consumer that
//! enables `voxora-bridge/parity` gets the cross-engine
//! guarantee for free.
//!
//! ## Why the WER + fixture helpers are inlined here (closes #140)
//!
//! Before #140, this test imported `voxora_testkit::{wer,
//! resolve_real_fixture}` from the dev-only testkit crate. As of
//! 0.5.2, `voxora-bridge` cannot declare `voxora-testkit` as a
//! `[dev-dependencies]` entry because testkit is `publish =
//! false` and cargo's package step refuses to resolve it from
//! the crates.io index — the previous setup blocked
//! `cargo publish` for `voxora-bridge`. The two helpers are now
//! inlined in this test crate (see `wer.rs` and `fixtures.rs`);
//! the surface is intentionally identical to testkit's so other
//! voxora-* parity tests that consume testkit directly keep
//! the same scoring math.
//!
//! ## Why `#[ignore]`-d
//!
//! The test requires real model weights:
//!
//! - `ggml-tiny.bin` (~75 MB) — the inlined
//!   `fixtures::resolve_real_fixture("ggml-tiny.bin")`.
//! - `Qwen/Qwen3-ASR-0.6B` (~1.7 GB, **not** the 600 MB figure
//!   from the original issue body — that was an early estimate
//!   for the un-sharded checkpoint; the actual safetensors are
//!   ~1.7 GB). Resolved through `voxora_hf::HuggingFaceSource`
//!   so the sha256 sidecar verification stays in the production
//!   download path.
//!
//! Both downloads are too heavy for PR CI; the test is gated
//! behind `#[ignore]` so the offline lanes stay green. Run
//! manually with:
//!
//! ```text
//! cargo test -p voxora-bridge --test cross_engine_parity \
//!     --features parity -- --ignored --nocapture
//! ```
//!
//! ## Threshold rationale
//!
//! `WER < 0.3` is the acceptance threshold from the issue body.
//! It is intentionally loose: Whisper's greedy decoder and
//! Qwen3-ASR's beam-search-lite will diverge on filler words,
//! punctuation, and the trailing silence in `sample1.wav`. The
//! test is a regression tripwire on gross engine disagreement,
//! not a strict-text-equality gate.
//!
//! ## Transcript normalisation
//!
//! [`normalize`] lives in [`voxora_bridge::normalize`] and is
//! pure-string; its unit tests run in the default offline lane
//! (`cargo test --workspace`). The integration test target here
//! is gated behind the `parity` Cargo feature (see
//! `Cargo.toml` → `[[test]].required-features`) so the heavy
//! download / load path only runs when explicitly requested.

mod fixtures;
mod wer;

use std::path::Path;

use voxora_bridge::normalize::normalize;
use voxora_bridge::{AsrEngine, HuggingFaceSource, QwenAsrEngine, TranscribeOptions, WhisperEngine};

use fixtures::resolve_real_fixture;
use wer::wer as wer_score;

const QWEN3_MODEL_ID: &str = "Qwen/Qwen3-ASR-0.6B";
const AUDIO_FIXTURE: &str = "sample1.wav";
const WHISPER_MODEL_FIXTURE: &str = "ggml-tiny.bin";
const WER_THRESHOLD: f64 = 0.3;

#[test]
#[ignore = "requires sample1.wav + ggml-tiny.bin (~75 MB) + Qwen/Qwen3-ASR-0.6B (~1.7 GB); run with --ignored"]
fn cross_engine_wer_below_threshold_on_sample1() {
    // 1. Resolve the audio fixture through the canonical
    //    `voxora-testkit` helper. This pulls ~40 KB from the
    //    `qwen3-asr-rs` upstream sample set on first run;
    //    subsequent runs hit the cache.
    let wav_path = resolve_real_fixture(AUDIO_FIXTURE).expect("resolve sample1.wav");
    let samples = decode_wav_mono_16k(&wav_path).expect("decode sample1.wav to mono f32 @ 16 kHz");

    // 2. Whisper. The testkit fixture for the model is the
    //    same ggml-tiny.bin `voxora-whisper/tests/parity.rs`
    //    uses; reusing it keeps the cache hot across the
    //    workspace.
    let whisper_model_path =
        resolve_real_fixture(WHISPER_MODEL_FIXTURE).expect("resolve ggml-tiny.bin");
    let whisper = WhisperEngine::load(&whisper_model_path).expect("load Whisper engine");
    let whisper_text = whisper
        .transcribe(
            &samples,
            &TranscribeOptions::new(Some("en".into()), false, false),
        )
        .expect("Whisper transcribe")
        .text;

    // 3. Qwen3-ASR. Resolves through the real `voxora-hf`
    //    client so the sha256 sidecar check stays in the
    //    production download path.
    let hf_source = HuggingFaceSource::new().expect("HF source");
    let qwen = futures_block_on(QwenAsrEngine::from_hf(
        &hf_source,
        QWEN3_MODEL_ID,
        &Default::default(),
    ))
    .expect("load Qwen3-ASR");
    let qwen_text = qwen
        .transcribe(
            &samples,
            &TranscribeOptions::new(Some("english".into()), false, false),
        )
        .expect("Qwen3 transcribe")
        .text;

    // 4. Normalise + score. The threshold is a regression
    //    tripwire, not a quality gate; the test fails if the
    //    two engines drift past 30 % word-level disagreement.
    let ref_norm = normalize(&whisper_text);
    let hyp_norm = normalize(&qwen_text);
    let score = wer_score(&ref_norm, &hyp_norm);
    eprintln!("whisper raw  : {whisper_text:?}");
    eprintln!("qwen3asr raw : {qwen_text:?}");
    eprintln!("whisper norm : {ref_norm:?}");
    eprintln!("qwen3asr norm: {hyp_norm:?}");
    eprintln!("wer = {score:.4}");
    assert!(
        score < WER_THRESHOLD,
        "cross-engine WER {score:.4} exceeds threshold {WER_THRESHOLD}; \
         whisper={ref_norm:?} qwen={hyp_norm:?}"
    );
}

/// Tiny tokio runtime used because the bridge test dev-dep
/// already pulls `tokio` for the bridge demo example, but
/// `#[tokio::test]` would force `flavor = "current_thread"` and
/// qwen3-asr model load is slow enough that we'd rather block on
/// a dedicated runtime than share one with the global test
/// harness.
fn futures_block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime")
        .block_on(fut)
}

/// Decode a mono 16 kHz WAV into `Vec<f32>` in [-1.0, 1.0].
///
/// The cross-engine parity check assumes both engines see
/// identical sample buffers; any resampling or channel
/// downmix would skew the WER. Mirrors the decoder the
/// per-engine parity tests already use.
fn decode_wav_mono_16k(path: &Path) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path).map_err(|e| format!("open wav: {e}"))?;
    let spec = reader.spec();
    if spec.channels != 1 {
        return Err(format!("expected mono WAV, got {} channels", spec.channels));
    }
    if spec.sample_rate != 16_000 {
        return Err(format!(
            "expected 16 kHz WAV, got {} Hz (no resampler in this test)",
            spec.sample_rate
        ));
    }
    reader
        .samples::<i16>()
        .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("decode i16 samples: {e}"))
}
