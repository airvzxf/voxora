# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] — 2026-09-03

### Deprecated
- `voxora_bridge::ModelKind` is marked
  `#[deprecated(since = "0.2.0")]` in favour of
  `voxora_engine::EngineFamily`. The variant still resolves and
  forwards to the matching engine feature, but new code should
  import the engine-family type from `voxora-engine`.

### Changed
- Forwarded `cuda` is now split into `cuda-whisper` /
  `cuda-qwen3asr` so downstream crates on hardware older than
  sm_70 (Volta) can enable `cuda-whisper` only.

## [0.1.1] — 2026-07-14

Forwarded `cuda` / `metal` / `vulkan` hardware features to the
matching engine features.

## [0.1.0] — 2026-07-12

Initial published release (umbrella crate re-exporting
`voxora-core`, `voxora-hf`, and the engine adapters behind
`whisper` / `qwen3asr` feature flags).