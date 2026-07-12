//! End-to-end `voxora run` against a real Qwen3-ASR model. Network
//! required, gated by `#[ignore]`.
//!
//! Run with:
//!
//! ```text
//! cargo test -p voxora-cli --test e2e_qwen3_asr -- --ignored --nocapture
//! ```

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use common::voxora_bin;

const MODEL_ID: &str = "Qwen/Qwen3-ASR-0.6B";
const SAMPLE_URL: &str =
    "https://github.com/alan890104/qwen3-asr-rs/raw/main/tests/fixtures/audio/sample1.wav";
const SAMPLE_REL: &str = "tests/fixtures/samples/sample1.wav";

fn sample_wav() -> &'static Path {
    static CACHE: OnceLock<PathBuf> = OnceLock::new();
    CACHE.get_or_init(|| ensure_sample(SAMPLE_REL, SAMPLE_URL))
}

fn ensure_sample(rel: &str, url: &str) -> PathBuf {
    let local = PathBuf::from(rel);
    if local.exists() {
        return local;
    }
    if let Some(parent) = local.parent() {
        std::fs::create_dir_all(parent).expect("create fixture parent");
    }
    download(url, &local);
    local
}

fn download(url: &str, dest: &Path) {
    eprintln!("e2e_qwen3_asr: downloading {url} -> {}", dest.display());
    let resp = ureq::get(url)
        .call()
        .expect("network fetch for fixture failed");
    let mut body = resp.into_body();
    let mut file = std::fs::File::create(dest).expect("create fixture");
    let mut reader = body.as_reader();
    std::io::copy(&mut reader, &mut file).expect("write fixture");
}

#[test]
#[ignore = "requires Qwen/Qwen3-ASR-0.6B (~1.7 GB) + sample1.wav; run with --ignored"]
fn run_qwen3_asr_transcribes_sample1() {
    let wav = sample_wav().to_str().expect("sample path");

    let cache_root = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("voxora-cli-e2e");

    let out = Command::new(voxora_bin())
        .args([
            "run",
            MODEL_ID,
            wav,
            "--cache",
            cache_root.to_str().unwrap(),
        ])
        .output()
        .expect("voxora run");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "stderr: {stderr}; stdout: {stdout}");
    let lower = stdout.to_ascii_lowercase();
    assert!(
        lower.contains("quick brown fox"),
        "expected the canonical pangram; got: {stdout}"
    );
}
