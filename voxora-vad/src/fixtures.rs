//! Inline audio fixtures used by the example and the
//! integration tests.
//!
//! The values here are kept byte-identical to the canonical
//! fixtures in `voxora_testkit::audio`:
//!
//! - [`SILENCE_1S`] mirrors `voxora_testkit::audio::SILENCE_1S`.
//! - [`sine_440hz_500ms`] mirrors
//!   `voxora_testkit::audio::sine_440hz_500ms`.
//!
//! `voxora-testkit` is `publish = false`, so depending on it
//! from a publishable crate blocks `cargo publish` (workspace
//! crates cannot be resolved against crates.io). Keeping the
//! fixtures inline is the cheapest way to keep the example and
//! the integration tests well-grounded without sacrificing the
//! "use a named, well-known fixture" property the issue asked
//! for. A new workspace-level fixture module under
//! `voxora-traits` would be the right long-term home (it is
//! publishable), tracked as a follow-up.

/// 1.0 s of silence at 16 kHz mono f32 — 16 000 samples.
///
/// Mirrors `voxora_testkit::audio::SILENCE_1S`.
pub const SILENCE_1S: &[f32] = &[0.0; 16_000];

/// A 440 Hz sine wave at 16 kHz mono f32, 0.5 s long.
///
/// Mirrors `voxora_testkit::audio::sine_440hz_500ms`.
pub fn sine_440hz_500ms() -> Vec<f32> {
    const SR: f32 = 16_000.0;
    const FREQ: f32 = 440.0;
    const N: usize = 8_000;
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
    fn silence_1s_has_expected_length() {
        assert_eq!(SILENCE_1S.len(), 16_000);
        assert!(SILENCE_1S.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn sine_440hz_500ms_is_non_trivial() {
        let s = sine_440hz_500ms();
        assert_eq!(s.len(), 8_000);
        assert!(s.iter().any(|&x| x.abs() > 0.1), "sine must have amplitude");
        assert!(
            s.iter().all(|&x| x.abs() <= 0.5),
            "amplitude should stay ≤ 0.5"
        );
    }
}
