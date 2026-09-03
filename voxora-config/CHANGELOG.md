# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] — 2026-09-03

### Added
- New crate. Environment-variable cascade (and optional TOML
  override) that is the single source of truth for the voxora
  workspace. Honours `VOXORA_CACHE_DIR`, `HF_TOKEN`,
  `HUGGING_FACE_HUB_TOKEN`, `VOXORA_HF_BASE_URL`,
  `VOXORA_HF_REVISION`, plus a layered `voxora.toml` lookup.
- `voxora_config::env::Env::load()` resolves the cascade in
  priority order: explicit overrides → environment → TOML file →
  defaults.
- Adapters in `voxora_config::cache` and `voxora_config::hf` so
  downstream crates (`voxora-hf`, `voxora-cli`) can plug in without
  re-implementing the lookup logic.