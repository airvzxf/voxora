//! Energy-based reference implementation of [`VadSegmenter`].
//!
//! The detector slides a window over the input stream, computes
//! the root-mean-square (RMS) energy in each frame, and emits a
//! [`VadSegment`] each time the speech/silence state machine
//! transitions past the configured debounce windows. It is
//! CPU-only, deterministic, has zero non-workspace dependencies,
//! and is intended as a baseline when an ML-based VAD (Silero
//! via onnxruntime, etc.) is overkill.
//!
//! Algorithm:
//!
//! 1. Fill a deque with the most recent `frame_size_samples`
//!    samples.
//! 2. Compute `RMS = sqrt(sum(x²) / N)`. A frame is "speech" when
//!    `RMS >= rms_threshold`.
//! 3. Advance the window by `hop_size_samples`.
//! 4. Compare the new frame's classification against the
//!    detector's current state. While the classification is the
//!    same, do nothing. When it flips, start a debounce counter
//!    and only flip the state when the opposite classification
//!    has held continuously for at least `min_speech_ms` (for
//!    the silence→speech edge) or `min_silence_ms` (for the
//!    speech→silence edge).
//!
//! The first frame after construction or [`reset`](VadSegmenter::reset)
//! seeds the initial state (silence if the frame's RMS is below
//! the threshold, speech otherwise) but does not emit a segment.
//! Subsequent flips always emit a segment that closes the run
//! that just ended:
//!
//! - `Silence → Speech` at sample `T` emits
//!   `VadSegment { start_sample: prev_run_start, end_sample: T, is_speech: false }`.
//! - `Speech → Silence` at sample `T` emits
//!   `VadSegment { start_sample: prev_run_start, end_sample: T, is_speech: true }`.
//!
//! The trailing run is not auto-closed by [`next_segment`](VadSegmenter::next_segment);
//! call [`flush`](VadSegmenter::flush) to retrieve it.

use std::collections::VecDeque;

use crate::error::VadConfigError;
use crate::traits::{VadSegment, VadSegmenter};

/// Tuning knobs for [`EnergyVad`].
///
/// Defaults are tuned for 16 kHz mono f32 audio (the voxora
/// convention): 30 ms frames with a 10 ms hop (the standard
/// WebRTC VAD frame size), an RMS threshold of `0.01` (suitable
/// for normalized `[-1.0, 1.0]` PCM), and 250 ms / 100 ms
/// speech/silence debounce windows.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct EnergyVadConfig {
    /// Sample rate of the audio stream, in Hz. Used to convert
    /// the millisecond debounce knobs to sample counts.
    pub sample_rate_hz: u32,

    /// Window size in samples over which RMS is computed per
    /// frame. Must be greater than `hop_size_samples`.
    pub frame_size_samples: u32,

    /// Hop size in samples — how far the window advances per
    /// step. Smaller hops overlap more, giving finer time
    /// resolution at the cost of more frames per second.
    pub hop_size_samples: u32,

    /// RMS threshold above which a frame is classified as
    /// speech. RMS is computed in the normalized `[-1.0, 1.0]`
    /// PCM range, so `0.0` is silence and `0.5 / sqrt(2) ≈ 0.354`
    /// is the RMS of a full-scale sine wave at amplitude `0.5`.
    pub rms_threshold: f32,

    /// Minimum duration of consecutive speech frames required to
    /// flip from silence to speech, in milliseconds.
    pub min_speech_ms: u32,

    /// Minimum duration of consecutive silence frames required to
    /// flip from speech to silence, in milliseconds.
    pub min_silence_ms: u32,
}

impl Default for EnergyVadConfig {
    fn default() -> Self {
        Self {
            sample_rate_hz: 16_000,
            frame_size_samples: 480,
            hop_size_samples: 160,
            rms_threshold: 0.01,
            min_speech_ms: 250,
            min_silence_ms: 100,
        }
    }
}

