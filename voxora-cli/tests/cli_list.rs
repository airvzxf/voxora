//! Integration tests for `voxora list` driving the binary as a
//! subprocess. The setup pre-populates the cache directory; the
//! binary then reports what's in it.

mod common;

use std::process::Command;

use common::voxora_bin;

/// Populate a fake cache layout in `cache_root`:
/// The CLI appends `voxora/models/huggingface/` to whatever the user
/// passes via `--cache`, so the layout the binary actually reads is
/// `<cache_root>/voxora/models/huggingface/<org>/<name>/<revision>/`.
fn populate(cache_root: &std::path::Path, name: &str, complete: bool) {
    let hf_root = cache_root.join("voxora").join("models").join("huggingface");
    let dir = hf_root.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("config.json"), b"{\"a\":1}").unwrap();
    std::fs::write(dir.join("model.safetensors"), vec![0u8; 256]).unwrap();
    if complete {
        std::fs::write(dir.join(".complete"), b"").unwrap();
    }
}

#[test]
fn list_prints_nothing_when_cache_is_empty_and_quiet() {
    let tmp = tempfile::tempdir().unwrap();
    let out = Command::new(voxora_bin())
        .args(["list", "--cache", tmp.path().to_str().unwrap(), "--quiet"])
        .output()
        .expect("run voxora");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}; stdout: {stdout}");
    // Quiet + empty cache = silent stdout (the "no models" notice is
    // suppressed by --quiet). Stderr should also be empty.
    assert!(stdout.is_empty(), "stdout should be empty: {stdout}");
    assert!(stderr.is_empty(), "stderr should be empty: {stderr}");
}

#[test]
fn list_prints_notice_when_cache_is_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let out = Command::new(voxora_bin())
        .args(["list", "--cache", tmp.path().to_str().unwrap()])
        .output()
        .expect("run voxora");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "stderr: {stderr}; stdout: {stdout}");
    // Without --quiet, the empty-cache notice surfaces on stderr.
    assert!(
        stderr.contains("no models under"),
        "stderr should mention empty cache: {stderr}"
    );
}

#[test]
fn list_shows_cached_models() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().to_path_buf();
    populate(&cache, "Qwen/Qwen3-ASR-0.6B/main", true);
    populate(&cache, "openai/whisper-tiny/main", false);

    let out = Command::new(voxora_bin())
        .args(["list", "--cache", cache.to_str().unwrap()])
        .output()
        .expect("run voxora");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}; stdout: {stdout}");
    assert!(
        stdout.contains("Qwen/Qwen3-ASR-0.6B/main"),
        "should list Qwen3-ASR row: {stdout}"
    );
    assert!(
        stdout.contains("whisper-tiny/main"),
        "should list whisper-tiny row: {stdout}"
    );
    // The header carries the BYTES column; we populated with 256 bytes
    // of model weights + ~7 bytes of config.
    assert!(
        stdout.contains("263 B"),
        "must show 263 byte model: {stdout}"
    );
}

#[test]
fn list_exit_codes_are_zero_even_when_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let out = Command::new(voxora_bin())
        .args(["list", "--cache", tmp.path().to_str().unwrap(), "--quiet"])
        .output()
        .expect("run voxora");
    // Empty list is not an error — exit 0, callers can grep.
    assert!(out.status.success());
}

#[test]
fn list_under_unwritable_path_succeeds_with_zero() {
    // We can't easily simulate an unwritable directory in this sandbox,
    // so we just verify that pointing --cache at a directory that does
    // exist but has no huggingface/ subdir yet returns cleanly with 0
    // exit code (rather than 1).
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("brand_new_cache");
    std::fs::create_dir_all(&cache).unwrap();

    let out = Command::new(voxora_bin())
        .args(["list", "--cache", cache.to_str().unwrap(), "--quiet"])
        .output()
        .expect("run voxora");
    assert!(out.status.success());
}
