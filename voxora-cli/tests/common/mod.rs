//! Shared helpers for the voxora-cli integration tests.
//!
//! The CLI integration tests drive the compiled `voxora` binary as a
//! subprocess. We don't shell out to `cargo run` (slow, depends on
//! cargo workspace state) — instead we use the
//! `env!("CARGO_BIN_EXE_voxora")` magic constant that cargo exposes
//! for exactly this purpose.
//!
//! Wiremock tests additionally spin up a `MockServer` and pass the
//! URL back to the binary via the `--base-url` hidden flag.

// Each integration test binary is compiled separately, so any helper
// that's only used by one binary looks "dead" to the others. The
// `#[allow(dead_code)]` here keeps the workspace `cargo build` clean
// without scattering the attribute across every helper.
#![allow(dead_code)]

use std::path::PathBuf;
use std::process::{Command, Output};

/// Path to the compiled `voxora` binary, computed by cargo at
/// build time.
pub fn voxora_bin() -> &'static str {
    env!("CARGO_BIN_EXE_voxora")
}

/// Run the voxora binary with `args`, returning the captured
/// `Output`. Use [`output_success`] / [`output_status`] to assert.
pub fn run_voxora<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let argv: Vec<String> = args.into_iter().map(|s| s.as_ref().to_string()).collect();
    Command::new(voxora_bin())
        .args(&argv)
        .output()
        .expect("failed to launch voxora")
}

/// Convenience: read the entire `voxora-hf`/`voxora-cli` fixture root.
pub fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Read a fixture file at `<fixture_dir>/<sub>/<name>` as bytes.
pub fn read_fixture_bytes(sub: &str, name: &str) -> Vec<u8> {
    std::fs::read(fixture_dir().join(sub).join(name))
        .unwrap_or_else(|e| panic!("read fixture {sub}/{name}: {e}"))
}

/// Standard 1 KB synthetic safetensors substitute, like voxora-hf uses.
pub fn synthetic_safetensors(tag: &str) -> Vec<u8> {
    let mut v = vec![0u8; 1024];
    let tag_bytes = tag.as_bytes();
    let n = tag_bytes.len().min(v.len());
    v[..n].copy_from_slice(&tag_bytes[..n]);
    v
}
