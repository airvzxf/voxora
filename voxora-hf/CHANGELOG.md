# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] — 2026-09-03

### Added
- `config` Cargo feature (now the default) that pulls in
  `voxora-config` and uses it as the single source of truth for the
  env-var cascade: cache root, Hugging Face token, HF base URL,
  default revision, and TOML overrides.

### Changed
- The default feature set now enables `config`; downstream consumers
  that need the pre-0.2.0 inline cascade (read-only `VOXORA_CACHE_DIR`,
  `HF_TOKEN`, `HUGGING_FACE_HUB_TOKEN`) must opt out with
  `default-features = false`.

## [0.1.0] — 2026-07-12

Initial published release (`HuggingFaceSource: ModelSource`,
wiremock integration tests, `#[ignore]`-gated live smoke test).