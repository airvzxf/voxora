//! Errors returned by voxora-backend.

use thiserror::Error;

/// Errors returned by voxora-backend when backend selection or
/// detection fails.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum BackendError {
    /// No compiled-in backend was usable at runtime.
    ///
    /// Carries both the list of backends that were selected at
    /// compile time and the (possibly empty) list of backends that
    /// were also detected as usable at runtime so callers can render
    /// a useful diagnostic.
    #[error("no usable backend found (compiled: {compiled:?}, available: {available:?})")]
    NoUsableBackend {
        /// Backends that the binary was built with.
        compiled: Vec<String>,
        /// Backends that compiled in AND were usable at runtime.
        available: Vec<String>,
    },
}
