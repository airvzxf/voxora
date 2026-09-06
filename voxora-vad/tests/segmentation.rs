//! Integration tests for `voxora-vad` driven by
//! `voxora-testkit::audio` fixtures.
//!
//! These tests are always-run (no `#[ignore]` gates): the
//! detector is CPU-only and deterministic, so CI exercises every
//! code path on every PR. The fixtures are 16 kHz mono f32
//! inline arrays — no downloads, no network.
//!
//! Mirrors the round-trip smoke pattern in
//! `voxora-traits/src/engine.rs::tests`.

use pretty_assertions::assert_eq;
use voxora_testkit::audio::{SILENCE_1S, sine_440hz_500ms};
use voxora_vad::{EnergyVad, VadSegment, VadSegmenter};

fn collect(vad: EnergyVad, samples: &[f32], chunk: usize) -> Vec<VadSegment> {
    let mut vad = vad;
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

#[test]
fn silence_fixture_emits_no_segments() {
    let vad = EnergyVad::new();
    let segs = collect(vad, SILENCE_1S, 480);
    assert!(
        segs.is_empty(),
        "silence must not produce any segment, got {segs:?}"
    );
}

#[test]
fn tone_fixture_emits_one_full_run_via_flush() {
    // 0.5 s of 440 Hz tone at amplitude 0.5. The very first
    // frame seeds the state to Speech (RMS ≈ 0.354 ≫ 0.01),
    // so no segment is emitted during the chunk. `flush`
    // releases the trailing speech run, which by convention
    // spans the whole input.
    let tone = sine_440hz_500ms();
    let vad = EnergyVad::new();
    let segs = collect(vad, &tone, 480);

    assert_eq!(
        segs.len(),
        1,
        "pure-tone input should yield exactly one trailing segment, got {segs:?}"
    );
    let seg = &segs[0];
    assert_eq!(seg.start_sample, 0);
    assert_eq!(seg.end_sample, tone.len() as u64);
    assert!(seg.is_speech);
}

#[test]
fn silence_tone_silence_emits_two_segments() {
    let sr = 16_000_usize;
    let silence_a = vec![0.0_f32; sr / 2]; // 0.5 s
    let tone = sine_440hz_500ms(); // 0.5 s of 440 Hz tone
    let silence_b = vec![0.0_f32; sr / 2]; // 0.5 s

    let mut all = Vec::with_capacity(sr * 3 / 2);
    all.extend_from_slice(&silence_a);
    all.extend_from_slice(&tone);
    all.extend_from_slice(&silence_b);

    let vad = EnergyVad::new();
    let segs = collect(vad, &all, 480);

    assert_eq!(
        segs.len(),
        2,
        "expected silence→speech + speech→silence, got {segs:?}"
    );

    let silence_seg = &segs[0];
    assert!(!silence_seg.is_speech);
    assert_eq!(silence_seg.start_sample, 0);

    let speech_seg = &segs[1];
    assert!(speech_seg.is_speech);
    assert_eq!(speech_seg.start_sample, silence_seg.end_sample);
    assert!(speech_seg.end_sample > speech_seg.start_sample);
}

#[test]
fn amplitude_below_threshold_is_treated_as_silence() {
    // 440 Hz tone at amplitude 0.001 — RMS ≈ 0.0007, which is
    // well below the default threshold of 0.01. The detector
    // should treat this as silence and emit no segments.
    let mut quiet = Vec::with_capacity(8000);
    for i in 0..8000 {
        let t = i as f32 / 16_000.0;
        quiet.push(0.001 * (2.0 * std::f32::consts::PI * 440.0 * t).sin());
    }

    let vad = EnergyVad::new();
    let segs = collect(vad, &quiet, 480);
    assert!(
        segs.is_empty(),
        "sub-threshold tone must be classified as silence, got {segs:?}"
    );
}

#[test]
fn amplitude_above_threshold_is_treated_as_speech() {
    // 440 Hz tone at amplitude 0.5 — RMS ≈ 0.354, well above
    // the default threshold of 0.01. Should produce the same
    // trailing-segment pattern as the testkit sine fixture.
    let mut loud = Vec::with_capacity(8000);
    for i in 0..8000 {
        let t = i as f32 / 16_000.0;
        loud.push(0.5 * (2.0 * std::f32::consts::PI * 440.0 * t).sin());
    }

    let vad = EnergyVad::new();
    let segs = collect(vad, &loud, 480);
    assert_eq!(segs.len(), 1);
    assert!(segs[0].is_speech);
    assert_eq!(segs[0].start_sample, 0);
    assert_eq!(segs[0].end_sample, loud.len() as u64);
}

#[test]
fn reset_returns_detector_to_seeded_silence() {
    let tone = sine_440hz_500ms();
    let mut vad = EnergyVad::new();

    // Force the detector into Speech state.
    let _ = vad.next_segment(&tone);
    assert!(vad.total_fed() > 0);

    vad.reset();
    assert_eq!(vad.total_fed(), 0);

    // After reset, silence produces no segments.
    let segs = collect(vad, SILENCE_1S, 480);
    assert!(segs.is_empty(), "post-reset silence must emit nothing");
}

#[test]
fn custom_config_tightens_detection() {
    // A higher RMS threshold plus a longer min_speech window
    // must produce the same outcomes on these simple fixtures
    // (the amplitudes are far enough from the threshold that
    // the exact debounce tuning does not matter).
    let vad = EnergyVad::builder()
        .rms_threshold(0.1)
        .min_speech_ms(500)
        .min_silence_ms(200)
        .build()
        .expect("valid config");

    let tone = sine_440hz_500ms();
    let segs = collect(vad, &tone, 480);
    assert_eq!(segs.len(), 1);
    assert!(segs[0].is_speech);
}
