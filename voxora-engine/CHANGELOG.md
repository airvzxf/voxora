# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] — 2026-09-03

### Changed
- Bumped to 0.4.0 as part of the Fase 4 coordinated release
  (issue #42). The `voxora-core` shim is removed in this
  release; this crate consumes the trait surface via
  `voxora-traits` directly. No API change. The bump continues
  the coordinated version policy introduced for the 0.3.0
  release (see `AGENTS.md` → Version coordination).

## [0.3.0] — 2026-09-03

### Changed
- Version bumped to 0.3.0 as a coordinated version-pin alignment
  with the rest of the workspace (closes #41). No functional or
  API change; the `as_streaming_engine()` default-method
  introduced during the 0.2.0 cycle remains a SemVer-minor
  addition. The bump keeps `voxora-engine` in lock-step with
  `voxora-core` / `voxora-traits` / `voxora-backend` /
  `voxora-bridge` / `voxora-cli` / `voxora-testkit`, all of
  which are already at 0.3.0 — restoring the "coordinated
  bumps by default" rule that the 0.3.0 release silently broke
  for this crate. See `AGENTS.md` → Version coordination.

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