//! Inline audio fixtures used by the voxora test suites.
//!
//! All fixtures are synthesised PCM at 16 kHz mono f32 — small enough
//! to ship inline. They are NOT real recordings (the real
//! `jfk.wav` / `quick_brown_fox.wav` still live in each engine's
//! `tests/parity.rs` for now).

/// 0.5 s of silence at 16 kHz mono f32 — 8 000 samples.
pub const SILENCE_500MS: &[f32] = &[0.0; 8000];

/// 1.0 s of silence at 16 kHz mono f32 — 16 000 samples.
pub const SILENCE_1S: &[f32] = &[0.0; 16000];

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
