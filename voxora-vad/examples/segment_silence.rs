//! Segment the testkit audio fixtures and print the boundaries.
//!
//! This is the canonical example for `voxora-vad`. It feeds two
//! well-known 16 kHz mono PCM fixtures through [`EnergyVad`] with
//! the default config and prints the segments that come back:
//!
//! - [`voxora_testkit::audio::SILENCE_1S`] — 1.0 s of zero
//!   samples. RMS of any window over silence is `0.0`, so the
//!   detector stays in its seeded-Silence state and emits no
//!   segments.
//! - [`voxora_testkit::audio::sine_440hz_500ms`] — 0.5 s of a
//!   440 Hz sine wave at amplitude `0.5`. The very first frame
//!   has RMS ≈ `0.5 / sqrt(2) ≈ 0.354`, well above the default
//!   threshold of `0.01`, so the detector seeds itself into the
//!   Speech state. There are no further transitions, so the
//!   trailing speech run is only released by
//!   [`VadSegmenter::flush`], which returns one segment that
//!   spans the full input.
//!
//! Run it with:
//!
//! ```text
//! cargo run --example segment_silence
//! ```
//!
//! Expected output:
//!
//! ```text
//! SILENCE_1S:
//!   segments: 0
//! sine_440hz_500ms:
//!   segments: 1
//!     [0, 8000) is_speech=true
//! ```

use voxora_testkit::audio::{SILENCE_1S, sine_440hz_500ms};
use voxora_vad::{EnergyVad, VadSegment, VadSegmenter};

fn collect(mut vad: EnergyVad, samples: &[f32], chunk: usize) -> Vec<VadSegment> {
    let mut out = Vec::new();
    for slice in samples.chunks(chunk) {
        if let Some(seg) = vad.next_segment(slice) {
            out.push(seg);
        }
    }
    if let Some(seg) = vad.flush() {
        out.push(seg);
    }
    out
}

fn print_segments(label: &str, segments: &[VadSegment]) {
    println!("{label}:");
    println!("  segments: {}", segments.len());
    for seg in segments {
        println!(
            "    [{}, {}) is_speech={}",
            seg.start_sample, seg.end_sample, seg.is_speech
        );
    }
}

fn main() {
    let silence_segs = collect(EnergyVad::new(), SILENCE_1S, 480);
    print_segments("SILENCE_1S", &silence_segs);

    let tone = sine_440hz_500ms();
    let tone_segs = collect(EnergyVad::new(), &tone, 480);
    print_segments("sine_440hz_500ms", &tone_segs);
}
