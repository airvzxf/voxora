# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- Updated the README's "Limitations" section to point at the new
  `Registry::with_builtin_descriptors_and_chained_source` helper
  shipped in `voxora-registry` 0.5.3 (closes #120's
  registry-side gap; this crate is downstream of that fix).
  **No API change** to `voxora-local` itself — per `AGENTS.md` §
  "Version coordination" additive-exception path, this crate
  stays at 0.5.1.

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

Initial published release as part of the
[EPIC #117](https://github.com/airvzxf/voxora/issues/117)
coordinated 0.5.0 minor bump (closes
[#49](https://github.com/airvzxf/voxora/issues/49)).

### Added
- `LocalSource` — `voxora_traits::ModelSource` impl that resolves
  model ids against an already-vendored directory on disk. No
  network, no tokio, no reqwest, no `voxora-config`. `resolve`
  joins `model_id` against the configured root and returns a
  `ModelDir` with `entry` set; missing files surface as
  `AsrError::ModelNotFound` naming both the missing path and the
  configured root.
- `ChainedSource` — first-hit-wins adapter that tries a primary
  source and falls back to a secondary source on
  `AsrError::ModelNotFound`. Honours the spirit of the issue's
  "local first, Hugging Face on miss" claim without forcing a
  `voxora-registry` refactor.

### Notes
- Documented limitation: `voxora-registry`'s built-in descriptors
  do not yet accept `SourceKind::Local` ids; consumers wanting a
  fallback chain should pass a `ChainedSource` directly to
  `Registry::new` rather than relying on the descriptor accept
  arm. A follow-up issue tracks closing the loop.
- Re-exported from `voxora-bridge 0.5.0` behind the non-default
  `local` Cargo feature.
