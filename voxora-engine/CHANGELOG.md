# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] — 2026-09-06

Coordinated minor release for [EPIC #117](https://github.com/airvzxf/voxora/issues/117).
Every workspace crate that participates in this release ships at
0.5.0 per `AGENTS.md` § "Version coordination". No public API
change for `voxora-engine` itself; this release adds the
`adapter_dispatch` example (closes #57) and a cross-reference to
`docs/GPU_SUPPORT.md` from the per-crate README (closes #55).

### Added
- **`adapter_dispatch` example** (closes #57): the smallest
  possible program that proves the adapter contract works without
  touching Hugging Face or loading a real model. Wraps a
  `MockAdapter` behind `AnyEngine`, dispatches on `family()`, and
  calls the borrowed `voxora_traits::AsrEngine` synchronously.
  Run with `cargo run --example adapter_dispatch -p voxora-engine`.
- **`docs/GPU_SUPPORT.md` link** from this crate's README
  (closes #55): the cross-engine hardware matrix is now the
  single source of truth for which engine ships which backend,
  CUDA compute capability per backend, and the runtime-picker
  guidance for the `candle` Cargo feature.

## [0.4.3] — 2026-09-06

Coordinated patch release for [EPIC #109](https://github.com/airvzxf/voxora/issues/109)
(PR [#115](https://github.com/airvzxf/voxora/pull/115)). All 11
workspace crates ship at 0.4.3. No public API change, no SemVer
break. Per `AGENTS.md` § "Version coordination".

### Fixed
- **Cargo.toml header drift**: stale "currently 0.3.0"
  comment updated to 0.4.2.

## [0.4.2] — 2026-09-05

Coordinated patch release for [EPIC #100](https://github.com/airvzxf/voxora/issues/100)
(PR [#105](https://github.com/airvzxf/voxora/pull/105)). All 11 workspace
crates ship at 0.4.2; the previous partial 0.4.1 release (commit
ecbfd2e, docs-only patches on voxora-traits/registry/bridge) remains
on crates.io as a separate tag. Per `AGENTS.md` § "Version coordination".

### Changed
- **MSRV bumped from 1.86 to 1.88.** The declared floor was
  aspirational since `whisper-rs-sys 0.15.0` raised the real floor
  to 1.88 with no CI job defending the claim. Consumers pinned to
  rustc 1.86 or 1.87 will need to update. The new `msrv` CI job
  at `rustc 1.88` enforces the floor going forward.
- **Cargo.toml workspace pin**: `chacha20` bumped from yanked
  0.10.1 to current 0.10.2 (RustCrypto bugfix; public API
  unchanged; 107 transitives unchanged). `deny.toml` now
  enforces `yanked = "deny"` so the next yanked transitive
  surfaces as a hard CI failure in the existing `cargo-deny` job
  instead of a silent warning (issue #96).

### Added
- **CI tracked-artifact guard** (issue #99): the
  `.github/workflows/ci.yml::fmt-check` job now rejects any
  tracked path matching `(target|.cargo-target|.worktrees)/` or
  cargo's content markers (`CACHEDIR.TAG`, `.rustc_info.json`,
  `.rustdoc_fingerprint.json`). Catches the class of bug that
  landed 4289 files into `main` in PR #97.
- **CI single-engine doc matrix** (issue #95): the `doc` job now
  runs `cargo doc --no-deps --workspace` in 4 configurations —
  default features, `--no-default-features`,
  `--no-default-features --features voxora-bridge/whisper`, and
  `--no-default-features --features voxora-bridge/qwen3asr`. The
  matrix uses `fail-fast: false` so a regression in one leg
  still reports the other.
- **CI MSRV check job** (issue #92): a new round-2 job runs
  `cargo check --workspace --locked --all-targets` at
  `RUSTUP_TOOLCHAIN=1.88` to defend the declared `rust-version`.
- **docs.rs metadata** (issue #93): this crate carries a
  `[package.metadata.docs.rs]` block. The hardware-backend
  features (`metal`, `cuda`, `vulkan`) are SDK-gated and stay
  off the docs.rs build via an explicit `features = [...]` list;
  safe features use `all-features = true`.
- **Pre-publish internal-deps guard** in
  `.github/workflows/release.yml` (issue #94): the
  `publish-cratesio` job now reads `cargo metadata` and verifies
  every internal `voxora-*` dependency of the tagged crate is
  already on crates.io at the required semver before publishing.
  Fails fast with an explicit "publish voxora-X first" `::error`
  annotation if not.
- **Badge attributes**: feature-gated public items now carry
  `#[cfg_attr(docsrs, doc(cfg(feature = "...")))]` so docs.rs
  renders the "Available on crate feature X" badge (no-op on
  stable rustdoc; the nightly `doc_cfg` feature gates the
  attribute).

### Fixed
- **`clippy::collapsible_if`** in 9 sites across the workspace.
  Pre-existing on `main` but hidden from CI by stale
  `Swatinem/rust-cache@v2` cache.
- **Stale doc references** in `docs/ROADMAP.md` and
  `CONTRIBUTING.md` (the MSRV was still cited as 1.85; an
  example tag referenced the removed `voxora-core` shim).
- **`[workspace.dependencies]` drift** at the root `Cargo.toml`:
  the three crates that landed 0.4.1 at `ecbfd2e` declared
  `version = "0.4.1"` in their own manifests but the workspace
  pins still read 0.4.0. Aligned to 0.4.2 in this release.

### Changed
- `src/testing.rs`: `use voxora_traits::{...}` block unwrapped
  from multi-line to single-line form (now 100 chars on one
  line). This is the documented side effect of the workspace
  adopting `reorder_imports = false` in `rustfmt.toml` (issue #77):
  rustfmt 1.9's import layout changes from "Mixed" to
  "Horizontal" once reordering is disabled.

## [0.4.0] — 2026-09-03

### Changed
- Bumped to 0.4.0 as part of the Fase 4 coordinated release
  (issue #42). The `voxora-core` shim is removed in this
  release; this crate consumes the trait surface via
  `voxora-traits` directly. No API change. The bump continues
  the coordinated version policy introduced for the 0.3.0
  release (see `AGENTS.md` → Version coordination).

## [0.3.0] — 2026-09-03

### Changed
- Version bumped to 0.3.0 as a coordinated version-pin alignment
  with the rest of the workspace (closes #41). No functional or
  API change; the `as_streaming_engine()` default-method
  introduced during the 0.2.0 cycle remains a SemVer-minor
  addition. The bump keeps `voxora-engine` in lock-step with
  `voxora-core` / `voxora-traits` / `voxora-backend` /
  `voxora-bridge` / `voxora-cli` / `voxora-testkit`, all of
  which are already at 0.3.0 — restoring the "coordinated
  bumps by default" rule that the 0.3.0 release silently broke
  for this crate. See `AGENTS.md` → Version coordination.

## [0.2.0] — 2026-09-03

### Changed
- Bumped to 0.2.0 as part of the Fase 2 coordinated release. No
  functional change; this release aligns the version with the
  breaking changes in `voxora-core` / `voxora-hf` / `voxora-whisper`
  / `voxora-qwen3asr` / `voxora-bridge` / `voxora-cli`.

## [0.1.2] — 2026-09-03

### Added
- New crate. `EngineAdapter` trait exposes engine identity
  (`family`, `info`, `backend`) alongside the `AsrEngine` surface.
- `AnyEngine` wrapper holds any concrete adapter as
  `Arc<dyn EngineAdapter + Send + Sync>` and forwards `AsrEngine`
  calls.
- `EngineFamily` enum (`Whisper`, `Qwen3Asr`, ...) with
  `from_config` / `as_config` / `crate_label` helpers for TOML and
  CLI plumbing.
- `BackendDescriptor` and `EngineInfo` structs that the registry
  uses to describe resolved models without lex-sort heuristics.
- `testing::Mock` helper for adapter tests (consumed by
  `voxora-testkit`).