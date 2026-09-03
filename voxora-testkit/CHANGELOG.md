# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] — 2026-09-03

### Changed
- Bumped to 0.4.0 as part of the Fase 4 coordinated release
  (issue #42). The `voxora-core` shim is removed; this
  dev-only crate consumes the trait surface via `voxora-traits`
  directly. No API change. The `fixtures::real::resolve_real_fixture`
  surface shipped in 0.3.0 (Fase 3 PR C) remains.

## [0.3.0] — 2026-09-03

### Added
- New `fixtures::real` submodule exposing the canonical
  `resolve_real_fixture(name)` entry point for parity tests that
  need real audio (`jfk.wav`, `sample1.wav`) or model weights
  (`ggml-tiny.bin`, `qwen3-asr-0.6b`). New `FixtureError` enum
  covers unknown names, missing cache directory, and download
  failures. The download logic itself is still owned by each
  engine's `tests/parity.rs` today; this release ships the
  surface so future engines (parakeet, voxtral, …) have a single
  shared place to land. Three unit tests cover the unknown-name
  path and the `VOXORA_FIXTURE_CACHE_DIR` env override.

### Changed
- Promoted `src/fixtures.rs` to `src/fixtures/mod.rs` to make room
  for the `real` submodule.

## [0.2.0] — 2026-09-03

### Changed
- Bumped to 0.2.0 as part of the Fase 2 coordinated release. No
  functional change; this release aligns the version with the
  breaking changes in `voxora-core` / `voxora-hf` / `voxora-whisper`
  / `voxora-qwen3asr` / `voxora-bridge` / `voxora-cli`.

## [0.1.2] — 2026-09-03

### Added
- New dev-only crate (`publish = false`). Shared fixtures and mocks
  for the workspace tests:
  - `audio` — small WAV fixtures used by parity tests.
  - `fixtures` — pre-recorded `ModelDescriptor` / `ModelDir` /
    `EngineAdapter` helpers.
  - `wer` — word error rate computation shared by whisper and
    qwen3asr parity tests.