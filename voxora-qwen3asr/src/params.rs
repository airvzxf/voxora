//! Mapping between voxora's [`TranscribeOptions`] / [`TranscriptionResult`]
//! and `qwen3_asr`'s `TranscribeOptions` / `TranscribeResult`.
//!
//! Two pure-ish functions:
//!
//! - [`build_qwen_opts`] — translate a voxora [`TranscribeOptions`]
//!   into an upstream [`qwen3_asr::TranscribeOptions`], rejecting
//!   options upstream does not support
//!   (`translate`, unknown languages, …).
//! - [`collect_qwen_result`] — pull the upstream
//!   [`qwen3_asr::TranscribeResult`] into a voxora
//!   [`TranscriptionResult`], normalising the language field.

use voxora_core::{AsrError, TranscribeOptions, TranscriptionResult};

use crate::language;

/// Sample rate, in Hz, that voxora feeds into `qwen3-asr`.
///
/// `qwen3-asr` resamples internally but the lowest-latency path is
/// 16 kHz mono `f32` in `[-1.0, 1.0]`. We document and assume this
/// rate in the [`crate::QwenAsrEngine::transcribe`] contract.
#[allow(dead_code)] // public constant; consumed by callers and the rustdoc contract
pub const QWEN_SAMPLE_RATE: u32 = 16_000;

/// Build an upstream [`qwen3_asr::TranscribeOptions`] from the voxora
/// [`TranscribeOptions`] the caller supplied.
///
/// Returns:
///
/// - [`AsrError::InvalidInput`] if `opts.language` is `Some(_)` but not
///   in the closed Qwen3 list ([`crate::validate_lang`]).
/// - [`AsrError::Unsupported`] if `opts.translate` is `true` (upstream
///   has no translation mode).
///
/// `opts.timestamps` is silently accepted but ignored downstream —
/// Qwen3-ASR does not emit segment boundaries, so the
/// `TranscriptionResult::segments` list will always be empty.
pub fn build_qwen_opts(opts: &TranscribeOptions) -> Result<qwen3_asr::TranscribeOptions, AsrError> {
    let language = match &opts.language {
        Some(name) => {
            language::validate_lang(name)?;
            Some(name.to_ascii_lowercase())
        }
        None => None,
    };

    if opts.translate {
        return Err(AsrError::Unsupported("translate"));
    }

    // `qwen3_asr::TranscribeOptions` is `#[non_exhaustive]`, so we
    // construct it via the Default + mutate path (which it provides)
    // rather than a struct expression.
    let mut out = qwen3_asr::TranscribeOptions::default();
    out.language = language;
    Ok(out)
}

/// Pull the upstream [`qwen3_asr::TranscribeResult`] into a voxora
/// [`TranscriptionResult`].
///
/// `opts.language` (the original caller request) is needed to decide
/// what the [`TranscriptionResult::language`] field should report
/// when upstream marked the result as `"forced"`: in that case the
/// caller pinned the language, so we echo it back rather than the
/// literal sentinel `"forced"` upstream emits.
pub fn collect_qwen_result(
    qwen: qwen3_asr::TranscribeResult,
    opts: &TranscribeOptions,
) -> TranscriptionResult {
    let qwen_language = qwen.language;
    let qwen_text = qwen.text;
    build_transcription_result(&qwen_text, &qwen_language, opts)
}

