//! Qwen3-ASR full-language-name ↔ voxora validation helpers.
//!
//! Upstream `qwen3-asr-rs` accepts the language as a full English name
//! (`"english"`, `"chinese"`, `"cantonese"`, …), not as an ISO 639-1
//! code. This module wraps the closed 20-entry list published in
//! `voxora-hf/src/known_models.rs::qwen3_languages()` so callers can
//! validate a user-supplied string before handing it to
//! [`crate::QwenAsrEngine::transcribe`].
//!
//! The list is duplicated here on purpose: `voxora-core` is
//! offline-pure and cannot depend on `voxora-hf`, and `voxora-qwen3asr`
//! must build without the `hf` feature. Keep the two lists in sync
//! when upstream adds a new language.

use voxora_core::AsrError;

/// Canonical list of full-language-names accepted by `qwen3-asr`.
///
/// Order matches the order returned by
/// `voxora_hf::known_models::qwen3_languages()`. Keep them aligned.
#[allow(dead_code)] // referenced from rustdoc; consumed by callers via known_languages()
pub(crate) const KNOWN_LANGUAGES: &[&str] = &[
    "chinese",
    "english",
    "cantonese",
    "arabic",
    "german",
    "french",
    "spanish",
    "portuguese",
    "indonesian",
    "italian",
    "korean",
    "russian",
    "thai",
    "vietnamese",
    "japanese",
    "turkish",
    "hindi",
    "malay",
    "dutch",
    "swedish",
];

/// Validate a caller-supplied language string against the closed set
/// accepted by upstream `qwen3-asr`.
///
/// Returns `Ok(())` when `name` matches one of the known full-name
/// languages (case-insensitive — upstream `qwen3-asr` lowercases
/// internally via `capitalize_first`, so `"English"` and `"english"`
/// both reach the same prompt prefix). Returns [`AsrError::InvalidInput`]
/// otherwise.
///
/// # Examples
///
/// ```
/// use voxora_qwen3asr::validate_lang;
///
/// assert!(validate_lang("english").is_ok());
/// assert!(validate_lang("English").is_ok());
/// assert!(validate_lang("xx").is_err());
/// assert!(validate_lang("en").is_err()); // ISO 639-1 is rejected — Qwen3 expects full names
/// ```
pub fn validate_lang(name: &str) -> Result<(), AsrError> {
    let lower = name.to_ascii_lowercase();
    if KNOWN_LANGUAGES.contains(&lower.as_str()) {
        Ok(())
    } else {
        Err(AsrError::InvalidInput(format!(
            "unknown qwen3-asr language: {name:?} (expected one of {KNOWN_LANGUAGES:?})"
        )))
    }
}

/// Borrowed slice of every language [`validate_lang`] accepts.
///
/// Convenience wrapper for callers that want the list as a
/// `&[&str]` (e.g. to populate a UI dropdown).
pub fn known_languages() -> &'static [&'static str] {
    KNOWN_LANGUAGES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_languages_is_non_empty_and_contains_english_and_chinese() {
        assert!(!KNOWN_LANGUAGES.is_empty());
        assert!(KNOWN_LANGUAGES.contains(&"english"));
        assert!(KNOWN_LANGUAGES.contains(&"chinese"));
    }

    #[test]
    fn known_languages_length_is_twenty() {
        // Pinned to the upstream config.json count; bump only when
        // Qwen adds a new language.
        assert_eq!(KNOWN_LANGUAGES.len(), 20, "language list drifted");
    }

    #[test]
    fn known_languages_have_no_duplicates() {
        let mut sorted: Vec<&str> = KNOWN_LANGUAGES.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), KNOWN_LANGUAGES.len(), "duplicate entry");
    }

    #[test]
    fn validate_lang_accepts_every_known_name() {
        for name in KNOWN_LANGUAGES {
            assert!(
                validate_lang(name).is_ok(),
                "validate_lang rejected a known language: {name:?}"
            );
        }
    }

    #[test]
    fn validate_lang_is_case_insensitive() {
        assert!(validate_lang("English").is_ok());
        assert!(validate_lang("ENGLISH").is_ok());
        assert!(validate_lang("chInEsE").is_ok());
    }

    #[test]
    fn validate_lang_rejects_iso_639_1_codes() {
        // The whole point of this adapter: whisper uses ISO codes,
        // qwen3-asr does not. Reject the ISO codes loudly so users
        // get a clear error.
        for code in ["en", "zh", "es", "fr", "de", "ja"] {
            assert!(
                validate_lang(code).is_err(),
                "ISO 639-1 code {code:?} should be rejected"
            );
        }
    }

    #[test]
    fn validate_lang_rejects_unknown_garbage() {
        let err = validate_lang("klingon").expect_err("klingon is not a qwen3 language");
        match err {
            AsrError::InvalidInput(msg) => {
                assert!(msg.contains("klingon"), "{msg}");
                assert!(msg.contains("english"), "{msg}");
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn validate_lang_rejects_empty_string() {
        assert!(validate_lang("").is_err());
    }

    #[test]
    fn known_languages_helper_returns_same_slice_as_constant() {
        assert_eq!(known_languages(), KNOWN_LANGUAGES);
    }
}
