//! Energy-based voice activity detection (VAD) for the voxora
//! workspace.
//!
//! `voxora-vad` ships a tiny, deterministic, CPU-only VAD
//! implementation that downstream crates (batch ASR front-ends,
//! live-transcription UIs, future speaker-diarization pre-filters)
//! can pull in without taking on a heavyweight ML dependency.
//! The detector slides a window over a stream of mono PCM samples,
//! computes the RMS energy in each frame, and emits
//! [`VadSegment`]s whenever the speech/silence state machine
//! transitions past the configured debounce windows.
//!
//! # When to use this crate
//!
//! | Use case | Notes |
//! |---|---|
//! | **Trimming silence before a batch ASR pass** | Pass the audio through [`EnergyVad`] first, then send only the speech runs to the engine. Caveat: trimming changes sample alignment, so re-stamp timestamps against the trimmed audio before showing UI timings. |
//! | **Live-transcription UIs** | Poll [`VadSegmenter::next_segment`] on each chunk of microphone PCM and only wake the decoder when `is_speech == true`. The ML-based Silero VAD via onnxruntime is a planned follow-up but is **not** part of this crate. |
//! | **Pre-filtering before diarization** | Speaker diarization ("who spoke when") composes on top of VAD — the diarizer only sees the speech runs, not the gaps. Diarization itself is tracked separately (see `docs/ROADMAP.md` Phase 7). |
//!
//! Integration with `voxora_traits::StreamingAsrEngine` is
//! intentionally **not** included in this crate: streaming
//! engines are still gated upstream on
//! whisper-rs / candle incremental-decoding APIs (issues
//! #50/#51). When the first streaming engine lands, a future
//! patch will add a `StreamingVadSegmenter` extension that
//! drives the decoder incrementally.
//!
//! # Example
//!
//! ```no_run
//! use voxora_vad::{EnergyVad, VadSegmenter, VadSegment};
//!
//! let mut vad = EnergyVad::new();
//!
//! // Feed a chunk of PCM. Returns a segment iff this call
//! // closed a speech/silence run; `None` while the detector is
//! // still inside an open run.
//! let samples: Vec<f32> = vec![0.0; 16_000];
//! let seg: Option<VadSegment> = vad.next_segment(&samples);
//! assert!(seg.is_none()); // pure silence → no transition
//!
//! // At end-of-stream, drain any trailing open run.
//! let trailing: Option<VadSegment> = vad.flush();
//! ```
//!
//! # Crate conventions
//!
//! - **Zero non-workspace runtime deps** — the detector is pure
//!   `f32` arithmetic over a sliding window, so the crate stays
//!   hermetic on a fresh `cargo build`. Matches the
//!   "tiny pure-Rust" stance of [`voxora-config`].
//! - **Deterministic, no `#[ignore]` tests** — the crate has no
//!   model/audio fixtures, so every test runs unconditionally
//!   and the CI test job stays fast.
//! - **ASR-specific** — not a generic signal-processing library.
//!   VAD here is a building block for the voxora
//!   speech-recognition stack only.
//!
//! [`voxora-config`]: https://docs.rs/voxora-config

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod energy;
mod error;
mod traits;

pub use energy::{EnergyVad, EnergyVadBuilder, EnergyVadConfig};
pub use error::VadConfigError;
pub use traits::{VadSegment, VadSegmenter};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_surface_round_trips() {
        // Compile-time check that the re-exports stay in sync
        // with the trait surface — mirrors the round-trip smoke
        // test in `voxora-traits/src/lib.rs`.
        let mut vad = EnergyVad::new();
        let _: Option<VadSegment> = vad.next_segment(&[]);
        vad.reset();
        let _: Option<VadSegment> = vad.flush();
        // The trait itself is reachable behind a dyn pointer.
        let _: Box<dyn VadSegmenter> = Box::new(EnergyVad::new());
    }
}
