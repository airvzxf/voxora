//! Integration test: every public fixture is usable end-to-end.

use voxora_testkit::{SILENCE_1S, SILENCE_500MS, sine_440hz_500ms, wer};

#[test]
fn silence_fixtures_are_zero_amplitude() {
    assert!(SILENCE_500MS.iter().all(|&s| s == 0.0));
    assert!(SILENCE_1S.iter().all(|&s| s == 0.0));
}

#[test]
fn sine_fixture_is_440hz_16khz() {
    let s = sine_440hz_500ms();
    assert_eq!(s.len(), 8000);
    // A 440 Hz tone over 0.5 s completes 220 full cycles, i.e.
    // ~440 zero crossings (two per cycle: pos→neg and neg→pos).
    // Allow a wide tolerance around that.
    let crossings = s
        .windows(2)
        .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
        .count();
    assert!(
        crossings > 430 && crossings < 450,
        "got {crossings} crossings"
    );
}

#[test]
fn wer_against_empty_reference_with_hypothesis_is_one() {
    assert_eq!(wer("", "anything"), 1.0);
}