impl EnergyVadConfig {
    /// Validate the config and return the first violation, if any.
    pub fn validate(&self) -> Result<(), VadConfigError> {
        if self.frame_size_samples == 0 {
            return Err(VadConfigError::ZeroFrameSize { got: 0 });
        }
        if self.hop_size_samples == 0 {
            return Err(VadConfigError::ZeroHopSize { got: 0 });
        }
        if self.hop_size_samples > self.frame_size_samples {
            return Err(VadConfigError::HopExceedsFrame {
                hop: self.hop_size_samples,
                frame: self.frame_size_samples,
            });
        }
        if !self.rms_threshold.is_finite() || self.rms_threshold < 0.0 {
            return Err(VadConfigError::InvalidThreshold {
                got: self.rms_threshold,
            });
        }
        if self.sample_rate_hz == 0 {
            return Err(VadConfigError::ZeroSampleRate { got: 0 });
        }
        Ok(())
    }

    /// Convert `min_speech_ms` into the minimum number of samples
    /// a candidate speech run must persist before the state flips.
    pub(crate) fn min_speech_samples(&self) -> u64 {
        self.min_speech_ms as u64 * self.sample_rate_hz as u64 / 1000
    }

    /// Convert `min_silence_ms` into the minimum number of samples
    /// a candidate silence run must persist before the state flips.
    pub(crate) fn min_silence_samples(&self) -> u64 {
        self.min_silence_ms as u64 * self.sample_rate_hz as u64 / 1000
    }
}

/// Builder for [`EnergyVad`] with a fluent API.
///
/// Mirrors the pattern in `voxora-hf::HuggingFaceSource::builder`
/// (Fase 2) so future voxora-* crates converge on the same shape.
#[derive(Debug, Clone, Default)]
#[must_use = "builders do nothing until .build() is called"]
pub struct EnergyVadBuilder {
    config: EnergyVadConfig,
}

impl EnergyVadBuilder {
    /// Override [`EnergyVadConfig::sample_rate_hz`].
    pub fn sample_rate_hz(mut self, hz: u32) -> Self {
        self.config.sample_rate_hz = hz;
        self
    }

    /// Override [`EnergyVadConfig::frame_size_samples`].
    pub fn frame_size_samples(mut self, samples: u32) -> Self {
        self.config.frame_size_samples = samples;
        self
    }

    /// Override [`EnergyVadConfig::hop_size_samples`].
    pub fn hop_size_samples(mut self, samples: u32) -> Self {
        self.config.hop_size_samples = samples;
        self
    }

    /// Override [`EnergyVadConfig::rms_threshold`].
    pub fn rms_threshold(mut self, threshold: f32) -> Self {
        self.config.rms_threshold = threshold;
        self
    }

    /// Override [`EnergyVadConfig::min_speech_ms`].
    pub fn min_speech_ms(mut self, ms: u32) -> Self {
        self.config.min_speech_ms = ms;
        self
    }

    /// Override [`EnergyVadConfig::min_silence_ms`].
    pub fn min_silence_ms(mut self, ms: u32) -> Self {
        self.config.min_silence_ms = ms;
        self
    }

    /// Build the [`EnergyVad`]. Returns the same [`VadConfigError`]
    /// as [`EnergyVad::with_config`] if the config is invalid.
    pub fn build(self) -> Result<EnergyVad, VadConfigError> {
        EnergyVad::with_config(self.config)
    }
}

/// Energy-based voice activity detector.
pub struct EnergyVad {
    config: EnergyVadConfig,
    frame: VecDeque<f32>,
    consumed: u64,
    total_fed: u64,
    state: VadState,
    state_seeded: bool,
    run_start: u64,
    cand_state: Option<VadState>,
    cand_start: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VadState {
    Silence,
    Speech,
}

impl EnergyVad {
    /// Create a new detector with [`EnergyVadConfig::default`].
    pub fn new() -> Self {
        Self::with_config(EnergyVadConfig::default())
            .expect("default config is validated by construction")
    }

