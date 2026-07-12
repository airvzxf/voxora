//! Common helpers shared by the wiremock-based integration tests.
//!
//! The fixtures under `tests/fixtures/<model>/` were captured live
//! from `huggingface.co`; see `tests/fixtures/README.md` for how to
//! refresh them. The wiremock tests in this directory replay those
//! recordings with the synthetic safetensors bytes substituted for
//! the (multi-hundred-MB) real weights, so the whole suite stays
//! under ~15 MB on disk and runs in a couple of seconds.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use voxora_core::{AsrError, ModelSource, ResolveOptions};
use voxora_hf::HuggingFaceSource;
use wiremock::MockServer;

/// Path to the fixtures directory.
pub fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Read a fixture file as bytes.
pub fn read_fixture(model: &str, name: &str) -> Vec<u8> {
    let path = fixtures_root().join(model).join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("read fixture {path:?}: {e}"))
}

/// Synthetic 1 KB payload that stands in for a safetensors shard.
/// Deterministic so equality assertions work.
pub fn synthetic_shard(label: &str) -> Vec<u8> {
    let mut v = vec![0u8; 1024];
    let stamp = format!("voxora-shard-{label}");
    let bytes = stamp.as_bytes();
    v[..bytes.len()].copy_from_slice(bytes);
    v
}

/// Synthetic single-file safetensors payload.
pub fn synthetic_safetensors(label: &str) -> Vec<u8> {
    synthetic_shard(label)
}

/// Build a [`HuggingFaceSource`] pointed at the supplied mock server.
pub async fn source_for(
    mock: &MockServer,
    token: Option<&str>,
) -> (tempfile::TempDir, HuggingFaceSource) {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut builder = HuggingFaceSource::builder()
        .base_url(mock.uri())
        .cache_dir(dir.path().to_path_buf());
    if let Some(t) = token {
        builder = builder.token(Some(t.to_string()));
    }
    let src = builder.build().expect("build source");
    (dir, src)
}

/// Convenience: run a resolve and unwrap to ModelDir.
pub async fn resolve_ok(src: &HuggingFaceSource, model_id: &str) -> voxora_core::ModelDir {
    src.resolve(model_id, &ResolveOptions::default())
        .await
        .expect("resolve should succeed")
}

/// Convenience: run a resolve and unwrap the error.
pub async fn resolve_err(src: &HuggingFaceSource, model_id: &str) -> AsrError {
    src.resolve(model_id, &ResolveOptions::default())
        .await
        .expect_err("resolve should fail")
}
