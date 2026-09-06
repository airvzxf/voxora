//! Built-in descriptors and a convenience constructor for the
//! canonical HF-backed registry.

use voxora_engine::EngineFamily;
use voxora_traits::ModelCapabilities;

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

/// Descriptor that accepts any [`SourceKind::Local`] id (closes #120).
///
/// The descriptor's `accepts` predicate is `|_| true` restricted to
/// the [`SourceKind::Local`] arm — every well-formed local id passes
/// the accept arm, and the registry's underlying `ModelSource`
/// (which is expected to be a `voxora_local::ChainedSource`
/// configured with a `LocalSource` as the primary) decides whether
/// the path actually exists on disk.
///
/// ## Engine-family routing
///
/// The descriptor's `family` is [`EngineFamily::Whisper`] by default
/// (matching `LocalSource::resolve` which joins `model_id` against
/// the configured `local_root` and returns a single-file
/// [`voxora_traits::ModelDir`] — the shape the Whisper engine loader
/// consumes directly). The minimum-viable dispatch path for a
/// consumer is therefore:
///
/// ```text
/// match resolved.descriptor.family {
///     EngineFamily::Whisper => WhisperEngine::load(&resolved.model_dir),
///     EngineFamily::Qwen3Asr => QwenAsrEngine::load(&resolved.model_dir),
/// }
/// ```
///
/// Consumers who need to load a Local-resolved directory through
/// the Qwen3-ASR engine (e.g. a `Qwen3-ASR` checkpoint vendored
/// under `local_root`) must register their own descriptor with
/// `family = EngineFamily::Qwen3Asr` **before** calling
/// `Registry::with_builtin_descriptors_and_chained_source`. The
/// default registration order — Whisper HF, Qwen3-ASR HF, then
/// this Local fallback — means a registered Qwen descriptor always
/// wins over the Local fallback for matching ids.
///
/// ## Path-joining semantics
///
/// `LocalSource::resolve` does `root.join(model_id)`. Rust's
/// `PathBuf::join` semantics: an absolute `model_id` replaces the
/// prefix, so absolute Local ids (e.g. `/srv/models/qwen.bin`)
/// resolve verbatim against the filesystem; relative ids (e.g.
/// `org/repo/file.bin`) join under the configured `local_root`.
/// Both shapes work without changes to the descriptor or the chain.
pub fn builtin_local_descriptor(family: EngineFamily) -> EngineDescriptor {
    EngineDescriptor::new(
        family,
        "local filesystem",
        |id| matches!(id.source, SourceKind::Local),
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

    #[test]
    fn local_descriptor_accepts_any_local_id() {
        let d = builtin_local_descriptor(EngineFamily::Whisper);
        for id_str in [
            "/srv/models/qwen.bin",
            "/cache/models/whisper/ggml-tiny.bin",
            "./local-model",
            "../sibling/model.bin",
        ] {
            let id = ModelId::parse(id_str).unwrap();
            assert!((d.accepts)(&id), "expected {id_str} to match local arm");
        }
    }

    #[test]
    fn local_descriptor_rejects_hf_ids() {
        let d = builtin_local_descriptor(EngineFamily::Whisper);
        for id_str in ["Qwen/Qwen3-ASR-0.6B", "ggerganov/whisper.cpp/ggml-tiny.bin"] {
            let id = ModelId::parse(id_str).unwrap();
            assert!(
                !(d.accepts)(&id),
                "local descriptor must reject HF id {id_str}"
            );
        }
    }

    #[test]
    fn local_descriptor_honours_configured_family() {
        let d_qwen = builtin_local_descriptor(EngineFamily::Qwen3Asr);
        assert_eq!(d_qwen.family, EngineFamily::Qwen3Asr);
        let d_whisper = builtin_local_descriptor(EngineFamily::Whisper);
        assert_eq!(d_whisper.family, EngineFamily::Whisper);
    }
}
