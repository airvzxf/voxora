//! The [`VadSegmenter`] trait and the [`VadSegment`] value type.
//!
//! A voice-activity detector (VAD) walks a buffer of PCM samples and
//! emits the boundaries of regions that contain speech versus
//! silence. The contract is intentionally narrow — one push per
//! chunk, one [`VadSegment`] out whenever the detector decides a
//! run of speech (or silence) has ended:
//!
//! ```text
//!                  ┌── chunk i ──┐ ┌── chunk i+1 ──┐
//!  caller feeds:   [pcm samples] │ [pcm samples] │ [pcm samples] ...
//!                  ▼             ▼ ▼              ▼ ▼
//!  detector emits:        Some(VadSegment)        Some(VadSegment)
//! ```
//!
//! ASR-specific by design: this is a building block for the voxora
//! speech-recognition stack (silence trimming, live-UI gating,
//! diarization pre-filter), not a generic signal-processing library.

/// One run of speech or silence inside a stream of PCM samples.
///
/// Constructed via [`VadSegment::new`] because the type is
/// `#[non_exhaustive]` — downstream crates cannot use a struct
/// expression.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct VadSegment {
    /// Inclusive segment start, in samples at the detector's
    /// expected sample rate (typically 16 kHz).
    ///
    /// Mirrors `voxora_traits::TranscriptionSegment::start_sample`
    /// so consumers can post-process VAD and ASR outputs without
    /// re-stamping timestamps against the original buffer.
    pub start_sample: u64,

    /// Exclusive segment end, in samples at the detector's
    /// expected sample rate.
    pub end_sample: u64,

    /// `true` if the segment contains speech, `false` if it is a
    /// silence gap.
    pub is_speech: bool,
}

impl VadSegment {
    /// Construct a [`VadSegment`] from its three fields.
    ///
    /// Provided because [`VadSegment`] is `#[non_exhaustive]` and
    /// so cannot be built with a struct expression from outside
    /// this crate; detectors use this constructor to emit one
    /// boundary per transition.
    pub fn new(start_sample: u64, end_sample: u64, is_speech: bool) -> Self {
        Self {
            start_sample,
            end_sample,
            is_speech,
        }
    }

    /// Length of the segment in samples.
    pub fn len(&self) -> u64 {
        self.end_sample.saturating_sub(self.start_sample)
    }

    /// `true` when [`VadSegment::len`] is `0`.
    pub fn is_empty(&self) -> bool {
        self.end_sample <= self.start_sample
    }
}

/// Voice-activity detector.
///
/// Implementors walk a streaming buffer of mono PCM samples at a
/// fixed sample rate (typically 16 kHz) and emit one [`VadSegment`]
/// each time the speech/silence state machine flips. The detector
/// keeps its own internal state across calls, so a single instance
/// can be polled repeatedly as more audio arrives.
///
/// Use [`reset`](Self::reset) to drop the internal state between
/// speakers, recordings, or other disjoint sources.
///
/// Future work: integration with
/// `voxora_traits::StreamingAsrEngine` (issues #50/#51) will
/// require a streaming-aware extension of this trait so the
/// segmenter can drive a live decoder. That extension is deferred
/// to Phase 8 alongside the streaming engine.
pub trait VadSegmenter: Send + Sync {
    /// Feed the next chunk of PCM samples.
    ///
    /// `samples` is a borrowed slice of mono f32 in `[-1.0, 1.0]`
    /// at the detector's expected sample rate. Returns a segment
    /// iff the call closed one — i.e. the detector just observed a
    /// speech→silence or silence→speech transition that survives
    /// the configured debounce windows. Returns `None` while the
    /// detector is still inside an open run.
    fn next_segment(&mut self, samples: &[f32]) -> Option<VadSegment>;

    /// Drop internal state and return to the "listening" position.
    ///
    /// After `reset`, the next call to [`next_segment`](Self::next_segment)
    /// behaves as if the detector had just been constructed. Use
    /// this between recordings, on speaker change, or before
    /// re-using a pooled detector.
    fn reset(&mut self);

    /// Release any trailing open run.
    ///
    /// Returns the speech segment that was still "open" at the
    /// last [`next_segment`](Self::next_segment) call (i.e. the
    /// detector had transitioned into the speech state but never
    /// back out of it). Returns `None` if there is no open run or
    /// if the run is a silence gap — those are not useful to a
    /// caller at end-of-stream because silence on its own does
    /// not delimit a region worth transcribing.
    ///
    /// The default implementation returns `None` for detectors
    /// that have no internal buffering; overriders should return
    /// the open segment and reset their internal run markers.
    fn flush(&mut self) -> Option<VadSegment> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_len_and_is_empty() {
        let s = VadSegment::new(0, 8000, true);
        assert_eq!(s.len(), 8000);
        assert!(!s.is_empty());

        let empty = VadSegment::new(4, 4, false);
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn segment_is_non_exhaustive_constructible() {
        // Smoke test: the `new` constructor is the canonical
        // way to build a segment because the struct is
        // `#[non_exhaustive]`.
        let s = VadSegment::new(10, 20, true);
        assert_eq!(s.start_sample, 10);
        assert_eq!(s.end_sample, 20);
        assert!(s.is_speech);
    }
}
