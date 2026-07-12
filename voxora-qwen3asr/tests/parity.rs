//! Parity / regression guard: load `Qwen/Qwen3-ASR-0.6B`, transcribe
//! the canonical `sample1.wav` ("The quick brown fox...") shipped
//! with `qwen3-asr-rs`, and assert the result contains the expected
//! substring.
//!
//! Gated by `#[ignore]` because both the audio fixture and the model
//! must be downloaded on first run. Enable with:
//!
//! ```text
//! cargo test -p voxora-qwen3asr --test parity -- --ignored --nocapture
//! ```
//!
//! ## Fixtures
//!
//! - `sample1.wav` — public domain English sample from the
//!   `alan890104/qwen3-asr-rs` upstream repo, mono 16 kHz, ~3 s,
//!   transcribed text "The quick brown fox jumps over the lazy dog."
//! - Model — `Qwen/Qwen3-ASR-0.6B` (~1.7 GB), downloaded by the test
//!   from `huggingface.co` via `voxora-hf` and cached under
//!   `$XDG_CACHE_HOME/voxora/models/huggingface/`.
//!
//! Running under `cargo test -- --ignored` exercises the full stack:
//! HF resolution → on-disk model → `QwenAsrEngine::from_hf` →
//! `AsrEngine::transcribe` → substring assertion.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use voxora_core::{AsrEngine, ResolveOptions, TranscribeOptions};
use voxora_hf::HuggingFaceSource;
use voxora_qwen3asr::QwenAsrEngine;

const SAMPLE_FIXTURE_URL: &str =
    "https://github.com/alan890104/qwen3-asr-rs/raw/main/tests/fixtures/audio/sample1.wav";
const SAMPLE_FIXTURE_REL: &str = "tests/fixtures/audio/sample1.wav";

const MODEL_ID: &str = "Qwen/Qwen3-ASR-0.6B";

/// Lazy-loaded, downloaded-once sample1.wav fixture path.
fn sample_wav() -> &'static Path {
    static CACHE: OnceLock<PathBuf> = OnceLock::new();
    CACHE.get_or_init(|| ensure_fixture(SAMPLE_FIXTURE_REL, SAMPLE_FIXTURE_URL))
}

fn ensure_fixture(rel: &str, url: &str) -> PathBuf {
    let local = PathBuf::from(rel);
    if local.exists() {
        return local;
    }
    if let Some(parent) = local.parent() {
        std::fs::create_dir_all(parent).expect("create fixture parent dir");
    }
    download_to(url, &local);
    local
}

fn download_to(url: &str, dest: &Path) {
    eprintln!("downloading {url} -> {}", dest.display());
    let resp = ureq::get(url)
        .call()
        .expect("network fetch for fixture failed");
    let mut body = resp.into_body();
    let mut file = std::fs::File::create(dest).expect("create dest file");
    let mut reader = body.as_reader();
    std::io::copy(&mut reader, &mut file).expect("write fixture to disk");
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires sample1.wav + Qwen/Qwen3-ASR-0.6B (~1.7 GB); run with --ignored"]
async fn quick_brown_fox_parity_substring_match() {
    let wav_path = sample_wav().to_path_buf();

    // Open the WAV and decode to mono f32 at its native rate. The
    // sample is already mono 16 kHz so no downmix or resample is
    // needed, but the path mirrors what a real consumer would do.
    let mut reader = hound::WavReader::open(&wav_path).expect("open sample1.wav");
    let spec = reader.spec();
    assert_eq!(spec.channels, 1, "sample1.wav must be mono");
    assert_eq!(
        spec.sample_rate, 16_000,
        "sample1.wav must be 16 kHz; this test does not resample"
    );
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
        .collect::<Result<_, _>>()
        .expect("decode i16 samples");

    let source = HuggingFaceSource::new().expect("HF source");
    let engine = QwenAsrEngine::from_hf(&source, MODEL_ID, &ResolveOptions::default())
        .await
        .expect("load Qwen3-ASR-0.6B");

    let caps = engine.capabilities();
    assert!(caps.multilingual, "Qwen3-ASR is multilingual");
    assert!(
        caps.languages.iter().any(|l| l == "english"),
        "english must be advertised in capabilities, got {:?}",
        caps.languages
    );

    let opts = TranscribeOptions::new(Some("english".into()), false, false);
    let result = engine
        .transcribe(&samples, &opts)
        .expect("transcribe sample1.wav");

    let lower = result.text.to_ascii_lowercase();
    assert!(
        lower.contains("quick brown fox"),
        "expected the canonical pangram in the transcript, got: {:?}",
        result.text
    );
    assert!(
        result.segments.is_empty(),
        "timestamps not supported upstream; segments should be empty"
    );
    assert_eq!(
        result.language.as_deref(),
        Some("english"),
        "forced-language result should echo back the caller request"
    );
}
