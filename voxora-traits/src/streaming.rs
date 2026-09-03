//! Streaming / incremental ASR extension trait.
//!
//! Most voxora engines today are whole-audio-in / whole-text-out
//! (see [`AsrEngine::transcribe`]). Some use cases — live
//! transcription, voice activity detection, real-time UIs — need
//! streaming: feed small audio chunks and get partial
//! transcriptions back as they become available.
//!
//! [`StreamingAsrEngine`] is the trait surface for that. It is
//! **additive for downstream callers**: an `Arc<dyn AsrEngine>`
//! keeps compiling. Engines opt in by implementing
//! `StreamingAsrEngine` in addition to `AsrEngine`.
//!
//! [`AsrEngine::transcribe`]: crate::engine::AsrEngine::transcribe

use async_trait::async_trait;

use crate::engine::{TranscribeOptions, TranscriptionResult};
use crate::error::AsrError;

/// Per-chunk options for [`StreamingAsrEngine::transcribe_chunk`].
///
/// Differs from [`TranscribeOptions`] in that timestamps are *always*
/// enabled (the consumer needs the partial-result location) and the
/// language cannot change mid-stream (set at
/// [`StreamingAsrEngine::begin_stream`] time).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct StreamingOptions {
    /// Initial language hint (ISO 639-1). Locked for the duration of
    /// the stream.
    pub language: Option<String>,
}

impl StreamingOptions {
    /// Construct a [`StreamingOptions`] from its single field.
    ///
    /// Provided because [`StreamingOptions`] is `#[non_exhaustive]`
    /// and so cannot be built with a struct expression outside this
    /// crate; engines and tests use this constructor to pick a
    /// non-default language.
    pub const fn new(language: Option<String>) -> Self {
        Self { language }
    }

    /// Convert these streaming options into a matching
    /// [`TranscribeOptions`] for the final [`TranscriptionResult`].
    ///
    /// Timestamps are forced on (a streaming consumer always needs
    /// segment locations to know what changed).
    pub fn as_transcribe_options(&self) -> TranscribeOptions {
        TranscribeOptions {
            language: self.language.clone(),
            translate: false,
            timestamps: true,
        }
    }
}

/// Output of a single [`StreamingAsrEngine::transcribe_chunk`] call.
///
/// Contains the partial transcript up to and including the latest
/// chunk, plus the segment boundaries that the engine identified
/// within the buffered audio.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct StreamingResult {
    /// Partial transcript (joined text).
    pub text: String,
    /// True iff the engine considers the buffered audio complete
    /// (e.g. silence detected).
    pub is_final: bool,
}

impl StreamingResult {
    /// Construct a non-final partial result with the given text.
    pub fn partial(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_final: false,
        }
    }

    /// Construct a final result with the given text.
    pub fn final_(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            is_final: true,
        }
    }
}

/// Streaming ASR engine — feeds audio incrementally.
///
/// The contract:
/// 1. Caller invokes [`begin_stream`](Self::begin_stream) once.
/// 2. Caller repeatedly invokes
///    [`transcribe_chunk`](StreamingSession::transcribe_chunk) with
///    successive audio buffers.
/// 3. Caller invokes
///    [`finalize_stream`](StreamingSession::finalize_stream) once at
///    end-of-stream and discards the engine.
///
/// Engines that don't support streaming do not implement this trait;
/// the [`AsrEngine::transcribe`] whole-buffer fallback remains
/// available.
#[async_trait]
pub trait StreamingAsrEngine: Send + Sync {
    /// Start a new streaming session. Returns an opaque handle the
    /// caller passes to subsequent calls.
    async fn begin_stream(
        &self,
        opts: &StreamingOptions,
    ) -> Result<Box<dyn StreamingSession>, AsrError>;
}

/// A single streaming session — typed via dyn-dispatch to avoid
/// leaking engine-specific state types.
///
/// Sessions are not required to be `Send`: they may hold
/// thread-local decoder state in whisper.cpp or candle. Callers must
/// drive a session from one thread at a time. The engine itself
/// remains `Send + Sync` (so multiple sessions can run concurrently
/// across threads).
#[async_trait]
pub trait StreamingSession {
    /// Feed an audio chunk and get the partial transcript back.
    async fn transcribe_chunk(&mut self, samples: &[f32]) -> Result<StreamingResult, AsrError>;

    /// Mark end-of-stream. Returns the final transcript.
    async fn finalize_stream(self: Box<Self>) -> Result<TranscriptionResult, AsrError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_options_default_is_empty() {
        let opts = StreamingOptions::default();
        assert!(opts.language.is_none());
    }

    #[test]
    fn streaming_result_default_is_empty() {
        let r = StreamingResult::default();
        assert_eq!(r.text, "");
        assert!(!r.is_final);
    }

    #[test]
    fn streaming_options_new_round_trips() {
        let opts = StreamingOptions::new(Some("en".into()));
        assert_eq!(opts.language.as_deref(), Some("en"));
    }

    #[test]
    fn streaming_options_as_transcribe_options_forces_timestamps() {
        let opts = StreamingOptions::new(Some("es".into()));
        let transcribe = opts.as_transcribe_options();
        assert_eq!(transcribe.language.as_deref(), Some("es"));
        assert!(transcribe.timestamps);
        assert!(!transcribe.translate);
    }

    #[test]
    fn streaming_result_constructors_set_is_final_correctly() {
        let partial = StreamingResult::partial("hello");
        assert_eq!(partial.text, "hello");
        assert!(!partial.is_final);

        let final_result = StreamingResult::final_("hello world");
        assert_eq!(final_result.text, "hello world");
        assert!(final_result.is_final);
    }
}
