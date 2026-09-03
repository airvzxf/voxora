# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] — 2026-09-03

### Changed
- First coordinated-release participation for this crate
  (issue #42). Skipped 0.3.0 because the central model
  resolver had no Fase-3 surface change. Now aligned with the
  rest of the workspace at 0.4.0 per the policy in
  `AGENTS.md` → Version coordination. No API change. The
  `voxora-core` shim is removed; this crate now consumes the
  trait surface via `voxora-traits` directly.

## [0.2.0] — 2026-09-03

### Changed
- Bumped to 0.2.0 as part of the Fase 2 coordinated release. No
  functional change; this release aligns the version with the
  breaking changes in `voxora-core` / `voxora-hf` / `voxora-whisper`
  / `voxora-qwen3asr` / `voxora-bridge` / `voxora-cli`.

## [0.1.2] — 2026-09-03

### Added
- New crate. `Registry::with_builtin_descriptors()` ships a default
  registry populated with the canonical Qwen3-ASR and Whisper
  descriptors, resolved through the voxora-hf cascade when the `hf`
  feature is on (default).

### Fixed
- The builtin Qwen3-ASR descriptor accepts `-suffix` siblings
  (`Qwen/Qwen3-ASR-0.6B` / `-1.7B` / `-2.0B` / `-old`) so every
  official Qwen3-ASR release resolves to the same engine family.

## [0.1.0] — 2026-07-12

Initial published release (central model registry, builtin
descriptors, HF adapter behind the `hf` feature).