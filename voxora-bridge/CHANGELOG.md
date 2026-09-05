# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.1] — 2026-09-04

Docs-only patch. Per AGENTS.md "Version coordination", the
coordinated-bump rule applies only to X.Y.0 breaking releases;
the other eight workspace crates stay at 0.4.0. No API change.

### Fixed
- Code-spanned two feature-gated intra-doc links in the
  module-level "Example: load a Whisper model from Hugging Face"
  doc block of `src/lib.rs` that failed
  `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p voxora-bridge --no-default-features`
  (issue #88):
  - `[`WhisperEngine::from_hf`]` → `` `WhisperEngine::from_hf` ``
  - `[`QwenAsrEngine::from_hf`]` → `` `QwenAsrEngine::from_hf` ``
  - The other bracketed links on the same doc block
    (`HuggingFaceSource`, `AsrEngine::transcribe`) were left
    intact: those re-exports live behind no feature gate.
- Code-spanned the same latent instance of the bug in
  `examples/bridge_demo.rs` (`[`WhisperEngine`]` →
  `` `WhisperEngine` ``). `whisper` is already a default
  feature of this crate, so the example builds under
  `--features` defaults today; the bug is dormant under
  `--no-default-features`. Fixed while in the area so a
  future `--no-default-features` consumer does not hit a
  `--no-default-features` rustdoc failure.

## [0.4.0] — 2026-09-03

### Changed
- Bumped to 0.4.0 as part of the Fase 4 coordinated release
  (issue #42). The umbrella crate now re-exports the trait
  surface from `voxora-traits` exclusively — the `voxora-core`
  re-export that backed this crate in 0.3.0 is removed in
  this release. No API change for downstream consumers that
  used the `voxora_bridge::AsrEngine` style import (those
  resolve to `voxora_traits::AsrEngine` now).

## [0.3.0] — 2026-09-03

### Removed
- **Breaking**: `voxora-bridge::ModelKind` removed (was deprecated
  since 0.2.0). Use `voxora_engine::EngineFamily` (re-exported from
  voxora-bridge).
- **Breaking**: `voxora-bridge::InvalidModelKind` removed (paired with
  `ModelKind`'s `FromStr` impl). Use `voxora_engine::InvalidEngineFamily`.

## [0.2.0] — 2026-09-03

### Deprecated
- `voxora_bridge::ModelKind` is marked
  `#[deprecated(since = "0.2.0")]` in favour of
  `voxora_engine::EngineFamily`. The variant still resolves and
  forwards to the matching engine feature, but new code should
  import the engine-family type from `voxora-engine`.

## [0.1.1] — 2026-07-14

### Changed
- Forwarded `cuda` is now split into `cuda-whisper` /
  `cuda-qwen3asr` so downstream crates on hardware older than
  sm_70 (Volta) can enable `cuda-whisper` only. Added forwarding
  `metal` and `vulkan` features to match.

## [0.1.0] — 2026-07-12

Initial published release (umbrella crate re-exporting
`voxora-core`, `voxora-hf`, and the engine adapters behind
`whisper` / `qwen3asr` feature flags).