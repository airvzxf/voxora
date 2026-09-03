# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.1] — 2026-09-03

### Deprecated
- The entire crate now emits a crate-level
  `#![deprecated(since = "0.3.1", note = "use voxora_traits
  instead; voxora-core shim is removed in voxora 0.4.0")]`
  warning. Any `use voxora_core::...;` (or
  `voxora-core = { ... }` in a downstream `Cargo.toml`) now
  produces a deprecation diagnostic with a one-cycle courtesy
  window before the crate is deleted entirely in voxora 0.4.0
  (issue #42). Migration target is `voxora-traits`; the voxora
  workspace itself has already completed this migration.

### Changed
- Bumped to `0.3.1`. This is the **first and only** release
  of `voxora-core` that is NOT in lock-step with
  `[workspace.package].version`: the other 10 crates stay at
  their existing versions (the 0.3.0 family for crates that
  participated in Fase 3, the 0.2.0 family for those that did
  not).
- `voxora-core/Cargo.toml` now hardcodes
  `version = "0.3.1"` instead of using
  `version.workspace = true`, so the workspace-wide bump to
  0.4.0 that lands together with the deletion of this shim
  (issue #42) cannot accidentally drag voxora-core forward
  into another pre-removal release. The hardcoded version is
  the documented exception to the "coordinated bumps by
  default" rule in `AGENTS.md` → Version coordination.
- The `serde` feature forwarding
  (`serde = ["voxora-traits/serde"]`) is unchanged. The
  `#![deprecated]` attribute warns; it does not disable the
  public API surface, so the re-export chain
  (`voxora_core::*` → `voxora_traits::*`) continues to
  compile.

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