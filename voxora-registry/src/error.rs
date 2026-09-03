//! Errors returned by voxora-registry.

use thiserror::Error;

/// All errors a [`crate::Registry`] may surface to its callers.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RegistryError {
    /// The supplied model id could not be parsed.
    #[error("could not parse model id: {0}")]
    Parse(String),

    /// No [`crate::descriptor::EngineDescriptor`] registered for this
    /// [`crate::id::ModelId`].
    #[error("no engine descriptor registered that accepts model id {0:?}")]
    NoMatchingDescriptor(String),

    /// The model id resolved to a directory but the requested file
    /// is not present on disk.
    #[error("model id resolved but the model file is missing on disk: {0:?}")]
    MissingModelFile(std::path::PathBuf),
}
