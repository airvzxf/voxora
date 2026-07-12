//! Parity / regression guard: load `ggml-tiny.bin`, transcribe the
//! canonical `jfk.wav` sample shipped with `whisper.cpp`, and assert
//! the result contains the expected substring.
//!
//! Gated by `#[ignore]` because both the audio fixture and the model
//! must be downloaded on first run. Enable with:
//!
//! ```text
//! cargo test -p voxora-whisper --test parity -- --ignored --nocapture
//! ```
//!
//! ## Fixtures
//!
//! - `tests/fixtures/jfk.wav` — public domain JFK audio from
//!   `github.com/ggerganov/whisper.cpp` (raw, ~30s, mono, 16 kHz).
//! - Model — `ggml-tiny.bin` (75 MB), downloaded by the test from
//!   `huggingface.co/ggerganov/whisper.cpp` if not already cached
//!   under `$XDG_CACHE_HOME/voxora/whisper-fixtures/`.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use voxora_core::{AsrEngine, TranscribeOptions};
use voxora_whisper::WhisperEngine;

const JFK_FIXTURE_URL: &str = "https://github.com/ggerganov/whisper.cpp/raw/master/samples/jfk.wav";
const JFK_FIXTURE_REL: &str = "tests/fixtures/jfk.wav";
const JFK_MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin";

const JFK_MODEL_FILENAME: &str = "ggml-tiny.bin";

/// Lazy-loaded, downloaded-once jfk.wav fixture path.
fn jfk_wav() -> &'static Path {
    static CACHE: OnceLock<PathBuf> = OnceLock::new();
    CACHE.get_or_init(|| ensure_fixture(JFK_FIXTURE_REL, JFK_FIXTURE_URL))
}

/// Lazy-loaded, downloaded-once ggml-tiny.bin model path.
fn tiny_model() -> &'static Path {
    static CACHE: OnceLock<PathBuf> = OnceLock::new();
    CACHE.get_or_init(|| {
        let dir = fixtures_dir();
        std::fs::create_dir_all(&dir).expect("create fixtures dir");
        let path = dir.join(JFK_MODEL_FILENAME);
        if !path.exists() {
            download_to(JFK_MODEL_URL, &path);
        }
        path
    })
}

fn fixtures_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .expect("HOME or XDG_CACHE_HOME must be set");
    base.join("voxora").join("whisper-fixtures")
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
        .expect("network fetch for fixture/model failed");
    let mut body = resp.into_body();
    let mut file = std::fs::File::create(dest).expect("create dest file");
    let mut reader = body.as_reader();
    std::io::copy(&mut reader, &mut file).expect("write fixture/model to disk");
}

#[test]
#[ignore = "requires jfk.wav + ggml-tiny.bin (~75 MB); run with --ignored"]
fn jfk_parity_substring_match() {
    let wav_path = jfk_wav().to_path_buf();
    let model_path = tiny_model().to_path_buf();

    let engine = WhisperEngine::load(&model_path).expect("load ggml-tiny.bin");
    assert!(
        engine.capabilities().multilingual,
        "ggml-tiny is multilingual; capabilities should report so"
    );

    let mut reader = hound::WavReader::open(&wav_path).expect("open jfk.wav");
    let spec = reader.spec();
    assert_eq!(spec.channels, 1, "jfk.wav must be mono");
    assert_eq!(
        spec.sample_rate, 16_000,
        "jfk.wav must be 16 kHz; this test does not resample"
    );
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
        .collect::<Result<_, _>>()
        .expect("decode i16 samples");

    let opts = TranscribeOptions::new(Some("en".into()), false, true);
    let result = engine.transcribe(&samples, &opts).expect("transcribe");

    let lower = result.text.to_ascii_lowercase();
    // The canonical JFK quote is "ask not what your country can do for you,
    // ask what you can do for your country." We check for the most
    // distinguishing phrase; whisper's greedy decoder is deterministic for
    // this well-known audio so the substring should be present.
    assert!(
        lower.contains("ask not what your country can do for you"),
        "expected JFK quote in transcript, got: {:?}",
        result.text
    );
    assert!(
        !result.segments.is_empty(),
        "timestamps were requested; segments must be non-empty"
    );
    // Segments should be in monotonic, non-overlapping order.
    for w in result.segments.windows(2) {
        assert!(
            w[0].end_sample <= w[1].start_sample,
            "segments overlap or go backwards: {:?}",
            result.segments
        );
    }
}

#[test]
#[ignore = "requires ggml-tiny.en.bin (~75 MB); run with --ignored"]
fn english_only_model_reports_no_multilingual() {
    let dir = fixtures_dir();
    std::fs::create_dir_all(&dir).expect("create fixtures dir");
    let path = dir.join("ggml-tiny.en.bin");
    if !path.exists() {
        download_to(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
            &path,
        );
    }
    let engine = WhisperEngine::load(&path).expect("load ggml-tiny.en.bin");
    let caps = engine.capabilities();
    assert!(!caps.multilingual, "ggml-tiny.en is English-only");
    assert!(
        caps.languages.iter().any(|l| l == "en"),
        "english-only model should advertise only [en]"
    );
}
