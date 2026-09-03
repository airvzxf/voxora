//! Built-in descriptors and a convenience constructor for the
//! canonical HF-backed registry.

use voxora_core::ModelCapabilities;
use voxora_engine::EngineFamily;

use crate::descriptor::EngineDescriptor;
use crate::id::SourceKind;

/// Match an HF `org/repo` string exactly or as a strict parent
/// (`"expected/anything"`). Rejects lookalikes like
/// `"ggerganov/whisper.cpp-fork"` that a plain `starts_with` would
/// have accepted.
fn is_hf_repo(id_repo: &str, expected: &str) -> bool {
    id_repo == expected || id_repo.starts_with(&format!("{expected}/"))
}

/// Descriptor that accepts any `ggerganov/whisper.cpp` model id.
pub fn builtin_whisper_descriptor() -> EngineDescriptor {
    EngineDescriptor::new(
        EngineFamily::Whisper,
        "ggerganov/whisper.cpp ggml",
        |id| {
            matches!(id.source, SourceKind::HuggingFace)
                && is_hf_repo(&id.repo, "ggerganov/whisper.cpp")
        },
        ModelCapabilities::UNKNOWN,
    )
}

/// Descriptor that accepts the `Qwen/Qwen3-ASR` HF repo (whole-repo
/// or any file inside it).
pub fn builtin_qwen3asr_descriptor() -> EngineDescriptor {
    EngineDescriptor::new(
        EngineFamily::Qwen3Asr,
        "Qwen/Qwen3-ASR",
        |id| matches!(id.source, SourceKind::HuggingFace) && is_hf_repo(&id.repo, "Qwen/Qwen3-ASR"),
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
        for id_str in ["Qwen/Qwen3-ASR", "Qwen/Qwen3-ASR/model.safetensors"] {
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

    #[test]
    fn whisper_descriptor_rejects_lookalike_repos() {
        let d = builtin_whisper_descriptor();
        for bad in [
            "ggerganov/whisper.cpp-fork",
            "ggerganov/whisper.cpp2",
            "ggerganov/whisper.cpp_old/model.bin",
        ] {
            let id = ModelId::parse(bad).unwrap();
            assert!(!(d.accepts)(&id), "must reject lookalike {bad}");
        }
    }

    #[test]
    fn qwen_descriptor_rejects_lookalike_repos() {
        let d = builtin_qwen3asr_descriptor();
        for bad in ["Qwen/Qwen3-ASR-old", "Qwen/Qwen3-ASR2-0.6B"] {
            let id = ModelId::parse(bad).unwrap();
            assert!(!(d.accepts)(&id), "must reject lookalike {bad}");
        }
    }
}
