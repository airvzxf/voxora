# voxora-vad

Energy-based voice activity detection (VAD) for the
[voxora](https://github.com/airvzxf/voxora) workspace. Tiny,
deterministic, CPU-only, zero non-workspace runtime
dependencies.

The detector slides a window over a stream of mono PCM samples,
computes the RMS energy in each frame, and emits a
[`VadSegment`](https://docs.rs/voxora-vad/latest/voxora_vad/struct.VadSegment.html)
each time the speech/silence state machine transitions past the
configured debounce windows.

## Use cases

- **Trimming silence before a batch ASR pass.** Pass the audio
  through [`EnergyVad`] first, then send only the speech runs to
  the engine.
- **Live-transcription UIs.** Poll
  [`VadSegmenter::next_segment`](https://docs.rs/voxora-vad/latest/voxora_vad/trait.VadSegmenter.html)
  on each chunk of microphone PCM and only wake the decoder when
  `is_speech == true`.
- **Pre-filtering before speaker diarization.** The diarizer
  only sees the speech runs, not the gaps.

## Quick start

```rust
use voxora_vad::{EnergyVad, VadSegmenter, VadSegment};

let mut vad = EnergyVad::new();

let samples: Vec<f32> = vec![0.0; 16_000];
let seg: Option<VadSegment> = vad.next_segment(&samples);
assert!(seg.is_none()); // pure silence → no transition

let trailing: Option<VadSegment> = vad.flush();
```

The example in `examples/segment_silence.rs` runs the canonical
`voxora-testkit` fixtures through the detector and prints the
boundaries:

```text
$ cargo run --example segment_silence
SILENCE_1S:
  segments: 0
sine_440hz_500ms:
  segments: 1
    [0, 8000) is_speech=true
```

## Streaming integration

The voxora streaming engine trait
([`voxora_traits::StreamingAsrEngine`](https://docs.rs/voxora-traits))
is intentionally **not** wired up by this crate. Streaming
engines are still gated upstream on whisper-rs / candle
incremental-decoding APIs (issues #50/#51). When the first
streaming engine lands, a future patch will add a
`StreamingVadSegmenter` extension that drives the decoder
incrementally.

## Crate conventions

- **Zero non-workspace runtime deps.** The detector is pure
  `f32` arithmetic over a sliding window, so the crate stays
  hermetic on a fresh `cargo build`. Matches the
  "tiny pure-Rust" stance of [`voxora-config`](https://docs.rs/voxora-config).
- **Deterministic, no `#[ignore]` tests.** The crate has no
  model/audio fixtures, so every test runs unconditionally and
  the CI test job stays fast.
- **ASR-specific.** Not a generic signal-processing library.
  VAD here is a building block for the voxora speech-recognition
  stack only.

## License

Apache-2.0. See [LICENSE](https://github.com/airvzxf/voxora/blob/main/LICENSE).