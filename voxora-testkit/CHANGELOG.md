# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] — 2026-09-06

Coordinated minor release for [EPIC #117](https://github.com/airvzxf/voxora/issues/117).
Every workspace crate that participates in this release ships at
0.5.0 per `AGENTS.md` § "Version coordination". No public API
change for `voxora-testkit` itself; this release adds the
`real_fixture_download` example (closes #57).

### Added
- **`real_fixture_download` example** (closes #57): a small
  binary that calls `fixtures::real::resolve_real_fixture` for
  a named fixture and prints the path. Demonstrates the canonical
  entry point for parity tests. Run with
  `cargo run --example real_fixture_download -p voxora-testkit -- <fixture-name>`.

### Notes
- The `fixtures::real::download(...)` helper is still a stub
  returning `FixtureError::Network` for any uncached fixture
  (closes the loop tracked in issue #59's deferred follow-up).
  The example surfaces the API gap as documented.

## [0.4.3] — 2026-09-06

Coordinated patch release for [EPIC #109](https://github.com/airvzxf/voxora/issues/109)
(PR [#115](https://github.com/airvzxf/voxora/pull/115)). All 11
workspace crates ship at 0.4.3. No public API change, no SemVer
break. Per `AGENTS.md` § "Version coordination".

### Fixed
- **Cargo.toml header drift**: stale "currently 0.4.0"
  comment updated to 0.4.2.

## [0.4.0] — 2026-09-03

### Changed
- Bumped to 0.4.0 as part of the Fase 4 coordinated release
  (issue #42). The `voxora-core` shim is removed; this
  dev-only crate consumes the trait surface via `voxora-traits`
  directly. No API change. The `fixtures::real::resolve_real_fixture`
  surface shipped in 0.3.0 (Fase 3 PR C) remains.

## [0.3.0] — 2026-09-03

### Added
- New `fixtures::real` submodule exposing the canonical
  `resolve_real_fixture(name)` entry point for parity tests that
  need real audio (`jfk.wav`, `sample1.wav`) or model weights
  (`ggml-tiny.bin`, `qwen3-asr-0.6b`). New `FixtureError` enum
  covers unknown names, missing cache directory, and download
  failures. The download logic itself is still owned by each
  engine's `tests/parity.rs` today; this release ships the
  surface so future engines (parakeet, voxtral, …) have a single
  shared place to land. Three unit tests cover the unknown-name
  path and the `VOXORA_FIXTURE_CACHE_DIR` env override.

### Changed
- Promoted `src/fixtures.rs` to `src/fixtures/mod.rs` to make room
  for the `real` submodule.

## [0.2.0] — 2026-09-03

### Changed
- Bumped to 0.2.0 as part of the Fase 2 coordinated release. No
  functional change; this release aligns the version with the
  breaking changes in `voxora-core` / `voxora-hf` / `voxora-whisper`
  / `voxora-qwen3asr` / `voxora-bridge` / `voxora-cli`.

## [0.1.2] — 2026-09-03

### Added
- New dev-only crate (`publish = false`). Shared fixtures and mocks
  for the workspace tests:
  - `audio` — small WAV fixtures used by parity tests.
  - `fixtures` — pre-recorded `ModelDescriptor` / `ModelDir` /
    `EngineAdapter` helpers.
  - `wer` — word error rate computation shared by whisper and
    qwen3asr parity tests.