    /// Create a new detector with the given config. Returns a
    /// [`VadConfigError`] if any knob is out of range — see
    /// [`EnergyVadConfig::validate`] for the rules.
    pub fn with_config(config: EnergyVadConfig) -> Result<Self, VadConfigError> {
        config.validate()?;
        let frame_capacity = config.frame_size_samples as usize;
        Ok(Self {
            config,
            frame: VecDeque::with_capacity(frame_capacity),
            consumed: 0,
            total_fed: 0,
            state: VadState::Silence,
            state_seeded: false,
            run_start: 0,
            cand_state: None,
            cand_start: None,
        })
    }

    /// Fluent builder. Equivalent to
    /// `EnergyVadBuilder::default().build()` but reads better at
    /// the call site.
    pub fn builder() -> EnergyVadBuilder {
        EnergyVadBuilder::default()
    }

    /// Read-only access to the active config.
    pub fn config(&self) -> &EnergyVadConfig {
        &self.config
    }

    /// Total samples fed to [`next_segment`](VadSegmenter::next_segment)
    /// since the last [`reset`](VadSegmenter::reset) (or
    /// construction).
    pub fn total_fed(&self) -> u64 {
        self.total_fed
    }
}

impl Default for EnergyVad {
    fn default() -> Self {
        Self::new()
    }
}

impl VadSegmenter for EnergyVad {
    fn next_segment(&mut self, samples: &[f32]) -> Option<VadSegment> {
        let mut last_emit: Option<VadSegment> = None;
        let frame_size = self.config.frame_size_samples as usize;
        let hop_size = self.config.hop_size_samples as usize;
        let threshold = self.config.rms_threshold;

        for &s in samples {
            self.frame.push_back(s);
            self.total_fed += 1;

            if self.frame.len() < frame_size {
                continue;
            }

            // We have a full window. Compute RMS over the most
            // recent `frame_size` samples.
            let rms = compute_rms(&self.frame);
            let is_speech = rms >= threshold;
            let target = if is_speech {
                VadState::Speech
            } else {
                VadState::Silence
            };

            let frame_classification_end = self.consumed + self.config.frame_size_samples as u64;

            // Seed the initial state on the very first frame so
            // we don't emit a zero-length "silence run from 0 to
            // 0" segment at construction time.
            if !self.state_seeded {
                self.state = target;
                self.run_start = 0;
                self.state_seeded = true;
            } else {
                last_emit = self.step(target, frame_classification_end, last_emit);
            }

            // Slide the window forward by `hop_size_samples`.
            for _ in 0..hop_size {
                self.frame.pop_front();
            }
            self.consumed += self.config.hop_size_samples as u64;
        }

        last_emit
    }

    fn reset(&mut self) {
        self.frame.clear();
        self.consumed = 0;
        self.total_fed = 0;
        self.state = VadState::Silence;
        self.state_seeded = false;
        self.run_start = 0;
        self.cand_state = None;
        self.cand_start = None;
    }

    fn flush(&mut self) -> Option<VadSegment> {
        // Only emit a trailing segment if the detector is
        // currently inside a speech run that the caller never
        // closed by transitioning back to silence. A trailing
        // silence run is not emitted (the caller already knows
        // the stream ended on silence).
        if !self.state_seeded {
            return None;
        }
        if self.state == VadState::Speech {
            let seg = VadSegment::new(self.run_start, self.total_fed, true);
            // Move into a silent post-state so a subsequent
            // `reset` does not see a stale open run.
            self.run_start = self.total_fed;
            self.cand_state = None;
            self.cand_start = None;
            return Some(seg);
        }
        None
    }
}

impl EnergyVad {
    fn step(
        &mut self,
        target: VadState,
        frame_classification_end: u64,
        mut emit: Option<VadSegment>,
    ) -> Option<VadSegment> {
        if target == self.state {
            // Same state as current — the candidate transition
            // (if any) is cancelled.
            self.cand_state = None;
            self.cand_start = None;
            return emit;
        }

        // Different state — start a candidate transition if not
        // already pending.
        let cand_start = match (self.cand_state, self.cand_start) {
            (Some(c), Some(s)) if c == target => s,
            _ => {
                self.cand_state = Some(target);
                self.cand_start = Some(frame_classification_end);
                frame_classification_end
            }
        };

        let min_samples = match target {
            VadState::Speech => self.config.min_speech_samples(),
            VadState::Silence => self.config.min_silence_samples(),
        };

        let cand_len = frame_classification_end.saturating_sub(cand_start);
        if cand_len < min_samples {
            return emit;
        }

        // Debounce satisfied — flip the state and emit the run
        // that just closed.
        let prev_state = self.state;
        let prev_run_start = self.run_start;
        emit = Some(VadSegment::new(
            prev_run_start,
            frame_classification_end,
            prev_state == VadState::Speech,
        ));
        self.state = target;
        self.run_start = frame_classification_end;
        self.cand_state = None;
        self.cand_start = None;
        emit
    }
}

fn compute_rms(frame: &VecDeque<f32>) -> f32 {
    // Accumulate in f64 to dodge catastrophic cancellation
    // when summing many small `f32` squares. The deque is at
    // most a few thousand samples long, so the f64 widening
    // is cheap.
    let mut sum_sq = 0.0_f64;
    for &x in frame {
        let xf = x as f64;
        sum_sq += xf * xf;
    }
    let n = frame.len() as f64;
    (sum_sq / n).sqrt() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() <= eps
    }

