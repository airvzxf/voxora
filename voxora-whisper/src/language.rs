//! ISO 639-1 ↔ whisper-rs language helpers.
//!
//! whisper.cpp ships its own internal language table. The free
//! functions in [`whisper_rs`] (`get_lang_id`, `get_lang_str`,
//! `get_lang_max_id`) bridge the two worlds; this module wraps them
//! in voxora-flavoured helpers that:
//!
//! - validate a user-supplied ISO 639-1 code (rejecting unknown
//!   inputs as [`AsrError::InvalidInput`]),
//! - enumerate the full known language set for
//!   [`crate::WhisperEngine::capabilities`], and
//! - convert a detected language id back to its ISO 639-1 code for
//!   [`TranscriptionResult::language`](voxora_core::TranscriptionResult::language).

use voxora_core::AsrError;

/// Validate a user-supplied ISO 639-1 code against whisper.cpp's
/// built-in language table.
///
/// Returns the whisper.cpp internal language id on success
/// (always non-negative), or [`AsrError::InvalidInput`] when the
/// code is not in whisper's table.
///
/// # Examples
///
/// ```
/// use voxora_whisper::validate_lang;
///
/// assert!(validate_lang("en").is_ok());
/// assert!(validate_lang("es").is_ok());
/// assert!(validate_lang("xx").is_err());
/// ```
pub fn validate_lang(code: &str) -> Result<i32, AsrError> {
    match whisper_rs::get_lang_id(code) {
        Some(id) => Ok(id),
        None => Err(AsrError::InvalidInput(format!(
            "unknown whisper language code: {code:?}"
        ))),
    }
}

/// Return the ISO 639-1 code that whisper.cpp uses for the given
/// internal language id, or `None` if the id is out of range.
///
/// Used after a `detect_language` pass to populate
/// [`voxora_core::TranscriptionResult::language`].
pub fn iso_code_from_id(id: i32) -> Option<String> {
    whisper_rs::get_lang_str(id).map(str::to_string)
}

/// Enumerate every language whisper.cpp knows about, in id order.
///
/// Always returns at least one entry (whisper.cpp always supports
/// English even on `.en` checkpoints).
pub fn known_languages() -> Vec<String> {
    let max = whisper_rs::get_lang_max_id();
    (0..=max)
        .filter_map(|id| whisper_rs::get_lang_str(id).map(str::to_string))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_lang_accepts_common_iso_codes() {
        assert!(validate_lang("en").is_ok());
        assert!(validate_lang("es").is_ok());
        assert!(validate_lang("fr").is_ok());
        assert!(validate_lang("ja").is_ok());
    }

    #[test]
    fn validate_lang_rejects_garbage() {
        let err = validate_lang("xx").expect_err("xx is not a whisper language");
        match err {
            AsrError::InvalidInput(msg) => {
                assert!(
                    msg.contains("xx"),
                    "message should mention the bad code: {msg}"
                );
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn validate_lang_rejects_empty_string() {
        assert!(validate_lang("").is_err());
    }

    #[test]
    fn iso_code_from_id_round_trips() {
        let id = validate_lang("en").expect("en is valid");
        assert_eq!(iso_code_from_id(id).as_deref(), Some("en"));
    }

    #[test]
    fn iso_code_from_id_out_of_range_is_none() {
        let max = whisper_rs::get_lang_max_id();
        assert!(iso_code_from_id(max + 1).is_none());
        assert!(iso_code_from_id(-1).is_none());
    }

    #[test]
    fn known_languages_is_non_empty_and_contains_english() {
        let langs = known_languages();
        assert!(!langs.is_empty());
        assert!(
            langs.iter().any(|l| l == "en"),
            "expected English in known languages, got {langs:?}"
        );
    }

    #[test]
    fn known_languages_length_matches_max_id() {
        let langs = known_languages();
        let expected = (whisper_rs::get_lang_max_id() + 1) as usize;
        assert_eq!(langs.len(), expected);
    }

    #[test]
    fn known_languages_have_no_duplicates() {
        let langs = known_languages();
        let mut sorted = langs.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), langs.len(), "duplicate language entries");
    }
}
