//! Subprocess-level help and version tests for `voxora`.
//!
//! These never hit the network — they only verify clap's parser is
//! wired correctly.

mod common;

use common::run_voxora;

#[test]
fn voxora_help_exits_zero() {
    let out = run_voxora(["--help"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("voxora"), "stdout: {stdout}");
    assert!(
        stdout.contains("list")
            && stdout.contains("info")
            && stdout.contains("download")
            && stdout.contains("run"),
        "every subcommand must be listed: {stdout}",
    );
}

#[test]
fn voxora_version_exits_zero() {
    let out = run_voxora(["--version"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Just check the version string looks like a Cargo version.
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "version mismatch: {stdout}"
    );
}

#[test]
fn voxora_run_help_exits_zero() {
    let out = run_voxora(["run", "--help"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("--engine"),
        "must document --engine: {stdout}"
    );
    assert!(
        stdout.contains("--language"),
        "must document --language: {stdout}"
    );
    assert!(
        stdout.contains("--timestamps"),
        "must document --timestamps: {stdout}"
    );
}

#[test]
fn voxora_run_without_args_exits_two() {
    // Missing the required <MODEL_ID> and <AUDIO>.
    let out = run_voxora(["run"]);
    assert!(!out.status.success());
    assert_eq!(
        out.status.code(),
        Some(2),
        "clap usage errors must exit 2; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn voxora_run_with_unknown_engine_exits_two() {
    let out = run_voxora(["run", "--engine", "parakeet", "model/id", "audio.wav"]);
    assert!(!out.status.success());
    assert_eq!(
        out.status.code(),
        Some(2),
        "engine-label error must exit 2; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown --engine") || stderr.contains("parakeet"),
        "stderr should mention the bad value: {stderr}"
    );
}

#[test]
fn voxora_unknown_subcommand_exits_two() {
    let out = run_voxora(["parakeet"]);
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn voxora_download_bad_quantization_exits_two() {
    let out = run_voxora([
        "download",
        "Qwen/Qwen3-ASR-0.6B",
        "--quantization",
        "int4-z",
    ]);
    assert!(!out.status.success());
    assert_eq!(
        out.status.code(),
        Some(2),
        "bad --quantization must exit 2; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("int4-z"),
        "stderr should mention the bad value: {stderr}"
    );
}

#[test]
fn voxora_serve_returns_not_implemented() {
    let out = run_voxora(["serve"]);
    assert!(!out.status.success());
    assert_eq!(
        out.status.code(),
        Some(2),
        "`voxora serve` not yet implemented must exit 2; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("not implemented"));
}
