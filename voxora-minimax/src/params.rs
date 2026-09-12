//! Mapping between voxora's [`TranscribeOptions`] and the
//! MiniMax `/v1/speech_to_text` request shape.
//!
//! MiniMax carries the `language` parameter as a header (per the
//! upstream OpenAPI doc — it lives under `parameters`, not
//! `requestBody.properties`). All other knobs map onto
//! `multipart/form-data` fields:
//!
//! - `timestamps = true` → `timestamp_level=word`
//!   (English gets word-level, Chinese gets character-level; for
//!   the rest MiniMax falls back to sentence boundaries, but the
//!   field is honoured the same way).
//! - `translate = true` → MiniMax has no translate mode; we
//!   surface [`AsrError::Unsupported`] so the caller gets a clear
//!   message.
//!
//! Pulled out into a small value type ([`MiniMaxParams`]) so the
//! layout can be unit-tested without firing HTTP.

use voxora_traits::{AsrError, TranscribeOptions};

/// Resolved MiniMax request parameters (the ones that map onto
/// multipart fields plus the language header).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct MiniMaxParams {
    /// BCP-47 language tag for the `language` header. `None`
    /// means "auto-detect / mixed-language" — the engine omits
    /// the header entirely.
    pub language: Option<String>,
    /// Timestamp granularity: `""` (sentence, the default) or
    /// `"word"`. Only meaningful when `response_format=verbose_json`
    /// (which is the only format this engine requests).
    pub timestamp_level: TimestampLevel,
}

/// Timestamp granularity requested via the multipart field.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum TimestampLevel {
    /// Sentence / segment boundaries (the MiniMax default for
    /// `timestamp_level=""`).
    #[default]
    Sentence,
    /// Word-level timestamps (English) / character-level
    /// timestamps (Chinese). Other languages fall back to
    /// sentence boundaries upstream.
    Word,
}

impl TimestampLevel {
    /// Wire representation on the multipart field.
    fn as_str(&self) -> &'static str {
        match self {
            Self::Sentence => "",
            Self::Word => "word",
        }
    }
}

/// Trait that lets the [`MiniMaxParams`] value emit both its
/// header form (for the `language` header) and its multipart
/// fields.
pub trait MiniMaxParamsApply {
    /// What to send in the `language` HTTP header. `None` means
    /// "omit the header entirely" (the upstream mixed-language /
    /// auto-detect path).
    fn language_header(&self) -> Option<&str>;

    /// Multipart field pairs to add to the form body, beyond the
    /// `model` + `response_format` + `file` baseline.
    fn multipart_fields(&self) -> Vec<(&'static str, &str)>;
}

impl MiniMaxParamsApply for MiniMaxParams {
    fn language_header(&self) -> Option<&str> {
        self.language
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }

