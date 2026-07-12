//! Curated list of model identifiers that
//! [`crate::source::HuggingFaceSource::list_available`] returns.
//!
//! We do **not** search the HF Hub for arbitrary repos in Phase 2;
//! the list is hand-maintained and only includes models with
//! confirmed good behaviour under `voxora-hf`. Adding a model here
//! is a deliberate code change so the public surface stays small.

use voxora_core::{ModelCapabilities, ModelDescriptor};

/// A static list of [`ModelDescriptor`]s known to work today.
///
/// Each entry must include at least an `id` in `org/name` form. The
/// `capabilities` field is filled in lazily by
/// [`crate::source::HuggingFaceSource::capabilities_for`] if the
/// caller asks, so the static list only sets what we can confirm
/// without a network round-trip.
pub(crate) fn curated() -> Vec<ModelDescriptor> {
    vec![
        ModelDescriptor::with_details(
            "Qwen/Qwen3-ASR-0.6B",
            Some("Qwen3-ASR 0.6B".into()),
            Some(ModelCapabilities::new(
                true,
                false,
                false,
                qwen3_languages(),
            )),
        ),
        ModelDescriptor::with_details(
            "Qwen/Qwen3-ASR-1.7B",
            Some("Qwen3-ASR 1.7B".into()),
            Some(ModelCapabilities::new(
                true,
                false,
                false,
                qwen3_languages(),
            )),
        ),
        ModelDescriptor::with_details(
            "openai/whisper-tiny",
            Some("Whisper tiny (multilingual)".into()),
            Some(ModelCapabilities::new(
                true,
                true,
                false,
                vec![
                    "en".into(),
                    "es".into(),
                    "fr".into(),
                    "de".into(),
                    "zh".into(),
                    "ja".into(),
                    "ko".into(),
                    "pt".into(),
                    "ru".into(),
                    "it".into(),
                ],
            )),
        ),
        ModelDescriptor::with_details(
            "ggerganov/whisper.cpp",
            Some("whisper.cpp GGUF models".into()),
            Some(ModelCapabilities::new(true, true, false, vec!["en".into()])),
        ),
    ]
}

/// Subset of Qwen3-ASR's published language list (kept in sync with
/// `support_languages` in the upstream `config.json`).
fn qwen3_languages() -> Vec<String> {
    vec![
        "chinese".into(),
        "english".into(),
        "cantonese".into(),
        "arabic".into(),
        "german".into(),
        "french".into(),
        "spanish".into(),
        "portuguese".into(),
        "indonesian".into(),
        "italian".into(),
        "korean".into(),
        "russian".into(),
        "thai".into(),
        "vietnamese".into(),
        "japanese".into(),
        "turkish".into(),
        "hindi".into(),
        "malay".into(),
        "dutch".into(),
        "swedish".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curated_is_non_empty_and_well_formed() {
        let list = curated();
        assert!(!list.is_empty());
        for d in &list {
            assert!(d.id.contains('/'), "bad id: {}", d.id);
            assert!(!d.id.starts_with('/'));
            assert!(!d.id.ends_with('/'));
        }
    }

    #[test]
    fn curated_ids_are_unique() {
        let list = curated();
        let mut seen = std::collections::HashSet::new();
        for d in &list {
            assert!(seen.insert(d.id.clone()), "duplicate id: {}", d.id);
        }
    }
}
