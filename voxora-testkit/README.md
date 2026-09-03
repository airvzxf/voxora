# voxora-testkit

Dev-only test helpers for the voxora workspace. `publish = false`,
no runtime footprint outside test code.

## Modules

- `audio` — inline PCM fixtures at 16 kHz mono f32 (`SILENCE_500MS`,
  `SILENCE_1S`, `sine_440hz_500ms`).
- `fixtures` — `InMemorySource` (mock `ModelSource`) and `EchoEngine`
  (mock `AsrEngine`) for offline contract tests.
- `wer` — `wer()` + `edit_distance()` for ASR parity assertions.

## Usage

Add to a voxora-* crate's `Cargo.toml` as a `dev-dependencies`
entry pointing at the workspace path. The helpers are pure-Rust and
do not pull in `tokio`, `reqwest`, or any network code, so they are
safe in offline CI lanes.

## ASR scope

Fixtures target the standard voxora sample rate (16 kHz, mono,
f32). No multi-modal fixtures.
