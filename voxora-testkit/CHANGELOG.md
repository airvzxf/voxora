# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.2] — 2026-09-06

Coordinated patch release for [EPIC #133](https://github.com/airvzxf/voxora/issues/133)
(closes #54, #56, #59, #137). The 7 participating crates
(`voxora-traits`, `voxora-engine`, `voxora-registry`, `voxora-whisper`,
`voxora-qwen3asr`, `voxora-testkit`, `voxora-bridge`) ship at 0.5.2;
the remaining 6 stay at 0.5.1 per `AGENTS.md` § "Version
coordination" additive-exception path. **No public API change**, no
SemVer break.

### Changed
- Coordinated patch bump for EPIC #133 quality hardening
  ([#134](https://github.com/airvzxf/voxora/pull/134),
   [#135](https://github.com/airvzxf/voxora/pull/135),
   [#136](https://github.com/airvzxf/voxora/pull/136),
   [#138](https://github.com/airvzxf/voxora/pull/138)).
  All changes additive; no API changes.

### Changed
- **Fixtures** —
  `voxora_testkit::fixtures::real::resolve_real_fixture` now
  actually downloads `jfk.wav` / `sample1.wav` via `ureq` (sync).
  The previous async stub returned `FixtureError::Network`. Old
  stub removed. `voxora-testkit` is `publish = false`; no crates.io
  surface impact.
- **Audio** — removed `SILENCE_30S` (was added then deleted in the
  same release cycle; the RTF benches that motivated it ship as
  stubs and don't consume it).

## [0.5.1] — 2026-09-06

Coordinated patch release for [EPIC #124](https://github.com/airvzxf/voxora/issues/124)
(closes #119, #121, #122). All 13 workspace crates ship at 0.5.1.
No public API change, no SemVer break. Per `AGENTS.md` § "Version
coordination".

### Fixed
- **Pre-publish internal-deps guard learns about `kind` and `publish = false`**
  (closes #119): `.github/workflows/release.yml::publish-cratesio` now
  emits kind-aware (`[dev-dependencies]` / `[build-dependencies]`)
  error annotations, and short-circuits with a "vendoring required"
  message before any HTTP round-trip when the target is a
  `publish = false` workspace crate. `cargo metadata` already
  returns a flat `.dependencies[]` array with a `kind` field, so
  the walker did not need extension — only the remediation paths.

### Added
- **`vulkan` Cargo feature on `voxora-cli` + `--hardware` flag**
  (closes #121): the CLI binary now forwards `vulkan` to
  `voxora-whisper/vulkan` (whisper-only, mirroring
  `voxora-bridge/Cargo.toml`; `qwen3-asr` upstream has no Vulkan
  backend). The `voxora run` subcommand gains
  `--hardware <cpu|cuda|metal|vulkan>` for build-time validation
  and observability — the flag is informational, not a runtime GPU
  switch (whisper-rs picks its own runtime backend from compiled
  Cargo features). `--hardware vulkan --engine qwen3-asr` fails fast
  with a clear message.

### Changed
- **`Waker::noop()` replaces manual `impl Wake for Noop`** (closes
  #122): `voxora-testkit/src/fixtures/mod.rs`'s dormant `mod tests`
  block now uses `std::task::Waker::noop()` (stable since Rust
  1.85, well below the 1.88 MSRV) instead of a hand-rolled `Wake`
  impl. `voxora-local/tests/local_source.rs` was already on
  `Waker::noop()` (migrated during EPIC #117) and required no
  change. Silences the new `clippy::manual_noop_waker` lint
  (added in clippy 1.98.0, the toolchain pinned by
  `rust-toolchain.toml`) the moment `voxora-testkit`'s
  `[lib] test` flag flips to `true`.
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