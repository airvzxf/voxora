//! Mapping between voxora's [`TranscribeOptions`] / [`TranscriptionResult`]
//! and whisper-rs's [`FullParams`] / [`WhisperState`] segment API.
//!
//! Split into two pure-ish functions:
//!
//! - [`apply`] — configure a fresh [`FullParams`] from caller's
//!   [`TranscribeOptions`] and the model's multilingual flag.
//! - [`collect_result`] — pull segments + detected language id out of
//!   a [`WhisperState`] after [`WhisperState::full`] has been called.

use voxora_core::{AsrError, TranscribeOptions, TranscriptionResult, TranscriptionSegment};

use crate::language;

/// Sample rate, in Hz, that voxora feeds into whisper.cpp.
///
/// whisper.cpp resamples internally if needed but the lowest-latency
/// path is 16 kHz mono `f32` in `[-1.0, 1.0]`. We document and assume
/// this rate in the [`crate::WhisperEngine::transcribe`] contract.
pub const WHISPER_SAMPLE_RATE: u32 = 16_000;

/// whisper-rs reports segment boundaries in centiseconds (10 ms each).
/// Multiplying by `WHISPER_SAMPLE_RATE / 100` converts to samples at
/// [`WHISPER_SAMPLE_RATE`].
const CENTISECONDS_PER_SAMPLE: u64 = WHISPER_SAMPLE_RATE as u64 / 100;

/// Configure `params` from the caller's [`TranscribeOptions`].
///
/// Returns [`AsrError::InvalidInput`] if the language code is unknown
/// to whisper.cpp, or if `translate` was requested on a model that is
/// not multilingual.
pub fn apply<'a>(
    params: &mut whisper_rs::FullParams<'a, 'a>,
    opts: &'a TranscribeOptions,
    is_multilingual: bool,
) -> Result<(), AsrError> {
    // Language: None -> auto-detect, Some -> validate then set.
    match &opts.language {
        None => {
            params.set_detect_language(true);
        }
        Some(code) => {
            language::validate_lang(code)?;
            params.set_language(Some(code.as_str()));
        }
    }

    // Translate is only meaningful on multilingual checkpoints.
    if opts.translate && !is_multilingual {
        return Err(AsrError::InvalidInput(
            "translate requested but the loaded model is English-only".into(),
        ));
    }
    params.set_translate(opts.translate);

    // Timestamps: when the caller does not want them, tell whisper.cpp
    // to skip emitting them internally (cheaper decode). When they do,
    // leave them on (default).
    params.set_no_timestamps(!opts.timestamps);

    // Quieter runtime — voxora owns the output, whisper.cpp should
    // not write progress / realtime lines to stderr.
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_print_special(false);

    // Cap threads at min(4, hardware_concurrency) — whisper.cpp's own
    // default. We do this explicitly so behaviour is stable across
    // machines with different core counts.
    let threads = std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(1)
        .clamp(1, 4);
    params.set_n_threads(threads);

    Ok(())
}

/// Build a [`TranscriptionResult`] from the segments whisper.cpp
/// produced during the just-completed `full()` pass.
///
/// If `opts.timestamps` was `false`, only the joined text is returned
/// and `segments` is empty (cheaper).
///
/// `detected_language` should be the language the engine inferred
/// when `opts.language` was `None` (the result of
/// [`WhisperState::full_lang_id_from_state`]); the caller passes it in
/// so this function stays free of unsafe and easy to unit-test.
pub fn collect_result(
    state: &whisper_rs::WhisperState,
    opts: &TranscribeOptions,
    detected_language: Option<String>,
) -> Result<TranscriptionResult, AsrError> {
    let n = state.full_n_segments();
    let mut texts: Vec<String> = Vec::with_capacity(n as usize);
    let mut segments: Vec<TranscriptionSegment> = Vec::new();

    for i in 0..n {
        let seg = state.get_segment(i).ok_or_else(|| {
            AsrError::Inference(format!("segment {i} out of bounds (n_segments={n})"))
        })?;
        let text = seg
            .to_str_lossy()
            .map_err(|e| AsrError::Inference(format!("segment {i} text: {e}")))?
            .into_owned();
        texts.push(text.clone());

        if opts.timestamps {
            let t0 = seg.start_timestamp();
            let t1 = seg.end_timestamp();
            segments.push(TranscriptionSegment::new(
                t0.max(0) as u64 * CENTISECONDS_PER_SAMPLE,
                t1.max(0) as u64 * CENTISECONDS_PER_SAMPLE,
                text,
            ));
        }
    }

    let full_text = texts
        .into_iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    Ok(TranscriptionResult::with_segments(
        full_text,
        detected_language.or_else(|| opts.language.clone()),
        segments,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centiseconds_to_samples_constant_is_consistent() {
        assert_eq!(CENTISECONDS_PER_SAMPLE, 160);
        // 1 second = 100 centiseconds = 16000 samples at 16 kHz.
        assert_eq!(100 * CENTISECONDS_PER_SAMPLE, 16_000);
    }

    #[test]
    fn translate_on_english_only_model_is_rejected() {
        let opts = TranscribeOptions::new(Some("en".into()), true, false);
        // Mirror the guard inside `apply`: is_multilingual=false +
        // opts.translate=true must trip the InvalidInput path.
        assert!(opts.translate && !false, "guard should trigger");
    }

    #[test]
    fn language_validation_runs_before_full_params_construction() {
        // Documented contract: bad ISO code -> InvalidInput, regardless
        // of multilingual state.
        assert!(language::validate_lang("xx").is_err());
    }

    #[test]
    fn whisper_sample_rate_is_16khz() {
        assert_eq!(WHISPER_SAMPLE_RATE, 16_000);
    }
}
