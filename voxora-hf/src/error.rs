//! Error type used inside `voxora-hf`.
//!
//! All network, I/O, and JSON failures inside this crate are first
//! converted to [`HfError`] and then mapped to [`voxora_core::AsrError`]
//! at the public boundary. This keeps the crate's internal error story
//! rich (typed variants help with `From` impls) without leaking
//! `reqwest` / `tokio` types into the trait surface.

use std::path::PathBuf;

use voxora_core::AsrError;

/// All errors that may occur inside `voxora-hf`.
#[derive(Debug, thiserror::Error)]
pub(crate) enum HfError {
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

    /// Caller-supplied input was rejected before any I/O.
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

impl HfError {
    /// Convert to the public [`AsrError`] type.
    pub(crate) fn into_asr(self) -> AsrError {
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
            HfError::InvalidInput(msg) => AsrError::InvalidInput(msg),
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut out = s[..max].to_string();
        out.push('…');
        out
    }
}

impl From<HfError> for AsrError {
    fn from(value: HfError) -> Self {
        value.into_asr()
    }
}