/// Build a [`TranscriptionResult`] from the three raw fields upstream
/// reports. Split out from [`collect_qwen_result`] so the mapping
/// logic is testable without needing to construct a
/// `qwen3_asr::TranscribeResult` (which is `#[non_exhaustive]` and
/// has no public constructor).
fn build_transcription_result(
    text: &str,
    qwen_language: &str,
    opts: &TranscribeOptions,
) -> TranscriptionResult {
    let language = match qwen_language {
        // Upstream returns the literal "forced" when the caller
        // pinned `opts.language`. Surface what the caller asked for
        // instead of the sentinel, so the result is self-describing.
        "forced" => opts.language.clone().map(|s| s.to_ascii_lowercase()),
        other => Some(other.to_string()),
    };

    TranscriptionResult::with_segments(text.trim().to_string(), language, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qwen_sample_rate_is_16khz() {
        assert_eq!(QWEN_SAMPLE_RATE, 16_000);
    }

    #[test]
    fn build_qwen_opts_defaults_to_auto_detect() {
        let opts = TranscribeOptions::default();
        let qwen = build_qwen_opts(&opts).expect("default opts should build");
        assert!(qwen.language.is_none(), "None means auto-detect upstream");
        assert_eq!(qwen.max_new_tokens, 512, "upstream default is 512");
    }

    #[test]
    fn build_qwen_opts_normalises_language_case() {
        let opts = TranscribeOptions::new(Some("English".into()), false, false);
        let qwen = build_qwen_opts(&opts).expect("English should validate");
        assert_eq!(qwen.language.as_deref(), Some("english"));
    }

    #[test]
    fn build_qwen_opts_rejects_unknown_language() {
        let opts = TranscribeOptions::new(Some("klingon".into()), false, false);
        match build_qwen_opts(&opts) {
            Err(AsrError::InvalidInput(msg)) => {
                assert!(msg.contains("klingon"), "{msg}");
            }
            Err(other) => panic!("expected InvalidInput, got {other:?}"),
            Ok(_) => panic!("klingon should have errored"),
        }
    }

    #[test]
    fn build_qwen_opts_rejects_iso_639_1_codes() {
        let opts = TranscribeOptions::new(Some("en".into()), false, false);
        assert!(build_qwen_opts(&opts).is_err());
    }

    #[test]
    fn build_qwen_opts_rejects_translate() {
        let opts = TranscribeOptions::new(Some("english".into()), true, false);
        match build_qwen_opts(&opts) {
            Err(AsrError::Unsupported("translate")) => {}
            Err(other) => panic!("expected Unsupported(\"translate\"), got {other:?}"),
            Ok(_) => panic!("translate should have errored"),
        }
    }

    #[test]
    fn build_qwen_opts_accepts_timestamps_silently() {
        // Qwen3-ASR does not emit segment boundaries; we silently
        // accept the option and emit an empty `segments` list.
        let opts = TranscribeOptions::new(Some("english".into()), false, true);
        let qwen = build_qwen_opts(&opts).expect("timestamps should pass");
        assert_eq!(qwen.language.as_deref(), Some("english"));
    }

    #[test]
    fn build_transcription_result_with_forced_language_echoes_caller() {
        let opts = TranscribeOptions::new(Some("english".into()), false, false);
        let r = build_transcription_result("hello world", "forced", &opts);
        assert_eq!(r.text, "hello world");
        assert_eq!(r.language.as_deref(), Some("english"));
        assert!(r.segments.is_empty(), "timestamps not supported upstream");
    }

    #[test]
    fn build_transcription_result_with_detected_language_passes_through() {
        let opts = TranscribeOptions::default(); // no language forced
        let r = build_transcription_result("hola mundo", "spanish", &opts);
        assert_eq!(r.text, "hola mundo");
        assert_eq!(r.language.as_deref(), Some("spanish"));
        assert!(r.segments.is_empty());
    }

    #[test]
    fn build_transcription_result_trims_whitespace() {
        let opts = TranscribeOptions::default();
        let r = build_transcription_result("  hello world  ", "english", &opts);
        assert_eq!(r.text, "hello world");
    }

    #[test]
    fn build_transcription_result_normalises_forced_to_lowercase() {
        let opts = TranscribeOptions::new(Some("English".into()), false, false);
        let r = build_transcription_result("hello", "forced", &opts);
        assert_eq!(r.language.as_deref(), Some("english"));
    }

    #[test]
    fn build_transcription_result_with_empty_text_stays_empty() {
        let opts = TranscribeOptions::default();
        let r = build_transcription_result("", "english", &opts);
        assert_eq!(r.text, "");
        assert_eq!(r.language.as_deref(), Some("english"));
    }
}
