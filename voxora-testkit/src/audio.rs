//! Inline audio fixtures used by the voxora test suites.
//!
//! All fixtures are synthesised PCM at 16 kHz mono f32 — small enough
//! to ship inline. They are NOT real recordings (the real
//! `jfk.wav` / `quick_brown_fox.wav` still live in each engine's
//! `tests/parity.rs` for now).

use std::sync::LazyLock;

/// 0.5 s of silence at 16 kHz mono f32 — 8 000 samples.
pub const SILENCE_500MS: &[f32] = &[0.0; 8000];

/// 1.0 s of silence at 16 kHz mono f32 — 16 000 samples.
pub const SILENCE_1S: &[f32] = &[0.0; 16000];

/// 30 s of silence at 16 kHz mono f32 — 480 000 samples.
///
/// Added for EPIC #133 (PR #56): the Real-Time Factor (RTF)
/// benches for `voxora-whisper` and `voxora-qwen3asr` need a
/// fixed-length input whose audio duration is well-known (RTF =
/// transcription_time / audio_duration). 30 s is the standard
/// benchmark length — long enough that decode setup overhead
/// amortises, short enough that a stub bench completes quickly.
/// Wrapped in a [`LazyLock`] because a 480 000-element `Vec`
/// constant would otherwise bloat the binary's `.rodata`; this
/// way it is allocated lazily and shared across the test suite
/// without per-call re-allocation.
pub static SILENCE_30S: LazyLock<Vec<f32>> = LazyLock::new(|| vec![0.0_f32; 480_000]);

/// A 440 Hz sine wave at 16 kHz mono f32, 0.5 s long.
pub fn sine_440hz_500ms() -> Vec<f32> {
    const SR: f32 = 16_000.0;
    const FREQ: f32 = 440.0;
    const N: usize = 8000;
    let mut out = Vec::with_capacity(N);
    for i in 0..N {
        let t = i as f32 / SR;
        out.push(0.5 * (2.0 * std::f32::consts::PI * FREQ * t).sin());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_500ms_has_expected_length() {
        assert_eq!(SILENCE_500MS.len(), 8000);
        assert!(SILENCE_500MS.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn silence_1s_has_expected_length() {
        assert_eq!(SILENCE_1S.len(), 16000);
    }

    #[test]
    fn silence_30s_has_expected_length_and_is_zero() {
        assert_eq!(SILENCE_30S.len(), 480_000);
        assert!(
            SILENCE_30S.iter().all(|&s| s == 0.0),
            "SILENCE_30S must be zero-amplitude"
        );
    }

    #[test]
    fn sine_440hz_500ms_is_non_trivial() {
        let s = sine_440hz_500ms();
        assert_eq!(s.len(), 8000);
        assert!(s.iter().any(|&x| x.abs() > 0.1), "sine must have amplitude");
        assert!(
            s.iter().all(|&x| x.abs() <= 0.5),
            "amplitude should stay ≤ 0.5"
        );
    }
}
