# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] — 2026-09-03

### Removed
- **Breaking**: `voxora-cli::BackendKind` removed (was deprecated
  since 0.2.0). The CLI now uses `voxora_engine::EngineFamily`
  directly. `--engine whisper|qwen3-asr` parsing is unchanged — the
  same case-insensitive matching and aliases (`qwen3_asr`, `qwen3asr`)
  are accepted via `EngineFamily::from_config`.

## [0.2.0] — 2026-09-03

### Deprecated
- `voxora_cli::BackendKind` is marked
  `#[deprecated(since = "0.2.0")]` in favour of
  `voxora_engine::EngineFamily`. Existing `--engine` flag parsing
  keeps working — `BackendKind::from_cli_label` is independent of
  the canonical `EngineFamily`.

## [0.1.0] — 2026-07-12

Initial release (`voxora list | info | download | run | serve`
subcommands, `cpu` / `metal` / `cuda` hardware features,
`make build-cli` + `make build-musl` artefacts).

### Fixed
- `hf_cache_dir()` honours `XDG_CACHE_HOME` /
  `$HOME/.cache/` via `dirs::cache_dir()` so the CLI's default cache
  root matches the platform convention.
- `decode_wav` honours the declared bit depth and uses
  `2^(bits-1)` as the PCM divisor for 16/24/32-bit WAVs (16-bit
  audio was 65536× too quiet previously).