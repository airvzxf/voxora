//! Error type used inside `voxora-hf`.
//!
//! All network, I/O, and JSON failures inside this crate are first
//! converted to [`HfError`] and then mapped to [`voxora_traits::AsrError`]
//! at the public boundary. This keeps the crate's internal error story
//! rich (typed variants help with `From` impls) without leaking
//! `reqwest` / `tokio` types into the trait surface.

use std::path::PathBuf;

use voxora_traits::AsrError;

/// All errors that may occur inside `voxora-hf`.
#[derive(Debug, thiserror::Error)]
pub enum HfError {
    /// HTTP transport failure (DNS, TCP, TLS, timeout, redirect loop).
    #[error("transport error fetching {url}: {message}")]
    Transport {
        /// Request URL.
        url: String,
        /// Human-readable description.
        message: String,
        /// Underlying error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// The remote returned a non-success status code.
    #[error("HTTP {status} at {url}: {body}")]
    HttpStatus {
        /// Request URL.
        url: String,
        /// Numeric HTTP status (e.g. `404`).
        status: u16,
        /// Trimmed body for diagnostics (may be empty).
        body: String,
    },

    /// Local file I/O failure.
    #[error("I/O error at {}: {message}", path.display())]
    Io {
        /// Path that failed.
        path: PathBuf,
        /// Human-readable description.
        message: String,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },

    /// JSON parsing failure.
    #[error("JSON parse error at {context}: {source}")]
    Json {
        /// Where the JSON was read from (file path, URL, …).
        context: String,
        /// Underlying error.
        #[source]
        source: serde_json::Error,
    },

    /// The response shape did not match what we expected (missing
    /// `siblings`, no `weight_map`, etc.).
    #[error("unexpected response shape at {url}: {message}")]
    Protocol {
        /// Request URL that returned the bad payload.
        url: String,
        /// Human-readable description.
        message: String,
    },

    /// Transient retry policy was exhausted without success
    /// (closes [#113](https://github.com/airvzxf/voxora/issues/113)).
    /// The last underlying error is preserved so callers can
    /// distinguish "service is down" from "deterministic 4xx".
    #[error("retries exhausted after {attempts} attempt(s) at {url}: {last_error}")]
    RetriesExhausted {
        /// Request URL.
        url: String,
        /// Number of attempts made (1-indexed; e.g. 3 = first try
        /// plus two retries).
        attempts: u32,
        /// Stringified last error (the underlying `HfError` is
        /// not stored because most variants are already owned by
        /// a different layer; the message is enough for logs).
        last_error: String,
    },

    /// Caller-supplied input was rejected before any I/O.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// The advisory lock on `<model_dir>/.lock` could not be
    /// acquired within the bounded retry budget (closes
    /// [#185](https://github.com/airvzxf/voxora/issues/185) and
    /// [#208](https://github.com/airvzxf/voxora/issues/208)).
    /// Two concurrent `HuggingFaceSource::resolve` calls against
    /// the same `(model_id, revision)` could not be serialised:
    /// the loser kept timing out on `try_lock_exclusive`. Surfaces
    /// to the caller as the typed `AsrError::LockUnavailable` so
    /// chained sources can fall through on lock contention rather
    /// than treating it as a fatal I/O failure.
    #[error("could not acquire lock at {} after {attempts} attempt(s): {message}", path.display())]
    LockUnavailable {
        /// Path to the `.lock` file we failed to take.
        path: PathBuf,
        /// Number of attempts before giving up (1-indexed; the budget).
        attempts: u32,
        /// Stringified underlying error from the last `try_lock_exclusive`.
        message: String,
    },
}

impl HfError {
    /// Convert to the public [`AsrError`] type.
    pub fn into_asr(self) -> AsrError {
        match self {
            HfError::Transport {
                url,
                message,
                source,
            } => AsrError::network(url, message, Some(source)),
            HfError::HttpStatus { url, status, body } => AsrError::network(
                url,
                format!("HTTP {status}: {}", truncate(&body, 200)),
                None,
            ),
            HfError::Io {
                path,
                message,
                source,
            } => AsrError::audio_io(path, std::io::Error::new(source.kind(), message)),
            HfError::Json { context, source } => {
                AsrError::InvalidInput(format!("JSON at {context}: {source}"))
            }
            HfError::Protocol { url, message } => {
                AsrError::InvalidInput(format!("{url}: {message}"))
            }
            HfError::RetriesExhausted {
                url,
                attempts,
                last_error,
            } => AsrError::network(
                url,
                format!("retries exhausted after {attempts} attempt(s): {last_error}"),
                None,
            ),
            HfError::InvalidInput(msg) => AsrError::InvalidInput(msg),
            HfError::LockUnavailable {
                path,
                attempts,
                message,
            } => AsrError::lock_unavailable(
                path,
                attempts,
                format!("try_lock_exclusive returned WouldBlock {attempts} times: {message}"),
            ),
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    // Walk the string by char boundary so we never slice through the
    // middle of a multi-byte UTF-8 codepoint. `s[..max]` would panic
    // when the byte at index `max` is not a char boundary, which is
    // common for HTTP error bodies (e.g. a Chinese / Japanese / Korean
    // proxy error message).
    let end = s
        .char_indices()
        .take_while(|(i, _)| *i < max)
        .last()
        .map_or(0, |(i, c)| i + c.len_utf8());
    let mut out = s[..end].to_string();
    out.push('…');
    out
}

impl From<HfError> for AsrError {
    fn from(value: HfError) -> Self {
        value.into_asr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::truncate;

    /// Closes #208: `HfError::LockUnavailable` must map to
    /// `AsrError::LockUnavailable` (not `AudioIo{WouldBlock}` as
    /// before the variant landed). Chained sources key off this
    /// variant to fall through to the fallback on lock contention.
    #[test]
    fn lock_unavailable_maps_to_typed_variant() {
        let hf_err = HfError::LockUnavailable {
            path: PathBuf::from("/cache/Qwen/Qwen3-ASR-0.6B/main/.lock"),
            attempts: 16,
            message: "WouldBlock".into(),
        };
        let asr = hf_err.into_asr();
        match asr {
            AsrError::LockUnavailable {
                ref path,
                ref attempts,
                ref message,
            } => {
                assert_eq!(
                    path,
                    &PathBuf::from("/cache/Qwen/Qwen3-ASR-0.6B/main/.lock")
                );
                assert_eq!(*attempts, 16);
                assert!(
                    message.contains("WouldBlock"),
                    "message must preserve the underlying kind: {message}"
                );
            }
            other => panic!("expected LockUnavailable, got {other:?}"),
        }
    }

    /// Closes #208 (defense in depth): the mapping does not produce
    /// `AudioIo` any more, so consumer code that patterns on the
    /// new variant does not have to defend against a stale
    /// `AudioIo{WouldBlock}` shape.
    #[test]
    fn lock_unavailable_does_not_collapse_to_audio_io() {
        let hf_err = HfError::LockUnavailable {
            path: PathBuf::from("/x/.lock"),
            attempts: 16,
            message: "x".into(),
        };
        let asr = hf_err.into_asr();
        assert!(
            !matches!(asr, AsrError::AudioIo { .. }),
            "LockUnavailable must NOT map to AudioIo"
        );
    }

    #[test]
    fn truncate_short_string_passes_through() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_ascii_at_byte_boundary() {
        assert_eq!(truncate("hello world", 5), "hello…");
    }

    #[test]
    fn truncate_respects_utf8_char_boundaries() {
        // The ellipsis at byte index 3 is mid-codepoint for the
        // first `é` (2 bytes in UTF-8). `s[..3]` would panic with
        // "byte index 3 is not a char boundary".
        let s = "ééééé";
        let out = truncate(s, 3);
        // We get either "" + … (all 5 chars start after byte 3) or
        // "é" + … (1 full char fits in 2 bytes, byte 3 is the start
        // of the second char). Either is safe; what we MUST NOT see
        // is a panic.
        assert!(out.ends_with('…'));
        assert!(
            out.len() <= "é…".len() + 3,
            "output must be within 1 char + ellipsis: {out:?}"
        );
    }
}
