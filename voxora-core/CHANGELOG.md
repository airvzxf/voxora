# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] — 2026-09-03

### Changed
- **Trait surface moved to `voxora-traits`.** This crate is
  retained as a thin compatibility shim that re-exports
  `voxora-traits::*` and forwards the optional `serde` feature.
  Callers depending on `voxora_core::*` keep compiling unchanged;
  new code should depend on `voxora-traits` directly.

### Added
- `serde` Cargo feature now forwards to `voxora-traits/serde`,
  preserving the same `voxora_core::*` derive surface for
  existing consumers.

### Notes
- The previously published `0.1.0` / `0.2.0` of this crate
  contained the original trait surface (`AsrEngine`,
  `ModelSource`, `TranscribeOptions`, etc.). Those types now
  live in `voxora-traits`; this crate only re-exports them.

## [0.2.0] — 2026-09-03

### Added
- `ModelDir::with_entry(path, entry, kind, quantization)` constructor
  so a `ModelSource` can name the specific file inside the directory
  it resolved. Whole-repo resolvers keep using `ModelDir::new`,
  which leaves `entry` as `None`.

### Changed
- `ModelDir` now carries an `entry: Option<PathBuf>` field. Engines
  that relied on `ModelDir::path` alone continue to work (the engine
  falls back to a directory scan), but the new field is the
  authoritative answer for `org/repo/file` resolutions.

## [0.1.0] — 2026-07-12

Initial published release.