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
- New crate. `EngineAdapter` trait exposes engine identity
  (`family`, `info`, `backend`) alongside the `AsrEngine` surface.
- `AnyEngine` wrapper holds any concrete adapter as
  `Arc<dyn EngineAdapter + Send + Sync>` and forwards `AsrEngine`
  calls.
- `EngineFamily` enum (`Whisper`, `Qwen3Asr`, ...) with
  `from_config` / `as_config` / `crate_label` helpers for TOML and
  CLI plumbing.
- `BackendDescriptor` and `EngineInfo` structs that the registry
  uses to describe resolved models without lex-sort heuristics.
- `testing::Mock` helper for adapter tests (consumed by
  `voxora-testkit`).