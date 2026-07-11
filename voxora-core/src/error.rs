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
}

impl AsrError {
    /// Construct an [`AsrError::AudioIo`] from an I/O error and a path.
    pub fn audio_io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::AudioIo {
            path: path.into(),
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

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
}
