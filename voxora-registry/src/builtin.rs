//! Built-in descriptors and a convenience constructor for the
//! canonical HF-backed registry.

use voxora_core::ModelCapabilities;
use voxora_engine::EngineFamily;

use crate::descriptor::EngineDescriptor;
use crate::id::SourceKind;

/// Descriptor that accepts any `ggerganov/whisper.cpp` model id.
pub fn builtin_whisper_descriptor() -> EngineDescriptor {
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

/// Descriptor that accepts any `Qwen/Qwen3-ASR-*` model id.
pub fn builtin_qwen3asr_descriptor() -> EngineDescriptor {
    EngineDescriptor::new(
        EngineFamily::Qwen3Asr,
        "Qwen/Qwen3-ASR",
        |id| matches!(id.source, SourceKind::HuggingFace) && id.repo.starts_with("Qwen/Qwen3-ASR"),
        ModelCapabilities::UNKNOWN,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::ModelId;

    #[test]
    fn whisper_descriptor_matches_whisper_cpp_ids() {
        let d = builtin_whisper_descriptor();
        for id_str in [
            "ggerganov/whisper.cpp",
            "ggerganov/whisper.cpp/ggml-tiny.bin",
            "ggerganov/whisper.cpp/ggml-large-v3.bin",
        ] {
            let id = ModelId::parse(id_str).unwrap();
            assert!((d.accepts)(&id), "expected {id_str} to match");
        }
    }

    #[test]
    fn qwen_descriptor_matches_qwen3_asr_ids() {
        let d = builtin_qwen3asr_descriptor();
        for id_str in ["Qwen/Qwen3-ASR-0.6B", "Qwen/Qwen3-ASR-1.7B"] {
            let id = ModelId::parse(id_str).unwrap();
            assert!((d.accepts)(&id), "expected {id_str} to match");
        }
    }

    #[test]
    fn whisper_descriptor_rejects_qwen() {
        let d = builtin_whisper_descriptor();
        let id = ModelId::parse("Qwen/Qwen3-ASR-0.6B").unwrap();
        assert!(!(d.accepts)(&id));
    }
}
