//! `Arc<dyn VadSegmenter>` smoke test.
//!
//! Mirrors `voxora-traits::engine::tests::engine_works_behind_arc_dyn`
//! and `engine_works_across_threads`. Verifies that:
//!
//! - [`VadSegmenter`] is `Send + Sync` (the trait bound itself).
//! - A `Box<dyn VadSegmenter>` can be polled from multiple threads
//!   when wrapped in `Arc<Mutex<…>>` (the practical shape a
//!   consumer would build).
//! - `reset` and `flush` are reachable through the trait object.

use std::sync::{Arc, Mutex};
use std::thread;

use voxora_vad::fixtures::{SILENCE_1S, sine_440hz_500ms};
use voxora_vad::{EnergyVad, VadSegmenter};

#[test]
fn segmenter_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<EnergyVad>();
    assert_send_sync::<Box<dyn VadSegmenter>>();
    assert_send_sync::<Arc<dyn VadSegmenter>>();
    assert_send_sync::<Arc<Mutex<EnergyVad>>>();
}

#[test]
fn segmenter_works_behind_box_dyn() {
    let mut owned: Box<dyn VadSegmenter> = Box::new(EnergyVad::new());

    // Silence produces nothing.
    assert!(owned.next_segment(SILENCE_1S).is_none());
    assert!(owned.flush().is_none());

    // After silence, the detector is in the Silence state. A
    // 440 Hz tone flips it to Speech past the 250 ms debounce
    // — so `next_segment` emits one segment (the silence run
    // that closed) and `flush` emits the trailing speech run.
    let tone = sine_440hz_500ms();
    let silence_close = owned
        .next_segment(&tone)
        .expect("silence→speech flip after debounce");
    assert!(!silence_close.is_speech);
    assert_eq!(silence_close.start_sample, 0);
    assert!(silence_close.end_sample > 0);

    let total_fed = SILENCE_1S.len() as u64 + tone.len() as u64;
    let trailing = owned.flush().expect("flush emits the trailing speech run");
    assert!(trailing.is_speech);
    assert_eq!(trailing.start_sample, silence_close.end_sample);
    assert_eq!(trailing.end_sample, total_fed);
}

#[test]
fn segmenter_works_across_threads() {
    // `Box<dyn VadSegmenter>` requires `&mut self`, so a true
    // cross-thread poller needs interior mutability. We
    // demonstrate the pattern a consumer would build by sharing
    // an `Arc<Mutex<EnergyVad>>` across threads.
    let vad: Arc<Mutex<EnergyVad>> = Arc::new(Mutex::new(EnergyVad::new()));
    let tone = sine_440hz_500ms();

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let vad = Arc::clone(&vad);
            let tone = tone.clone();
            thread::spawn(move || {
                let mut guard = vad.lock().expect("not poisoned");
                guard.next_segment(&tone);
                guard.flush()
            })
        })
        .collect();

    let mut speeches_seen = 0;
    for h in handles {
        let trailing = h.join().expect("thread did not panic");
        if let Some(seg) = trailing {
            assert!(seg.is_speech);
            speeches_seen += 1;
        }
    }

    // The first thread to grab the lock drives the full
    // pipeline and emits the trailing segment. Subsequent
    // threads see `state_seeded == true` but no further
    // transitions, so `flush` returns `None` for them. The
    // invariant is therefore "at least one trailing segment
    // is reported across the pool."
    assert!(
        speeches_seen >= 1,
        "expected at least one trailing segment, got {speeches_seen}"
    );
}

#[test]
fn reset_through_trait_object_clears_state() {
    let mut vad: Box<dyn VadSegmenter> = Box::new(EnergyVad::new());
    vad.next_segment(&sine_440hz_500ms());
    vad.reset();
    // After reset, silence must produce no segments.
    assert!(vad.next_segment(SILENCE_1S).is_none());
    assert!(vad.flush().is_none());
}
