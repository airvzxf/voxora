//! Metadata about a loaded engine.

use voxora_core::{AsrEngine, ModelCapabilities};

use crate::adapter::EngineAdapter;
use crate::family::EngineFamily;

/// Information about an [`AsrEngine`] (or its [`EngineAdapter`]).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EngineInfo {
    /// Which engine family this engine belongs to.
    pub family: EngineFamily,

    /// Capabilities reported by the underlying model.
    pub capabilities: ModelCapabilities,

    /// Optional human-readable identifier (e.g. whisper's
    /// `whisper_model_type_readable`).
    pub model_label: Option<String>,

    /// Optional on-disk path that the engine was loaded from.
    pub source_path: Option<std::path::PathBuf>,
}

impl EngineInfo {
    /// Construct an `EngineInfo` with the required fields; the
    /// optional `model_label` and `source_path` start as `None` and
    /// can be filled in via the builder helpers below.
    pub fn new(family: EngineFamily, capabilities: ModelCapabilities) -> Self {
        Self {
            family,
            capabilities,
            model_label: None,
            source_path: None,
        }
    }

    /// Set the human-readable model label and return `self` for
    /// chaining.
    pub fn with_model_label(mut self, label: impl Into<String>) -> Self {
        self.model_label = Some(label.into());
        self
    }

    /// Set the on-disk path the engine was loaded from and return
    /// `self` for chaining.
    pub fn with_source_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.source_path = Some(path.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_builds_minimal_info() {
        let info = EngineInfo::new(EngineFamily::Whisper, ModelCapabilities::UNKNOWN);
        assert_eq!(info.family, EngineFamily::Whisper);
        assert!(info.model_label.is_none());
        assert!(info.source_path.is_none());
    }

    #[test]
    fn builder_chain_appends_optional_fields() {
        let info = EngineInfo::new(EngineFamily::Qwen3Asr, ModelCapabilities::UNKNOWN)
            .with_model_label("qwen-tiny")
            .with_source_path("/cache/qwen");
        assert_eq!(info.model_label.as_deref(), Some("qwen-tiny"));
        assert_eq!(
            info.source_path,
            Some(std::path::PathBuf::from("/cache/qwen"))
        );
    }
}
