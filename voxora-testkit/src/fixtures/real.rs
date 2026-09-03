//! Real audio + model fixtures used by parity tests.
//!
//! These fixtures download large model checkpoints (~75 MB) from
//! Hugging Face. Tests that exercise them must be `#[ignore]`-gated
//! so they do not run on every `cargo test` invocation.
//!
//! Downloads happen once per machine (cached under the voxora cache
//! directory) and reuse the result on subsequent runs.
//!
//! ## Status (Fase 3 PR C)
//!
//! This module currently defines the canonical surface only.
//! [`resolve_real_fixture`] dispatches on a known set of fixture
//! names and returns a [`FixtureError`] for unknown inputs, but the
//! actual network download logic is still owned by each engine's
//! `tests/parity.rs`. A follow-up patch will replace those
//! per-engine helpers with calls into this module.
//!
//! Engine parity tests today:
//!
//! - `voxora-whisper/tests/parity.rs` — `jfk.wav`, `ggml-tiny.bin`.
//! - `voxora-qwen3asr/tests/parity.rs` — `sample1.wav`.
//!
//! Future engines (parakeet, voxtral, …) should reach for
//! [`resolve_real_fixture`] directly so the download cache and URL
//! list live in exactly one place.

use std::path::PathBuf;

/// Known fixture names accepted by [`resolve_real_fixture`].
pub const KNOWN_FIXTURES: &[&str] = &["jfk.wav", "sample1.wav", "ggml-tiny.bin", "qwen3-asr-0.6b"];

/// Resolve a fixture file by canonical name.
///
/// `name` selects from [`KNOWN_FIXTURES`]. Unknown names return
/// [`FixtureError::UnknownFixture`]. Tests that call this must be
/// `#[ignore]`-gated — see the module docs.
pub async fn resolve_real_fixture(name: &str) -> Result<PathBuf, FixtureError> {
    match name {
        "jfk.wav" => download("jfk.wav", JFK_URL).await,
        "sample1.wav" => download("sample1.wav", SAMPLE1_URL).await,
        "ggml-tiny.bin" => download("ggml-tiny.bin", GGML_TINY_URL).await,
        "qwen3-asr-0.6b" => download("qwen3-asr-0.6b", QWEN3_ASR_URL).await,
        other => Err(FixtureError::UnknownFixture {
            name: other.to_string(),
            known: KNOWN_FIXTURES.join(", "),
        }),
    }
}

/// Errors returned by [`resolve_real_fixture`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FixtureError {
    /// `name` was not in [`KNOWN_FIXTURES`].
    #[error("unknown fixture name: {name}; expected one of {known}")]
    UnknownFixture {
        /// The fixture name that was requested.
        name: String,
        /// Comma-separated list of known fixture names for the
        /// error message.
        known: String,
    },

    /// Neither `VOXORA_FIXTURE_CACHE_DIR` nor `dirs::cache_dir()`
    /// resolved to a writable location.
    #[error("fixture cache directory not configured")]
    CacheNotConfigured,

    /// Network or filesystem I/O failure while downloading a
    /// fixture.
    #[error("fixture error for {fixture}: {message}")]
    Network {
        /// The fixture name that triggered the failure.
        fixture: String,
        /// Human-readable description of the failure.
        message: String,
        /// Underlying error (I/O, HTTP, etc.) if available.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

const JFK_URL: &str = "https://github.com/ggerganov/whisper.cpp/raw/master/samples/jfk.wav";
const SAMPLE1_URL: &str =
    "https://github.com/alan890104/qwen3-asr-rs/raw/main/tests/fixtures/audio/sample1.wav";
const GGML_TINY_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin";
const QWEN3_ASR_URL: &str =
    "https://huggingface.co/Qwen/Qwen3-ASR-0.6B/resolve/main/preprocessor_config.json";

async fn download(name: &str, url: &str) -> Result<PathBuf, FixtureError> {
    let cache_root = cache_root().map_err(|message| FixtureError::Network {
        fixture: name.to_string(),
        message,
        source: None,
    })?;
    std::fs::create_dir_all(&cache_root).map_err(|e| FixtureError::Network {
        fixture: name.to_string(),
        message: format!("create cache dir {}: {e}", cache_root.display()),
        source: Some(Box::new(e)),
    })?;
    let dest = cache_root.join(name);
    if dest.is_file() {
        return Ok(dest);
    }
    Err(FixtureError::Network {
        fixture: name.to_string(),
        message: format!(
            "download from {url} not yet implemented in voxora-testkit; \
             the engine parity test still owns this URL today"
        ),
        source: None,
    })
}

fn cache_root() -> Result<PathBuf, String> {
    if let Some(dir) = std::env::var_os("VOXORA_FIXTURE_CACHE_DIR") {
        return Ok(PathBuf::from(dir));
    }
    dirs::cache_dir()
        .map(|p| p.join("voxora").join("fixtures"))
        .ok_or_else(|| {
            "VOXORA_FIXTURE_CACHE_DIR is unset and dirs::cache_dir() returned None".to_string()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_fixture_errors() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let err = rt
            .block_on(resolve_real_fixture("does-not-exist"))
            .expect_err("unknown fixture must error");
        match err {
            FixtureError::UnknownFixture { name, .. } => assert_eq!(name, "does-not-exist"),
            other => panic!("expected UnknownFixture, got {other:?}"),
        }
    }

    #[test]
    fn unknown_fixture_message_lists_known_names() {
        let err = FixtureError::UnknownFixture {
            name: "nope".to_string(),
            known: KNOWN_FIXTURES.join(", "),
        };
        let msg = err.to_string();
        for fixture in KNOWN_FIXTURES {
            assert!(
                msg.contains(fixture),
                "error {msg:?} must mention {fixture}"
            );
        }
    }
}
