# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] — 2026-09-03

### Added
- Opt-in `engine-adapter` Cargo feature that adopts
  `voxora_engine::EngineAdapter` so introspectable adapters can be
  composed in `AnyEngine` wrappers (PR A).
- `from_hf` now synthesises a `tokenizer.json` from
  `vocab.json` + `merges.txt` + `tokenizer_config.json` so the
  official Qwen3-ASR HF release loads without manual packaging
  (phase 6 fix, ported into the 0.2.0 changelog).

### Changed
- Engine crate now carries an opt-in dependency on `voxora-engine`
  via the `engine-adapter` feature. The default feature set is
  unchanged.

## [0.1.0] — 2026-07-12

Initial published release (`QwenAsrEngine` over `qwen3-asr-rs`,
20-language validation list, `from_hf` behind the `hf` feature,
`#[ignore]`-gated parity + concurrency tests).