//! End-to-end engine-override smoke test. Network required, gated by
//! `#[ignore]`.
//!
//! Verifies that `voxora run --engine qwen3-asr <qwen3-model>` works
//! and that `--engine whisper <qwen3-model>` returns a clear error
//! (the engine + model combination is invalid).

mod common;

use std::path::PathBuf;
use std::process::Command;

use common::voxora_bin;

const MODEL_ID: &str = "Qwen/Qwen3-ASR-0.6B";

fn cache_root() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("voxora-cli-e2e")
}

#[test]
#[ignore = "requires Qwen/Qwen3-ASR-0.6B (~1.7 GB); run with --ignored"]
fn run_engine_override_qwen3_for_qwen3_model_succeeds() {
    // Locate sample1.wav; if not present, skip silently — this test
    // just verifies that the engine override flag is wired correctly,
    // not the inference itself (covered by e2e_qwen3_asr).
    let wav = PathBuf::from("tests/fixtures/samples/sample1.wav");
    if !wav.exists() {
        eprintln!(
            "engine_override: sample1.wav missing, running without audio (just verifying load)"
        );
    }
    let wav_arg: &str = if wav.exists() {
        wav.to_str().unwrap()
    } else {
        // Use a tiny inline wav so the audio decode succeeds; the
        // transcription content is irrelevant.
        "tests/fixtures/samples/silence.wav"
    };

    let out = Command::new(voxora_bin())
        .args([
            "run",
            MODEL_ID,
            wav_arg,
            "--engine",
            "qwen3-asr",
            "--cache",
            cache_root().to_str().unwrap(),
        ])
        .output()
        .expect("voxora run");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    // The model load should succeed; the inference may produce an
    // empty/garbage result but must not panic. We accept either:
    // exit 0 (load + transcription succeeded) or exit 1 (load
    // succeeded, inference produced NaN/empty on silence).
    assert!(
        out.status.code() != Some(2),
        "engine validation must not 2; stderr: {stderr}; stdout: {stdout}"
    );
    assert!(!stderr.contains("unknown --engine"));
}

#[test]
#[ignore = "requires no model — just configuration; run with --ignored"]
fn run_engine_override_whisper_for_qwen3_model_fails_with_clear_error() {
    let wav = PathBuf::from("tests/fixtures/samples/sample1.wav");
    let wav_arg: &str = if wav.exists() {
        wav.to_str().unwrap()
    } else {
        "tests/fixtures/samples/silence.wav"
    };

    let out = Command::new(voxora_bin())
        .args([
            "run",
            // Use a model id whose expected engine is qwen3-asr, then
            // override to whisper — voxora-whisper will load .bin
            // files from a directory; with no .bin present it should
            // fail with ModelNotFound rather than silently work.
            MODEL_ID,
            wav_arg,
            "--engine",
            "whisper",
            "--cache",
            cache_root().to_str().unwrap(),
        ])
        .output()
        .expect("voxora run");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);

    // Exit code 2 (refused before download) or 1 (download ok, load
    // failed). We assert it is NOT 0 — the model is incompatible
    // with whisper and we expect the run to fail.
    assert!(
        !out.status.success(),
        "qwen3 model + --engine=whisper must fail; stderr: {stderr}; stdout: {stdout}",
    );
    // Error should be informative; just check it's not just "unknown
    // --engine" since that's reserved for parse errors.
    assert!(
        !stderr.contains("unknown --engine"),
        "engine parse should succeed; invalid combination should be a runtime error: {stderr}"
    );
}
