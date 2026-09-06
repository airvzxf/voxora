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
//! ## Why `#[ignore]`-d
//!
//! The test requires real model weights:
//!
//! - `ggml-tiny.bin` (~75 MB) — `voxora-testkit`'s
//!   `resolve_real_fixture("ggml-tiny.bin")`.
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
//! `voxora_testkit::wer` splits on whitespace and compares
//! tokens verbatim. Without normalisation, semantically
//! identical transcripts ("Hello, world!" vs "hello world"
//! vs "Hello World") produce a non-zero WER. The
//! [`normalize`] helper below applies the four canonical
//! transforms so the comparison reflects semantic agreement.

#![cfg(feature = "parity")]

use std::path::Path;

use voxora_bridge::{AsrEngine, HuggingFaceSource, QwenAsrEngine, TranscribeOptions, WhisperEngine};
use voxora_testkit::{resolve_real_fixture, wer as wer_score};

const QWEN3_MODEL_ID: &str = "Qwen/Qwen3-ASR-0.6B";
const AUDIO_FIXTURE: &str = "sample1.wav";
const WHISPER_MODEL_FIXTURE: &str = "ggml-tiny.bin";
const WER_THRESHOLD: f64 = 0.3;

/// Canonical transcript normalisation for cross-engine WER.
///
/// Applies, in order:
///
/// 1. Lower-case (ASCII).
/// 2. Strip ASCII punctuation; collapse curly / smart quotes
///    to ASCII apostrophes so `it's` matches `it's`.
/// 3. Collapse runs of whitespace to a single space and trim
///    leading / trailing whitespace.
///
/// Returns the empty string for inputs that reduce to nothing
/// after stripping (e.g. `"…!"`).
fn normalize(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    // First pass: map smart-quote variants to ASCII apostrophes
    // so "don't" and "don't" tokenise identically. We do this
    // before stripping punctuation because some punctuation
    // chars carry semantic information in the apostrophe
    // position (e.g. "can't" vs "cant").
    let smart_quoted = lower
        .replace('\u{2018}', "'")
        .replace('\u{2019}', "'")
        .replace('\u{201C}', "\"")
        .replace('\u{201D}', "\"");
    // Second pass: keep ASCII apostrophes; drop everything else
    // that is not an ASCII alphanumeric or whitespace.
    let mut out = String::with_capacity(smart_quoted.len());
    for ch in smart_quoted.chars() {
        let keep = ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace() || ch == '\'';
        if keep {
            out.push(ch);
        } else {
            out.push(' ');
        }
    }
    // Third pass: collapse runs of whitespace.
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

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

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn normalize_lowercases_ascii() {
        assert_eq!(normalize("HELLO World"), "hello world");
    }

    #[test]
    fn normalize_strips_punctuation() {
        assert_eq!(normalize("Hello, world!"), "hello world");
    }

    #[test]
    fn normalize_keeps_apostrophes() {
        assert_eq!(normalize("don't stop"), "don't stop");
    }

    #[test]
    fn normalize_smart_quotes_to_ascii() {
        // Smart single quotes (U+2018, U+2019) carry semantic
        // information (apostrophe / opening quote) so they are
        // kept as ASCII apostrophes. Smart double quotes (U+201C,
        // U+201D) are stripped along with the rest of the ASCII
        // punctuation so `Hello "world"` matches `hello world`
        // token-for-token after normalisation.
        assert_eq!(
            normalize("\u{2018}hello\u{2019} world\u{201D}"),
            "'hello' world"
        );
    }

    #[test]
    fn normalize_collapses_whitespace() {
        assert_eq!(normalize("  hello\t\n world  "), "hello world");
    }

    #[test]
    fn normalize_pure_punctuation_yields_empty() {
        assert_eq!(normalize("…!?"), "");
    }
}
