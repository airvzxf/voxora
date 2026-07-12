//! The [`AsrEngine`] trait and the value types that travel through it.
//!
//! An [`AsrEngine`] wraps a single ASR backend (whisper.cpp, qwen3-asr,
//! future engines) behind a synchronous, `Send + Sync` interface so any
//! caller can hold an `Arc<dyn AsrEngine>` and swap implementations
//! without touching inference code.

use crate::error::AsrError;

/// Reported capabilities of a model.
///
/// Engines that do not know their own capabilities should leave every
/// field at its conservative default (see [`ModelCapabilities::UNKNOWN`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ModelCapabilities {
    /// Whether the model supports multilingual ASR (as opposed to
    /// English-only).
    pub multilingual: bool,

    /// Whether the model can emit word-level timestamps.
    pub word_timestamps: bool,

    /// Whether the model supports streaming (incremental) inference.
    pub streaming: bool,

    /// ISO 639-1 language codes the model was trained on. Empty means
    /// "unknown".
    pub languages: Vec<String>,
}

impl ModelCapabilities {
    /// Sentinel value meaning "capabilities unknown / not advertised".
    ///
    /// Returned by the default [`AsrEngine::capabilities`] so implementors
    /// only override what they actually know.
    pub const UNKNOWN: Self = Self {
        multilingual: false,
        word_timestamps: false,
        streaming: false,
        languages: Vec::new(),
    };

    /// Construct a `ModelCapabilities` from all four fields.
    ///
    /// `ModelCapabilities` is `#[non_exhaustive]`, so downstream crates
    /// cannot use a struct expression. This constructor is the canonical
    /// way to build one with known values.
    pub const fn new(
        multilingual: bool,
        word_timestamps: bool,
        streaming: bool,
        languages: Vec<String>,
    ) -> Self {
        Self {
            multilingual,
            word_timestamps,
            streaming,
            languages,
        }
    }
}

impl std::fmt::Display for ModelCapabilities {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "multilingual={}, word_timestamps={}, streaming={}, languages={}",
            self.multilingual,
            self.word_timestamps,
            self.streaming,
            self.languages.len()
        )
    }
}

/// Options that affect a single transcription pass but not model selection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct TranscribeOptions {
    /// ISO 639-1 code (e.g. `"en"`, `"es"`) or `None` to let the engine
    /// auto-detect.
    pub language: Option<String>,

    /// Whether to translate the output to English (only meaningful for
    /// multilingual models; ignored otherwise).
    pub translate: bool,

    /// Whether to include per-segment timestamps in
    /// [`TranscriptionResult::segments`].
    pub timestamps: bool,
}

/// One segment of a transcription.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TranscriptionSegment {
    /// Inclusive segment start, in samples at the engine's expected
    /// sample rate (typically 16 kHz).
    pub start_sample: u64,

    /// Exclusive segment end, in samples at the engine's expected
    /// sample rate.
    pub end_sample: u64,

    /// Transcribed text for this segment.
    pub text: String,
}

/// The full output of a transcription pass.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct TranscriptionResult {
    /// Full transcribed text, segments joined with a single space.
    pub text: String,

    /// Language that was forced or detected, if known.
    pub language: Option<String>,

    /// Per-segment breakdown. Empty when
    /// [`TranscribeOptions::timestamps`] was `false`.
    pub segments: Vec<TranscriptionSegment>,
}

impl TranscriptionResult {
    /// Construct an empty result with the given full text.
    ///
    /// Provided because [`TranscriptionResult`] is `#[non_exhaustive]`
    /// and so cannot be built with a struct expression outside of this
    /// crate; this constructor gives engines a one-line way to return a
    /// minimal result.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            language: None,
            segments: Vec::new(),
        }
    }
}

impl std::fmt::Display for TranscriptionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

/// An automatic-speech-recognition engine.
///
/// Inference is exposed synchronously because it is a CPU/GPU-bound
/// task; wrapping it in `async` would add cost without benefit. Model
/// acquisition (downloads, cache lookups) lives in [`crate::ModelSource`]
/// instead.
pub trait AsrEngine: Send + Sync {
    /// What this engine supports. Defaults to [`ModelCapabilities::UNKNOWN`]
    /// so implementors only override what they know.
    fn capabilities(&self) -> ModelCapabilities {
        ModelCapabilities::UNKNOWN
    }

    /// Transcribe a buffer of mono PCM samples at the engine's expected
    /// sample rate (typically 16 kHz, f32 in `[-1.0, 1.0]`).
    fn transcribe(
        &self,
        samples: &[f32],
        opts: &TranscribeOptions,
    ) -> Result<TranscriptionResult, AsrError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal `AsrEngine` used to verify trait-object construction.
    struct EchoEngine;

    impl AsrEngine for EchoEngine {
        fn capabilities(&self) -> ModelCapabilities {
            ModelCapabilities {
                multilingual: true,
                languages: vec!["en".into(), "es".into()],
                ..ModelCapabilities::UNKNOWN
            }
        }