    #[test]
    fn config_default_validates() {
        assert!(EnergyVadConfig::default().validate().is_ok());
    }

    #[test]
    fn config_rejects_zero_frame() {
        let cfg = EnergyVadConfig {
            frame_size_samples: 0,
            ..EnergyVadConfig::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(VadConfigError::ZeroFrameSize { .. })
        ));
    }

    #[test]
    fn config_rejects_hop_exceeding_frame() {
        let cfg = EnergyVadConfig {
            hop_size_samples: 500,
            frame_size_samples: 480,
            ..EnergyVadConfig::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(VadConfigError::HopExceedsFrame { .. })
        ));
    }

    #[test]
    fn config_rejects_negative_threshold() {
        let cfg = EnergyVadConfig {
            rms_threshold: -0.01,
            ..EnergyVadConfig::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(VadConfigError::InvalidThreshold { .. })
        ));
    }

    #[test]
    fn config_rejects_nan_threshold() {
        let cfg = EnergyVadConfig {
            rms_threshold: f32::NAN,
            ..EnergyVadConfig::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(VadConfigError::InvalidThreshold { .. })
        ));
    }

    #[test]
    fn rms_of_silence_is_zero() {
        let mut frame: VecDeque<f32> = VecDeque::with_capacity(8);
        for _ in 0..8 {
            frame.push_back(0.0);
        }
        assert_eq!(compute_rms(&frame), 0.0);
    }

    #[test]
    fn rms_of_constant_amplitude_is_amplitude() {
        let mut frame: VecDeque<f32> = VecDeque::with_capacity(8);
        for _ in 0..8 {
            frame.push_back(0.5);
        }
        assert!(approx_eq(compute_rms(&frame), 0.5, 1e-6));
    }

    #[test]
    fn rms_of_unit_sine_is_one_over_sqrt_two() {
        // RMS of `A * sin(2π ft)` is `|A| / sqrt(2)`. With A=1,
        // that's ≈ 0.7071.
        let mut frame: VecDeque<f32> = VecDeque::with_capacity(64);
        for i in 0..64 {
            let t = i as f32 / 64.0;
            frame.push_back((2.0 * std::f32::consts::PI * t).sin());
        }
        let expected = 1.0 / 2.0_f32.sqrt();
        assert!(approx_eq(compute_rms(&frame), expected, 1e-3));
    }

    #[test]
    fn silence_input_emits_no_segments() {
        let mut vad = EnergyVad::new();
        let seg = vad.next_segment(&vec![0.0_f32; 16_000]);
        assert!(seg.is_none());
        // No trailing flush either — the stream ended in silence.
        assert!(vad.flush().is_none());
    }

    #[test]
    fn pure_tone_emits_one_segment_after_flush() {
        // Generate 0.5 s of 440 Hz tone at amplitude 0.5. After
        // the 250 ms speech debounce, the detector flips from
        // silence to speech and seeds `run_start` at the
        // transition point. There is no further flip because
        // the tone is continuous, so the trailing speech run
        // is only released by `flush`.
        let mut tone = Vec::with_capacity(8000);
        for i in 0..8000 {
            let t = i as f32 / 16_000.0;
            tone.push(0.5 * (2.0 * std::f32::consts::PI * 440.0 * t).sin());
        }

        let mut vad = EnergyVad::new();
        let initial = vad.next_segment(&tone);
        let trailing = vad.flush();

        // The initial frame is speech (RMS of the tone is well
        // above the threshold), so no segment is emitted during
        // the chunk — the state machine stays in speech for the
        // entire input. `flush` then returns the trailing
        // segment that closes the run.
        assert!(
            initial.is_none(),
            "no transition happens inside a pure-tone chunk"
        );
        let trailing = trailing.expect("flush must emit the open speech run");
        assert_eq!(trailing.start_sample, 0);
        assert_eq!(trailing.end_sample, 8000);
        assert!(trailing.is_speech);
    }

    #[test]
    fn silence_speech_silence_emits_two_segments() {
        // 0.5 s of silence, 0.5 s of 440 Hz tone, 0.5 s of
        // silence. We expect: silence run from 0 to T1 (flip to
        // speech), then speech run from T1 to T2 (flip back to
        // silence), then a trailing silence run that flush
        // does not emit.
        let sr = 16_000_u64;
        let chunk = sr as usize; // 1 s per chunk
        let silence_a = vec![0.0_f32; chunk / 2];
        let mut tone = Vec::with_capacity(chunk / 2);
        for i in 0..(chunk / 2) {
            let t = i as f32 / sr as f32;
            tone.push(0.5 * (2.0 * std::f32::consts::PI * 440.0 * t).sin());
        }
        let silence_b = vec![0.0_f32; chunk / 2];

        let mut vad = EnergyVad::new();
        let mut emitted = Vec::new();
        for chunk in [&silence_a[..], &tone[..], &silence_b[..]] {
            // Each chunk emits at most one segment per call
            // (the last flip the state machine observed); drain
            // by feeding the same chunk again.
            let first = vad.next_segment(chunk);
            if let Some(seg) = first {
                emitted.push(seg);
            }
        }

        assert_eq!(
            emitted.len(),
            2,
            "expected silence→speech + speech→silence segments, got {emitted:?}"
        );

        let silence_seg = &emitted[0];
        assert!(!silence_seg.is_speech);
        assert_eq!(silence_seg.start_sample, 0);
        assert!(silence_seg.end_sample > 0);

        let speech_seg = &emitted[1];
        assert!(speech_seg.is_speech);
        assert_eq!(speech_seg.start_sample, silence_seg.end_sample);
        assert!(speech_seg.end_sample > speech_seg.start_sample);
    }

    #[test]
    fn reset_clears_state() {
        let mut vad = EnergyVad::new();
        // Feed some tone so state becomes "speech".
        let mut tone = Vec::with_capacity(8000);
        for i in 0..8000 {
            let t = i as f32 / 16_000.0;
            tone.push(0.5 * (2.0 * std::f32::consts::PI * 440.0 * t).sin());
        }
        vad.next_segment(&tone);
        assert!(vad.total_fed() > 0);

        vad.reset();
        assert_eq!(vad.total_fed(), 0);

        // After reset, silence produces nothing — state machine
        // is back to the seeded-Silence starting position.
        let seg = vad.next_segment(&vec![0.0_f32; 16_000]);
        assert!(seg.is_none());
    }

    #[test]
    fn builder_succeeds_with_valid_knobs() {
        let vad = EnergyVad::builder()
            .rms_threshold(0.05)
            .min_speech_ms(100)
            .min_silence_ms(50)
            .sample_rate_hz(8_000)
            .frame_size_samples(80)
            .hop_size_samples(40)
            .build();
        assert!(vad.is_ok());
    }

    #[test]
    fn builder_propagates_validation_errors() {
        let res = EnergyVad::builder().frame_size_samples(0).build();
        assert!(matches!(res, Err(VadConfigError::ZeroFrameSize { .. })));
    }
}
