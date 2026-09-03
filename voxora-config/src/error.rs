//! Errors returned by voxora-config.
//!
//! Only the optional TOML file loader can fail — every other lookup in
//! the cascade either finds a value or falls through to the next layer,
//! so [`ConfigError`] is exclusively about reading and parsing a file.

use std::path::PathBuf;
use thiserror::Error;

/// All errors that may occur while loading a voxora configuration.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// The requested config file does not exist. Callers that treat a
    /// missing file as "just use the defaults" should match on this
    /// variant instead of ignoring every error.
    #[error("config file not found: {}", .0.display())]
    FileNotFound(PathBuf),

    /// The config file exists but could not be read.
    #[error("config file could not be read: {}: {message}", path.display())]
    FileIo {
        /// Path that failed to read.
        path: PathBuf,
        /// Human-readable description of the failing operation.
        message: String,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },

    /// The config file was read but is not valid TOML, or contains a
    /// key that does not belong to the schema.
    ///
    /// The parser error is boxed: `toml::de::Error` alone is larger
    /// than the whole rest of this enum, and every `Result` in the
    /// crate would pay for it.
    #[error("config file is not valid TOML: {}: {message}", path.display())]
    FileParse {
        /// Path that failed to parse (`"inline"` for string input).
        path: PathBuf,
        /// Human-readable description from the TOML parser.
        message: String,
        /// Underlying error.
        #[source]
        source: Box<toml::de::Error>,
    },
}