        fn transcribe(
            &self,
            samples: &[f32],
            opts: &TranscribeOptions,
        ) -> Result<TranscriptionResult, AsrError> {
            Ok(TranscriptionResult {
                text: format!("echo of {} samples", samples.len()),
                language: opts.language.clone(),
                segments: if opts.timestamps && !samples.is_empty() {
                    vec![TranscriptionSegment {
                        start_sample: 0,
                        end_sample: samples.len() as u64,
                        text: format!("echo of {} samples", samples.len()),
                    }]
                } else {
                    Vec::new()
                },
            })
        }
    }

    #[test]
    fn transcribe_options_default_is_empty() {
        let opts = TranscribeOptions::default();
        assert!(opts.language.is_none());
        assert!(!opts.translate);
        assert!(!opts.timestamps);
    }

    #[test]
    fn unknown_capabilities_sentinel_is_distinct_from_default() {
        // UNKNOWN and Default should both be conservative but they are
        // reached by different paths — UNKNOWN is what engines get when
        // they skip overriding capabilities(); Default::default() is for
        // ad-hoc construction. They must compare equal today but the
        // contract allows UNKNOWN to diverge later.
        assert_eq!(ModelCapabilities::UNKNOWN, ModelCapabilities::default());
    }

    #[test]
    fn default_capabilities_impl_returns_unknown() {
        struct Silent;
        impl AsrEngine for Silent {
            fn transcribe(
                &self,
                _: &[f32],
                _: &TranscribeOptions,
            ) -> Result<TranscriptionResult, AsrError> {
                unreachable!("test only calls capabilities()")
            }
        }
        assert_eq!(Silent.capabilities(), ModelCapabilities::UNKNOWN);
    }

    #[test]
    fn engine_works_behind_arc_dyn() {
        let engine: std::sync::Arc<dyn AsrEngine> = std::sync::Arc::new(EchoEngine);
        let caps = engine.capabilities();
        assert!(caps.multilingual);
        assert_eq!(caps.languages, vec!["en".to_string(), "es".to_string()]);

        let result = engine
            .transcribe(&[0.0_f32; 16], &TranscribeOptions::default())
            .expect("transcribe should succeed");
        assert_eq!(result.text, "echo of 16 samples");
        assert!(result.language.is_none());
        assert!(result.segments.is_empty(), "timestamps disabled");
    }

    #[test]
    fn engine_emits_segments_when_timestamps_requested() {
        let engine = EchoEngine;
        let opts = TranscribeOptions {
            timestamps: true,
            ..TranscribeOptions::default()
        };
        let result = engine
            .transcribe(&[0.0_f32; 4], &opts)
            .expect("transcribe should succeed");
        assert_eq!(result.segments.len(), 1);
        assert_eq!(result.segments[0].start_sample, 0);
        assert_eq!(result.segments[0].end_sample, 4);
    }

    #[test]
    fn engine_works_across_threads() {
        use std::sync::Arc;
        use std::thread;

        let engine: Arc<dyn AsrEngine> = Arc::new(EchoEngine);
        let handles: Vec<_> = (0..4)
            .map(|i| {
                let e = Arc::clone(&engine);
                thread::spawn(move || {
                    e.transcribe(&vec![0.0_f32; i], &TranscribeOptions::default())
                        .map(|r| r.text)
                })
            })
            .collect();

        for (i, h) in handles.into_iter().enumerate() {
            let text = h.join().expect("thread did not panic").expect("ok");
            assert_eq!(text, format!("echo of {i} samples"));
        }
    }

    #[test]
    fn transcription_result_display_returns_text() {
        let r = TranscriptionResult::new("hello world");
        assert_eq!(format!("{r}"), "hello world");
        assert_eq!(r.to_string(), "hello world");
    }

    #[test]
    fn transcription_result_new_accepts_string_and_str() {
        let from_str: TranscriptionResult = TranscriptionResult::new("from &str");
        let from_string: TranscriptionResult =
            TranscriptionResult::new(String::from("from String"));
        assert_eq!(format!("{from_str}"), "from &str");
        assert_eq!(format!("{from_string}"), "from String");
    }

    #[test]
    fn model_capabilities_display_summarises_fields() {
        let caps = ModelCapabilities {
            multilingual: true,
            word_timestamps: true,
            streaming: false,
            languages: vec!["en".into(), "es".into(), "fr".into()],
        };
        assert_eq!(
            format!("{caps}"),
            "multilingual=true, word_timestamps=true, streaming=false, languages=3"
        );
    }

    #[test]
    fn model_capabilities_unknown_display_is_conservative() {
        let s = format!("{}", ModelCapabilities::UNKNOWN);
        assert!(s.contains("multilingual=false"), "{s}");
        assert!(s.contains("languages=0"), "{s}");
    }

    #[test]
    fn transcribe_options_implements_eq() {
        let a = TranscribeOptions {
            language: Some("en".into()),
            translate: true,
            timestamps: false,
        };
        let b = TranscribeOptions {
            language: Some("en".into()),
            translate: true,
            timestamps: false,
        };
        let c = TranscribeOptions {
            language: Some("es".into()),
            translate: true,
            timestamps: false,
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
