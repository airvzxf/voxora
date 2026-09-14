//! Error type returned by every voxora operation.

use std::path::PathBuf;

/// All errors a voxora engine or model source may return.
///
/// `#[non_exhaustive]` so we can add variants in future minor releases
/// without breaking downstream `match` arms.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AsrError {
    /// The requested model could not be located by any known source.
    #[error("model not found: {0}")]
    ModelNotFound(String),

    /// The requested operation is not supported by this engine / source.
    #[error("operation not supported: {0}")]
    Unsupported(&'static str),

    /// Caller-supplied input was rejected (bad audio format, unknown
    /// language code, out-of-range parameter, …).
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Audio file I/O failed.
    #[error("audio I/O error at {}: {source}", path.display())]
    AudioIo {
        /// Path that failed to read or write.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The inference pass failed inside the engine (numerical error,
    /// shape mismatch, OOM, …).
    #[error("inference failed: {0}")]
    Inference(String),

    /// The model or runtime configuration is invalid.
    #[error("configuration error: {0}")]
    Config(String),

    /// The advisory lock on the model's cache directory could not
    /// be acquired within the bounded retry budget (closes
    /// [#208](https://github.com/airvzxf/voxora/issues/208)).
    ///
    /// Two concurrent `HuggingFaceSource::resolve` calls against
    /// the same `(model_id, revision)` could not be serialised:
    /// the loser kept timing out on `try_lock_exclusive`. Distinct
    /// from [`AsrError::AudioIo`] because a lock-contention retry
    /// is a transient coordination signal — chained sources should
    /// fall through to the next source rather than surface it as
    /// a fatal I/O failure.
    #[error("could not acquire lock at {} after {attempts} attempt(s): {message}", path.display())]
    LockUnavailable {
        /// Path to the `.lock` file we failed to take.
        path: PathBuf,
        /// Number of attempts before giving up (1-indexed; the budget).
        attempts: u32,
        /// Human-readable description of the failure mode.
        message: String,
    },

    /// Network failure while acquiring a model (DNS, TCP, TLS, HTTP
    /// transport, timeout, non-success status, or auth challenge).
    ///
    /// The `voxora-traits` crate stays offline-pure (no `reqwest`, no
    /// `tokio`); this variant only carries a `String` URL, a `String`
    /// message, and an optional boxed `std::error::Error`. The actual
    /// network code lives in `voxora-hf`.
    #[error("network error at {url}: {message}")]
    Network {
        /// URL that failed, if known.
        url: String,
        /// Human-readable description of the failure mode.
        message: String,
        /// Underlying error, when available.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl AsrError {
    /// Construct an [`AsrError::AudioIo`] from an I/O error and a path.
    pub fn audio_io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::AudioIo {
            path: path.into(),
            source,
        }
    }

    /// Construct an [`AsrError::Network`] from a URL, a message, and an
    /// optional inner error.
    pub fn network(
        url: impl Into<String>,
        message: impl Into<String>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::Network {
            url: url.into(),
            message: message.into(),
            source,
        }
    }

    /// Construct an [`AsrError::LockUnavailable`] from a path, attempt
    /// count, and message. See the variant docs for the contract.
    pub fn lock_unavailable(
        path: impl Into<PathBuf>,
        attempts: u32,
        message: impl Into<String>,
    ) -> Self {
        Self::LockUnavailable {
            path: path.into(),
            attempts,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    /// Closes #208: the new variant round-trips through the
    /// constructor, carries all three fields, and renders the path
    /// + attempts + message in `Display`.
    #[test]
    fn lock_unavailable_helper_constructs_variant_with_path_attempts_message() {
        let err =
            AsrError::lock_unavailable("/cache/Qwen/Qwen3-ASR-0.6B/main/.lock", 16, "WouldBlock");
        match err {
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
                assert_eq!(message, "WouldBlock");
            }
            other => panic!("expected LockUnavailable, got {other:?}"),
        }
    }

    #[test]
    fn lock_unavailable_display_includes_path_attempts_message() {
        let err = AsrError::lock_unavailable("/cache/foo/.lock", 16, "WouldBlock");
        let rendered = err.to_string();
        assert!(rendered.contains("/cache/foo/.lock"), "{rendered}");
        assert!(rendered.contains("16"), "{rendered}");
        assert!(rendered.contains("WouldBlock"), "{rendered}");
    }

    #[test]
    fn lock_unavailable_has_no_source_chain() {
        let err = AsrError::lock_unavailable("/x", 4, "boom");
        // The variant intentionally does NOT carry an `#[source]` —
        // the underlying `HfError::LockUnavailable` is already
        // stringified at the trait boundary. Mirrors
        // `AsrError::InvalidInput` / `AsrError::ModelNotFound`.
        assert!(err.source().is_none());
    }

    #[test]
    fn display_messages_are_stable() {
        assert_eq!(
            AsrError::ModelNotFound("foo".into()).to_string(),
            "model not found: foo"
        );
        assert_eq!(
            AsrError::Unsupported("list_available").to_string(),
            "operation not supported: list_available"
        );
        assert_eq!(
            AsrError::InvalidInput("bad lang".into()).to_string(),
            "invalid input: bad lang"
        );
        assert_eq!(
            AsrError::Inference("NaN".into()).to_string(),
            "inference failed: NaN"
        );
        assert_eq!(
            AsrError::Config("missing tokenizer".into()).to_string(),
            "configuration error: missing tokenizer"
        );
    }

    #[test]
    fn audio_io_helper_wraps_inner_error() {
        let inner = std::io::Error::new(std::io::ErrorKind::NotFound, "missing.wav");
        let err = AsrError::audio_io("/tmp/missing.wav", inner);
        match err {
            AsrError::AudioIo { path, source } => {
                assert_eq!(path, PathBuf::from("/tmp/missing.wav"));
                assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
            }
            other => panic!("expected AudioIo, got {other:?}"),
        }
    }

    #[test]
    fn audio_io_display_includes_path_and_source() {
        let err = AsrError::audio_io("/data/x.wav", std::io::Error::other("disk gone"));
        let rendered = err.to_string();
        assert!(rendered.contains("/data/x.wav"), "{rendered}");
        assert!(rendered.contains("disk gone"), "{rendered}");
    }

    #[test]
    fn source_chain_is_walkable() {
        let err = AsrError::audio_io("/p", std::io::Error::other("boom"));
        let chain = err.source();
        assert!(chain.is_some(), "audio_io must expose its inner io::Error");
        let first = chain.expect("checked is_some");
        assert_eq!(first.to_string(), "boom");
        assert!(first.source().is_none());
    }

    #[test]
    fn network_helper_constructs_variant_with_url_and_message() {
        let inner = std::io::Error::other("connection reset");
        let err = AsrError::network(
            "https://huggingface.co/foo/bar/resolve/main/config.json",
            "HTTP 503",
            Some(Box::new(inner)),
        );
        match err {
            AsrError::Network {
                ref url,
                ref message,
                ref source,
            } => {
                assert_eq!(
                    url,
                    "https://huggingface.co/foo/bar/resolve/main/config.json"
                );
                assert_eq!(message, "HTTP 503");
                let src = source.as_deref().expect("source must be present");
                assert_eq!(src.to_string(), "connection reset");
            }
            other => panic!("expected Network, got {other:?}"),
        }
    }

    #[test]
    fn network_display_includes_url_and_message() {
        let err = AsrError::network("https://huggingface.co/x", "DNS failure", None);
        let rendered = err.to_string();
        assert!(rendered.contains("https://huggingface.co/x"), "{rendered}");
        assert!(rendered.contains("DNS failure"), "{rendered}");
    }

    #[test]
    fn network_with_no_source_walks_to_none() {
        let err = AsrError::network("u", "m", None);
        assert!(err.source().is_none());
    }
}
