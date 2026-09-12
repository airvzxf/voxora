//! BCP-47 language whitelist for MiniMax.
//!
//! MiniMax supports exactly 20 BCP-47 tags today (per the
//! [`/v1/speech_to_text` API docs](https://platform.minimax.io/docs/api-reference/speech-to-text.md)).
//! An empty / missing `language` header enables mixed-language
//! recognition; any other value outside the whitelist is rejected
//! upstream with a 400 `bad_request_error`. We pre-validate
//! locally so the caller gets a clear error before the round-trip.
//!
//! Order matches the order listed in the upstream API doc. Keep
//! them aligned if MiniMax adds a new language.

use voxora_traits::AsrError;

/// Canonical list of BCP-47 language tags accepted by MiniMax.
///
/// The list is 20 entries. Mirror any upstream change here and
/// update the matching length assertion below.
pub(crate) const KNOWN_LANGUAGES_BCP47: &[&str] = &[
    "zh",  // Chinese
    "yue", // Cantonese
    "en",  // English
    "ja",  // Japanese
    "ko",  // Korean
    "th",  // Thai
    "vi",  // Vietnamese
    "id",  // Indonesian
    "ms",  // Malay
    "fil", // Filipino
    "ar",  // Arabic
    "tr",  // Turkish
    "fr",  // French
    "de",  // German
    "es",  // Spanish
    "it",  // Italian
    "pt",  // Portuguese
    "pl",  // Polish
    "ru",  // Russian
    "uk",  // Ukrainian
];

/// Validate a caller-supplied BCP-47 language tag against the
/// MiniMax whitelist.
///
/// Returns `Ok(())` for an empty / unset input (which upstream
/// treats as "auto-detect / mixed-language"), or for a tag that
/// matches one of the 20 known entries (case-insensitive). Returns
/// [`AsrError::InvalidInput`] otherwise.
///
/// # Examples
///
/// ```
/// use voxora_minimax::validate_lang_bcp47;
///
/// assert!(validate_lang_bcp47(Some("en")).is_ok());
/// assert!(validate_lang_bcp47(Some("EN")).is_ok());
/// assert!(validate_lang_bcp47(None).is_ok()); // mixed-language mode
/// assert!(validate_lang_bcp47(Some("klingon")).is_err());
/// ```
pub fn validate_lang_bcp47(lang: Option<&str>) -> Result<(), AsrError> {
    let Some(raw) = lang else {
        return Ok(());
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    let lower = trimmed.to_ascii_lowercase();
    if KNOWN_LANGUAGES_BCP47.contains(&lower.as_str()) {
        Ok(())
    } else {
        Err(AsrError::InvalidInput(format!(
            "unknown MiniMax language: {raw:?} (expected one of {KNOWN_LANGUAGES_BCP47:?})"
        )))
    }
}

/// Borrowed slice of every BCP-47 tag [`validate_lang_bcp47`]
/// accepts.
///
/// Convenience wrapper for callers that want the list as a
/// `&[&str]` (e.g. to populate a UI dropdown).
pub fn known_languages_bcp47() -> &'static [&'static str] {
    KNOWN_LANGUAGES_BCP47
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_languages_length_is_twenty() {
        assert_eq!(
            KNOWN_LANGUAGES_BCP47.len(),
            20,
            "language list drifted from the documented 20 entries"
        );
    }

    #[test]
    fn known_languages_contains_english_and_chinese() {
        assert!(KNOWN_LANGUAGES_BCP47.contains(&"en"));
        assert!(KNOWN_LANGUAGES_BCP47.contains(&"zh"));
        assert!(KNOWN_LANGUAGES_BCP47.contains(&"yue"));
    }

    #[test]
    fn known_languages_have_no_duplicates() {
        let mut sorted: Vec<&str> = KNOWN_LANGUAGES_BCP47.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), KNOWN_LANGUAGES_BCP47.len());
    }

    #[test]
    fn validate_accepts_every_known_tag() {
        for tag in KNOWN_LANGUAGES_BCP47 {
            assert!(validate_lang_bcp47(Some(tag)).is_ok());
        }
    }

    #[test]
    fn validate_is_case_insensitive() {
        assert!(validate_lang_bcp47(Some("EN")).is_ok());
        assert!(validate_lang_bcp47(Some("Yue")).is_ok());
    }

    #[test]
    fn validate_accepts_none_for_mixed_language() {
        // MiniMax interprets an empty/missing `language` header
        // as "auto-detect / mixed-language recognition".
        assert!(validate_lang_bcp47(None).is_ok());
        assert!(validate_lang_bcp47(Some("")).is_ok());
        assert!(validate_lang_bcp47(Some("   ")).is_ok());
    }

    #[test]
    fn validate_rejects_unknown_garbage() {
        let err =
            validate_lang_bcp47(Some("klingon")).expect_err("klingon is not a MiniMax language");
        match err {
            AsrError::InvalidInput(msg) => {
                assert!(msg.contains("klingon"), "{msg}");
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn known_languages_helper_returns_same_slice_as_constant() {
        assert_eq!(known_languages_bcp47(), KNOWN_LANGUAGES_BCP47);
    }
}
