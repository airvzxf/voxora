//! Local copy of `voxora_testkit::resolve_real_fixture` for the
//! `cross_engine_parity` test.
//!
//! Inlined because `voxora-testkit` is `publish = false` and
//! `voxora-bridge` cannot declare it as a `[dev-dependencies]`
//! (see issue #140). Mirrors
//! `voxora-testkit/src/fixtures/real.rs` byte-for-byte (URL set,
//! cache root resolution, atomic `<dest>.part` download dance) so
//! the bridge test stays in lock-step with every other
//! voxora-* parity test's fixture cache layout.
//!
//! Each integration test file is its own crate root, so this file
//! is pulled into `tests/cross_engine_parity.rs` via `mod fixtures;`.

use std::path::{Path, PathBuf};

/// Known fixture names accepted by [`resolve_real_fixture`].
pub const KNOWN_FIXTURES: &[&str] = &["jfk.wav", "sample1.wav", "ggml-tiny.bin"];

/// Errors returned by [`resolve_real_fixture`].
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
// `UnknownFixture` + `CacheNotConfigured` mirror testkit's surface; only `Network` is exercised here.
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

/// Resolve a fixture file by canonical name.
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
    // Stream into a sibling `<dest>.part` first, then rename
    // onto `<dest>` only after `io::copy` returns `Ok`. The
    // pre-existing `is_file()` cache-hit short-circuit at the
    // top of `download` therefore only ever sees a fully
    // written file; a truncated download (e.g. a 1.7 GB
    // checkpoint whose connection drops partway through) leaves
    // a `<dest>.part` next to a non-existent `<dest>` and is
    // retried on the next call. The `.part` suffix is removed
    // on any error path so it does not accumulate across runs.
    let part = dest_with_suffix(&dest, ".part");
    let mut file = match std::fs::File::create(&part) {
        Ok(f) => f,
        Err(e) => {
            return Err(FixtureError::Network {
                fixture: name.to_string(),
                message: format!("create {}: {e}", part.display()),
                source: Some(Box::new(e)),
            });
        }
    };
    if let Err(e) = std::io::copy(&mut reader, &mut file) {
        drop(file);
        let _ = std::fs::remove_file(&part);
        return Err(FixtureError::Network {
            fixture: name.to_string(),
            message: format!("write {}: {e}", part.display()),
            source: Some(Box::new(e)),
        });
    }
    if let Err(e) = std::fs::rename(&part, &dest) {
        let _ = std::fs::remove_file(&part);
        return Err(FixtureError::Network {
            fixture: name.to_string(),
            message: format!("rename {} -> {}: {e}", part.display(), dest.display()),
            source: Some(Box::new(e)),
        });
    }
    Ok(dest)
}

/// Append `suffix` to `path`'s file name, leaving the parent
/// directory untouched. Used to build the sibling `.part` path
/// for the atomic-download dance above.
fn dest_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|s| s.to_os_string())
        .unwrap_or_default();
    name.push(suffix);
    match path.parent() {
        Some(parent) => parent.join(name),
        None => PathBuf::from(name),
    }
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
