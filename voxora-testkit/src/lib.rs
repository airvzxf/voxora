//! Test helpers for the voxora workspace (offline-only, ASR-specific).
//!
//! Crate is **dev-only**: `publish = false`. It is intended to be a
//! `dev-dependencies` of other voxora-* test suites.
//!
//! Today it provides:
//!
//! - [`audio`] — inline PCM fixtures (silence, sine-wave).
//! - [`fixtures`] — `InMemorySource` mock for
//!   [`voxora_core::ModelSource`] and `EchoEngine` mock for
//!   [`voxora_core::AsrEngine`].
//! - [`mod@wer`] — word error rate + edit distance.
//!
//! ASR-specific: fixtures are PCM at 16 kHz, the standard sample rate
//! for voxora engines. No multi-modal fixtures.

#![warn(missing_docs)]

pub mod audio;
pub mod fixtures;
pub mod wer;

pub use audio::{SILENCE_1S, SILENCE_500MS, sine_440hz_500ms};
pub use fixtures::{EchoEngine, InMemorySource};
pub use wer::{edit_distance, wer};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_surface_round_trips() {
        assert_eq!(SILENCE_500MS.len(), 8000);
        assert_eq!(SILENCE_1S.len(), 16000);
        assert_eq!(wer("hello", "hello"), 0.0);
    }
}
