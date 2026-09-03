# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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