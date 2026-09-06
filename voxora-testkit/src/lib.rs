//! Test helpers for the voxora workspace (offline-only, ASR-specific).
//!
//! Crate is **dev-only**: `publish = false`. It is intended to be a
//! `dev-dependencies` of other voxora-* test suites.
//!
//! Today it provides:
//!
//! - [`audio`] — inline PCM fixtures (silence, sine-wave).
//! - [`fixtures`] — `InMemorySource` mock for
//!   [`voxora_traits::ModelSource`] and `EchoEngine` mock for
//!   [`voxora_traits::AsrEngine`], plus the
//!   [`fixtures::real::resolve_real_fixture`] stub for parity tests
//!   that need real audio / model weights.
//! - [`mod@wer`] — word error rate + edit distance.
//!
//! ASR-specific: fixtures are PCM at 16 kHz, the standard sample rate
//! for voxora engines. No multi-modal fixtures.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod audio;
pub mod fixtures;
pub mod wer;

pub use audio::{SILENCE_1S, SILENCE_30S, SILENCE_500MS, sine_440hz_500ms};
pub use fixtures::real::{FixtureError, KNOWN_FIXTURES, resolve_real_fixture};
pub use fixtures::{EchoEngine, InMemorySource};
pub use wer::{edit_distance, wer};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_surface_round_trips() {
        assert_eq!(SILENCE_500MS.len(), 8000);
        assert_eq!(SILENCE_1S.len(), 16000);
        assert_eq!(SILENCE_30S.len(), 480_000);
        assert_eq!(wer("hello", "hello"), 0.0);
    }
}
