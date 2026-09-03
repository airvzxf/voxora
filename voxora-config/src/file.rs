//! Optional TOML file loader for voxora-config.

use std::path::Path;

use crate::VoxoraConfig;
use crate::error::ConfigError;

impl VoxoraConfig {
    /// Load config from a TOML file at the given path. Returns
    /// `FileNotFound` if the file does not exist (other paths under
    /// the file are an explicit error).
    ///
    /// # Errors
    ///
    /// - [`ConfigError::FileNotFound`] when `path` does not exist.
    /// - [`ConfigError::FileIo`] when `path` exists but cannot be read.
    /// - [`ConfigError::FileParse`] when the contents are not valid
    ///   TOML or carry a key outside the schema.
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Err(ConfigError::FileNotFound(path.to_path_buf()));
        }
        let text = std::fs::read_to_string(path).map_err(|e| ConfigError::FileIo {
            path: path.to_path_buf(),
            message: "read_to_string".into(),
            source: e,
        })?;
        Self::from_str(&text, path)
    }

    /// Parse a TOML string into a VoxoraConfig. `path_for_errors` is
    /// only used to label error messages.
    ///
    /// # Errors
    ///
    /// - [`ConfigError::FileParse`] when `text` is not valid TOML or
    ///   carries a key outside the schema.
    pub fn from_str(text: &str, path_for_errors: &Path) -> Result<Self, ConfigError> {
        toml::from_str(text).map_err(|e| ConfigError::FileParse {
            path: path_for_errors.to_path_buf(),
            message: e.to_string(),
            source: Box::new(e),
        })
    }
}
