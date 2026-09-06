# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] — 2026-09-05

Initial release as part of the [EPIC #117](https://github.com/airvzxf/voxora/issues/117)
coordinated 0.5.0 minor bump (closes
[#48](https://github.com/airvzxf/voxora/issues/48)). The crate
is a new workspace member; the version is pinned directly to
`0.5.0` to match the rest of the EPIC participants, even though
`[workspace.package]` on trunk still tracks `0.4.3` at this
point. The orchestrator's release branch will realign both
sides when the EPIC #117 merge closes.

### Added

- **New crate** — energy-based voice activity detection utility.
  Slides an RMS window over a stream of mono PCM samples and
  emits a `VadSegment` each time the speech/silence state
  machine transitions past the configured debounce windows.
  CPU-only, deterministic, zero non-workspace runtime deps.

- **`VadSegmenter` trait** (`pub trait VadSegmenter: Send + Sync`)
  with `next_segment(&[f32]) -> Option<VadSegment>`,
  `reset(&mut self)`, and `flush(&mut self) -> Option<VadSegment>`
  for draining the trailing open run at end of stream. The
  trait and `VadSegment` are `#[non_exhaustive]` per
  `AGENTS.md` so the surface can grow without breaking
  downstream callers.

- **`EnergyVad` reference implementation** — the
  sliding-window RMS detector. Defaults are tuned for 16 kHz
  mono f32 audio (30 ms frames, 10 ms hops, RMS threshold
  `0.01`, 250 ms speech / 100 ms silence debounce). A
  fluent `EnergyVadBuilder` exposes every knob for tweaking.

- **`EnergyVadConfig` and `EnergyVadBuilder`** — the
  configuration knobs and a fluent builder. The config is
  validated on construction via `EnergyVadConfig::validate`
  and any violation surfaces as a typed `VadConfigError`
  (mirrors the `thiserror` pattern in `voxora-config`).

- **Tests + integration suite** — every code path is
  deterministic, so all tests are always-run (no
  `#[ignore]` gates). The integration tests in
  `tests/segmentation.rs` consume the canonical
  `voxora-testkit::audio::SILENCE_1S` and
  `sine_440hz_500ms()` fixtures; `tests/trait_object.rs`
  verifies the `Arc<dyn VadSegmenter>` dispatch surface and
  multi-threaded polling.

- **Example `examples/segment_silence.rs`** — feeds the two
  testkit fixtures through the detector and prints the
  resulting segment boundaries. Mirrors the
  `voxora-hf/examples/inspect.rs` doc-comment header pattern.

- **`README.md`** — crate-level landing page so docs.rs
  renders the right page rather than the workspace README.

### Out of scope (tracked as follow-ups)

- **Integration with `voxora_traits::StreamingAsrEngine`** —
  deferred until the first streaming engine lands (issues
  #50/#51). The streaming trait exists but is gated upstream
  on whisper-rs / candle incremental-decoding APIs. When it
  ships, this crate will add a `StreamingVadSegmenter`
  extension that drives the decoder incrementally.
- **ML-based VAD (e.g. Silero VAD via onnxruntime)** — useful
  follow-up but explicitly out of scope per the issue body.
  Will track as a separate issue.