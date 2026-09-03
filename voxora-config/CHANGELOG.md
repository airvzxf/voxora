# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] — 2026-09-03

### Changed
- First coordinated-release participation for this crate
  (issue #42). Skipped 0.3.0 because the env-var cascade had
  no Fase-3 surface change. Now aligned with the rest of the
  workspace at 0.4.0 per the policy in `AGENTS.md` →
  Version coordination. No API change. The `voxora-core`
  shim is removed in this release; this crate never depended
  on it directly.

## [0.2.0] — 2026-09-03

### Changed
- Bumped to 0.2.0 as part of the Fase 2 coordinated release. No
  functional change; this release aligns the version with the
  breaking changes in `voxora-core` / `voxora-hf` / `voxora-whisper`
  / `voxora-qwen3asr` / `voxora-bridge` / `voxora-cli`.

## [0.1.2] — 2026-09-03

### Added
- New crate. Environment-variable cascade (and optional TOML
  override) that is the single source of truth for the voxora
  workspace. Honours `VOXORA_CACHE_DIR`, `HF_TOKEN`,
  `HUGGING_FACE_HUB_TOKEN`, `VOXORA_HF_BASE_URL`,
  `VOXORA_HF_REVISION`, plus a layered `voxora.toml` lookup.
- `VoxoraConfig::from_file(path)` resolves the cascade from a TOML
  override file (see `voxora-config/src/file.rs`). The env-var
  cascade itself lives in `voxora_config::env` constants.
- Adapters in `voxora_config::cache` and `voxora_config::hf` so
  downstream crates (`voxora-hf`, `voxora-cli`) can plug in without
  re-implementing the lookup logic.