    fn multipart_fields(&self) -> Vec<(&'static str, &str)> {
        // `timestamp_level` is always emitted (matches the upstream
        // schema which lists it as a default-valued property).
        vec![("timestamp_level", self.timestamp_level.as_str())]
    }
}

/// Translate a voxora [`TranscribeOptions`] into the
/// [`MiniMaxParams`] that the engine will send to MiniMax.
///
/// Returns:
///
/// - [`AsrError::InvalidInput`] if `opts.language` is `Some(_)`
///   but not in MiniMax's 20-tag whitelist
///   ([`crate::validate_lang_bcp47`]).
/// - [`AsrError::Unsupported`] if `opts.translate` is `true`
///   (MiniMax has no translation mode).
pub fn apply(opts: &TranscribeOptions) -> Result<MiniMaxParams, AsrError> {
    crate::language::validate_lang_bcp47(opts.language.as_deref())?;

    if opts.translate {
        return Err(AsrError::Unsupported(
            "MiniMax does not support translation to English",
        ));
    }

    let timestamp_level = if opts.timestamps {
        TimestampLevel::Word
    } else {
        TimestampLevel::Sentence
    };

    Ok(MiniMaxParams {
        language: opts.language.clone(),
        timestamp_level,
    })
}

/// Convert a MiniMax [`AsrResp`](crate::client::AsrResp) into a
/// voxora [`TranscriptionResult`].
///
/// `opts` is the original caller request — used to surface what
/// the caller asked for when the result is auto-detected and
/// upstream doesn't echo a forced-language sentinel (it doesn't,
/// so this just forwards whatever upstream reports).
pub fn collect_result(
    resp: crate::client::AsrResp,
    opts: &TranscribeOptions,
) -> voxora_traits::TranscriptionResult {
    use voxora_traits::{TranscriptionResult, TranscriptionSegment};

    // Convert seconds → samples at 16 kHz (the engine's documented
    // sample rate). MiniMax does not report sample rate; the
    // engine contract is documented as "16 kHz mono" in the
    // sample-rate contract on the crate root.
    const ASSUMED_SAMPLE_RATE: u64 = 16_000;

    let segments = resp
        .segments
        .into_iter()
        .map(|seg| {
            let start = (seg.start.max(0.0) * ASSUMED_SAMPLE_RATE as f64) as u64;
            let end = (seg.end.max(0.0) * ASSUMED_SAMPLE_RATE as f64) as u64;
            // Speaker labels are folded into the segment text for
            // now; surfacing the `speaker` field requires a new
            // `TranscriptionSegment` field which is out of scope
            // for this engine (EPIC #153 § "Scope (out)").
            let text = match &seg.speaker {
                Some(label) => format!("[{label}] {}", seg.text),
                None => seg.text,
            };
            TranscriptionSegment::new(start, end, text)
        })
        .collect();

    let language = opts.language.clone();
    TranscriptionResult::with_segments(resp.text, language, segments)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::AsrResp;

    #[test]
    fn apply_defaults_to_auto_detect() {
        let opts = TranscribeOptions::default();
        let p = apply(&opts).expect("default opts should build");
        assert!(p.language.is_none());
        assert_eq!(p.timestamp_level, TimestampLevel::Sentence);
        assert!(p.language_header().is_none());
    }

    #[test]
    fn apply_with_language_keeps_it() {
        let opts = TranscribeOptions::new(Some("en".into()), false, false);
        let p = apply(&opts).expect("en is valid");
        assert_eq!(p.language.as_deref(), Some("en"));
        assert_eq!(p.language_header(), Some("en"));
    }

    #[test]
    fn apply_rejects_unknown_language() {
        let opts = TranscribeOptions::new(Some("klingon".into()), false, false);
        let err = apply(&opts).expect_err("klingon should be rejected");
        match err {
            AsrError::InvalidInput(msg) => {
                assert!(msg.contains("klingon"), "{msg}");
            }
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn apply_rejects_translate() {
        let opts = TranscribeOptions::new(Some("en".into()), true, false);
        let err = apply(&opts).expect_err("translate should be unsupported");
        assert!(matches!(err, AsrError::Unsupported(_)));
    }

    #[test]
    fn apply_with_timestamps_picks_word_level() {
        let opts = TranscribeOptions::new(Some("en".into()), false, true);
        let p = apply(&opts).expect("en is valid");
        assert_eq!(p.timestamp_level, TimestampLevel::Word);
        let fields = p.multipart_fields();
        assert!(
            fields
                .iter()
                .any(|(k, v)| *k == "timestamp_level" && *v == "word")
        );
    }

    #[test]
    fn apply_without_timestamps_picks_sentence_level() {
        let opts = TranscribeOptions::new(Some("en".into()), false, false);
        let p = apply(&opts).expect("en is valid");
        assert_eq!(p.timestamp_level, TimestampLevel::Sentence);
        let fields = p.multipart_fields();
        assert!(
            fields
                .iter()
                .any(|(k, v)| *k == "timestamp_level" && v.is_empty())
        );
    }

    #[test]
    fn language_header_trims_whitespace() {
        let p = MiniMaxParams {
            language: Some("   ".to_string()),
            timestamp_level: TimestampLevel::Sentence,
        };
        assert!(
            p.language_header().is_none(),
            "whitespace must collapse to None"
        );
    }

    #[test]
    fn collect_result_folds_speaker_into_text() {
        let resp = AsrResp {
            text: "Hello".into(),
            duration: Some(1.0),
            n_speakers: Some(1),
            segments: vec![crate::client::AsrSegment {
                id: 0,
                start: 0.0,
                end: 1.0,
                speaker: Some("S1".into()),
                text: "Hello".into(),
            }],
            trace_id: None,
        };
        let opts = TranscribeOptions::default();
        let result = collect_result(resp, &opts);
        assert_eq!(result.text, "Hello");
        assert_eq!(result.segments.len(), 1);
        assert_eq!(result.segments[0].text, "[S1] Hello");
    }

    #[test]
    fn collect_result_converts_seconds_to_samples_at_16khz() {
        let resp = AsrResp {
            text: "x".into(),
            duration: Some(1.0),
            n_speakers: None,
            segments: vec![crate::client::AsrSegment {
                id: 0,
                start: 0.5,
                end: 1.5,
                speaker: None,
                text: "x".into(),
            }],
            trace_id: None,
        };
        let opts = TranscribeOptions::default();
        let result = collect_result(resp, &opts);
        assert_eq!(result.segments[0].start_sample, 8000); // 0.5 * 16000
        assert_eq!(result.segments[0].end_sample, 24000); // 1.5 * 16000
    }

    #[test]
    fn collect_result_echoes_caller_language() {
        let resp = AsrResp {
            text: "Hello".into(),
            duration: None,
            n_speakers: None,
            segments: vec![],
            trace_id: None,
        };
        let opts = TranscribeOptions::new(Some("en".into()), false, false);
        let result = collect_result(resp, &opts);
        assert_eq!(result.language.as_deref(), Some("en"));
    }
}
