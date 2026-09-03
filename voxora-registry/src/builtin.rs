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

/// True iff `id_repo` is `family` exactly, `family/anything`, or
/// `family-anything` (e.g. `Qwen/Qwen3-ASR`, `Qwen/Qwen3-ASR/model.bin`,
/// `Qwen/Qwen3-ASR-0.6B`). The `-` suffix is the Qwen3-ASR family
/// convention for versioned siblings (0.6B, 1.7B, 2.0B); Whisper has
/// no `-`-suffix variants today but is forward-compatible.
fn is_family(id_repo: &str, family: &str) -> bool {
    is_hf_repo(id_repo, family) || id_repo.starts_with(&format!("{family}-"))
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

/// Descriptor that accepts the `Qwen/Qwen3-ASR` HF repo and its
/// `-` suffix siblings (e.g. `Qwen/Qwen3-ASR-0.6B`).
pub fn builtin_qwen3asr_descriptor() -> EngineDescriptor {
    EngineDescriptor::new(
        EngineFamily::Qwen3Asr,
        "Qwen/Qwen3-ASR",
        |id| matches!(id.source, SourceKind::HuggingFace) && is_family(&id.repo, "Qwen/Qwen3-ASR"),
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
        let bad = "Qwen/Qwen3-ASR2-0.6B";
        let id = ModelId::parse(bad).unwrap();
        assert!(!(d.accepts)(&id), "must reject lookalike {bad}");
    }

    #[test]
    fn qwen_descriptor_accepts_versioned_siblings() {
        let d = builtin_qwen3asr_descriptor();
        for good in [
            "Qwen/Qwen3-ASR",
            "Qwen/Qwen3-ASR/model.bin",
            "Qwen/Qwen3-ASR-0.6B",
            "Qwen/Qwen3-ASR-1.7B",
            "Qwen/Qwen3-ASR-2.0B",
            "Qwen/Qwen3-ASR-old",
        ] {
            let id = ModelId::parse(good).unwrap();
            assert!((d.accepts)(&id), "must accept {good}");
        }
    }

    #[test]
    fn qwen_descriptor_rejects_unrelated_repos() {
        let d = builtin_qwen3asr_descriptor();
        for bad in [
            "Qwen/Qwen2-ASR",
            "Qwen/Qwen3ASR",
            "Qwen/Qwen4-ASR",
            "OpenGVLab/InternVL",
        ] {
            let id = ModelId::parse(bad).unwrap();
            assert!(!(d.accepts)(&id), "must reject {bad}");
        }
    }

    #[test]
    fn whisper_descriptor_accepts_subpath_and_rejects_dash_variant() {
        let d = builtin_whisper_descriptor();
        let id_ok = ModelId::parse("ggerganov/whisper.cpp/ggml-tiny.bin").unwrap();
        assert!((d.accepts)(&id_ok));
        let id_dash = ModelId::parse("ggerganov/whisper.cpp-fork").unwrap();
        assert!(
            !(d.accepts)(&id_dash),
            "dash variant must still be rejected"
        );
    }
}
