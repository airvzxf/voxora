//! Errors returned by [`voxora_vad`](crate) constructors.
//!
//! The [`VadSegmenter`](crate::VadSegmenter) trait itself is
//! infallible (`next_segment` returns `Option<VadSegment>`), so
//! errors surface exclusively from the configuration paths
//! ([`EnergyVad::with_config`](crate::EnergyVad::with_config) and
//! the builders). Runtime audio processing never errors.

use thiserror::Error;

/// All errors that may occur while building an energy-based
/// voice-activity detector.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum VadConfigError {
    /// The `frame_size_samples` knob was set to `0`. A zero-length
    /// frame would divide by zero in the RMS calculation.
    #[error("frame_size_samples must be > 0 (got {got})")]
    ZeroFrameSize {
        /// The value that was rejected.
        got: u32,
    },

    /// The `hop_size_samples` knob was set to `0`. A zero-length
    /// hop would mean the detector never advances.
    #[error("hop_size_samples must be > 0 (got {got})")]
    ZeroHopSize {
        /// The value that was rejected.
        got: u32,
    },

    /// `hop_size_samples` exceeded `frame_size_samples`. The
    /// detector only buffers one frame at a time, so a hop wider
    /// than the frame would lose samples between frames.
    #[error("hop_size_samples ({hop}) must be <= frame_size_samples ({frame})")]
    HopExceedsFrame {
        /// The rejected hop value.
        hop: u32,
        /// The configured frame value.
        frame: u32,
    },

    /// The RMS threshold was negative or `NaN`. RMS is always
    /// non-negative, and a `NaN` threshold would make every frame
    /// compare as `false` (`NaN > NaN` is `false`).
    #[error("rms_threshold must be a finite, non-negative number (got {got})")]
    InvalidThreshold {
        /// The value that was rejected.
        got: f32,
    },

    /// `sample_rate_hz` was set to `0`. The sample rate is used
    /// to convert millisecond debounce knobs to sample counts.
    #[error("sample_rate_hz must be > 0 (got {got})")]
    ZeroSampleRate {
        /// The value that was rejected.
        got: u32,
    },
}
