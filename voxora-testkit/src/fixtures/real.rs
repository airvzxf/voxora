//! Real audio + model fixtures used by parity tests.
//!
//! These fixtures download large model checkpoints (~75 MB for
//! Whisper's `ggml-tiny.bin`, ~1.7 GB for `Qwen/Qwen3-ASR-0.6B`)
//! from public mirrors. Tests that exercise them must be
//! `#[ignore]`-gated so they do not run on every `cargo test`
//! invocation.
//!
//! Downloads happen once per machine (cached under the voxora cache
//! directory) and reuse the result on subsequent runs. The HTTP
//! fetch is sync (`ureq`); the API is `fn -> Result`, not `async
//! fn`, because every caller today is itself a `#[ignore]`-d
//! integration test that already drives a tokio runtime.
//!
//! ## Status (EPIC #133, PR #59)
//!
//! The download helper used to be a stub that returned
//! `FixtureError::Network { message: "download not yet implemented"
//! }`. PR #59 fills it in with a sync `ureq::get(url).call()` +
//! `std::io::copy` and adds the cache-hit short-circuit (the
//! pre-existing `is_file()` check was already correct; it just
//! never had a real download behind it).
//!
//! ## Scope (deliberate)
//!
//! Only **audio** fixtures (`jfk.wav`, `sample1.wav`) and the
//! Whisper `ggml-tiny.bin` checkpoint live here. The
//! `Qwen/Qwen3-ASR-0.6B` model (1.7 GB) is resolved via the real
//! `voxora-hf` HF client in each engine's `tests/parity.rs` —
//! testkit deliberately does not host it because the HF client's
//! sha256 sidecar verification lives in `voxora-hf`, not here, and
//! re-implementing it inside the testkit would silently fork the
//! download path. PR #59 also removes the broken
//! `qwen3-asr-0.6b` entry that pointed at `preprocessor_config.json`
//! (a 1 KB config blob, not the model) — a placeholder from a
//! pre-Fase 4 draft that never made it onto crates.io.

use std::path::PathBuf;

/// Known fixture names accepted by [`resolve_real_fixture`].
pub const KNOWN_FIXTURES: &[&str] = &["jfk.wav", "sample1.wav", "ggml-tiny.bin"];

/// Resolve a fixture file by canonical name.
///
/// `name` selects from [`KNOWN_FIXTURES`]. Unknown names return
/// [`FixtureError::UnknownFixture`]. Tests that call this must be
/// `#[ignore]`-gated — see the module docs.
pub fn resolve_real_fixture(name: &str) -> Result<PathBuf, FixtureError> {
    match name {
        "jfk.wav" => download("jfk.wav", JFK_URL),
        "sample1.wav" => download("sample1.wav", SAMPLE1_URL),
        "ggml-tiny.bin" => download("ggml-tiny.bin", GGML_TINY_URL),
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

fn download(name: &str, url: &str) -> Result<PathBuf, FixtureError> {
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
    let resp = ureq::get(url).call().map_err(|e| FixtureError::Network {
        fixture: name.to_string(),
        message: format!("GET {url}: {e}"),
        source: Some(Box::new(e)),
    })?;
    let mut body = resp.into_body();
    let mut reader = body.as_reader();
    let mut file = std::fs::File::create(&dest).map_err(|e| FixtureError::Network {
        fixture: name.to_string(),
        message: format!("create {}: {e}", dest.display()),
        source: Some(Box::new(e)),
    })?;
    std::io::copy(&mut reader, &mut file).map_err(|e| FixtureError::Network {
        fixture: name.to_string(),
        message: format!("write {}: {e}", dest.display()),
        source: Some(Box::new(e)),
    })?;
    Ok(dest)
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
        let err = resolve_real_fixture("does-not-exist").expect_err("unknown fixture must error");
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

    #[test]
    fn broken_qwen3_asr_entry_is_removed() {
        // Closes the EPIC #133 cleanup: pre-Fase-4 drafts of this
        // module referenced a `qwen3-asr-0.6b` fixture pointing
        // at `preprocessor_config.json` — a 1 KB config blob, not
        // the 1.7 GB model. The entry is gone from KNOWN_FIXTURES
        // and `resolve_real_fixture`; this test pins the
        // regression so a future re-add has to acknowledge it.
        assert!(
            !KNOWN_FIXTURES.contains(&"qwen3-asr-0.6b"),
            "KNOWN_FIXTURES must not contain the broken qwen3-asr-0.6b entry"
        );
        let err = resolve_real_fixture("qwen3-asr-0.6b").expect_err("removed fixture must error");
        assert!(
            matches!(err, FixtureError::UnknownFixture { .. }),
            "removed fixture must return UnknownFixture, got {err:?}"
        );
    }
}
