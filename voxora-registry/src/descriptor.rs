//! Engine descriptors that a [`crate::Registry`] knows about.

use voxora_core::ModelCapabilities;
use voxora_engine::EngineFamily;

use crate::id::ModelId;

/// What this engine wants to advertise in [`ModelCapabilities`].
///
/// Just a wrapper around the upstream [`ModelCapabilities`] so the
/// descriptor can carry it without re-inventing the wheel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineCapabilities(pub ModelCapabilities);

/// Static metadata about a registered engine family.
///
/// The registry holds a `Vec<EngineDescriptor>`; resolution is
/// first-match-wins over `accepts(&ModelId)`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct EngineDescriptor {
    /// Which [`EngineFamily`] this descriptor represents.
    pub family: EngineFamily,
    /// Free-form human-readable name (e.g. "ggerganov/whisper.cpp ggml").
    pub label: String,
    /// Predicate: does this engine family accept the given model id?
    pub accepts: fn(&ModelId) -> bool,
    /// Static capabilities advertised by this engine (used when
    /// [`ModelCapabilities::UNKNOWN`] is not acceptable).
    pub default_capabilities: EngineCapabilities,
}

impl EngineDescriptor {
    /// Build a new [`EngineDescriptor`] with the given family,
    /// human-readable label, accept predicate, and default
    /// capabilities.
    pub fn new(
        family: EngineFamily,
        label: impl Into<String>,
        accepts: fn(&ModelId) -> bool,
        default_capabilities: ModelCapabilities,
    ) -> Self {
        Self {
            family,
            label: label.into(),
            accepts,
            default_capabilities: EngineCapabilities(default_capabilities),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::SourceKind;

    fn whisper_descriptor() -> EngineDescriptor {
        EngineDescriptor::new(
            EngineFamily::Whisper,
            "ggerganov/whisper.cpp ggml",
            |id| {
                matches!(id.source, SourceKind::HuggingFace)
                    && id.repo.starts_with("ggerganov/whisper.cpp")
            },
            ModelCapabilities::UNKNOWN,
        )
    }

    #[test]
    fn whisper_descriptor_accepts_own_repo() {
        let d = whisper_descriptor();
        let id = ModelId::parse("ggerganov/whisper.cpp/ggml-tiny.bin").unwrap();
        assert!((d.accepts)(&id));
    }

    #[test]
    fn whisper_descriptor_rejects_other_repo() {
        let d = whisper_descriptor();
        let id = ModelId::parse("Qwen/Qwen3-ASR-0.6B").unwrap();
        assert!(!(d.accepts)(&id));
    }
}
