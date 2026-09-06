# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] — 2026-09-05

Coordinated minor release for [EPIC #117](https://github.com/airvzxf/voxora/issues/117).
Every workspace crate that participates in this release ships at
0.5.0 per `AGENTS.md` § "Version coordination". No public API
change for `voxora-registry` itself; this release adds the
`registry_resolve` example (closes #57) and a cross-reference to
`docs/GPU_SUPPORT.md` from the per-crate README (closes #55).

### Added
- **`registry_resolve` example** (closes #57): builds a
  `Registry` with both built-in descriptors and resolves a model
  id, printing the descriptor and on-disk
  `voxora_traits::ModelDir` the resolver chose. Run with
  `cargo run --example registry_resolve -p voxora-registry -- <hf-model-id>`.
- **`docs/GPU_SUPPORT.md` link** from this crate's README
  (closes #55): documents that model resolution is engine-agnostic
  and that the hardware backend is selected downstream by the
  engine adapter.

### Notes
- `voxora-local` is a new workspace member at 0.5.0 (closes #49)
  but `voxora-registry` does NOT depend on it. Consumers wanting
  a "local-first, HF-on-miss" chain should construct a
  `voxora_local::ChainedSource` and pass it to `Registry::new`
  rather than relying on the built-in descriptors (which today
  only accept HF ids). A follow-up issue tracks closing the loop.

## [0.4.3] — 2026-09-06

Coordinated patch release for [EPIC #109](https://github.com/airvzxf/voxora/issues/109)
(PR [#115](https://github.com/airvzxf/voxora/pull/115)). All 11
workspace crates ship at 0.4.3. No public API change, no SemVer
break. Per `AGENTS.md` § "Version coordination".

### Security
- **Path-traversal in `ModelId::parse`** (issue #102): the
  3-segment HF id arm now rejects `.`, `..`, embedded
  `\`, and NUL bytes in the file component. The previous
  parser only checked for empty segments and spaces, so
  `ModelId::parse("foo/bar/..")` returned `Ok` with
  `path = Some(vec![".."])`. The actual write was already
  blocked downstream by `voxora-hf/src/api.rs`'s
  `filename.contains("..")` check, but the parser contract
  was "a validated identifier" — defense in depth closes
  the gap for future code that consumes `ModelId::path`
  directly. The `.hidden` reject in the original recipe
  is deliberately OMITTED: HF permits dot-prefixed
  filenames (e.g. `.gitattributes`).
- **CI supply-chain pin** (issue #104, workspace-wide):
  `Swatinem/rust-cache@v2` is now SHA-pinned in
  `.github/workflows/ci.yml`.

### Fixed
- **Cargo.toml header drift**: stale "stay at 0.4.0"
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

## [0.4.1] — 2026-09-04

Docs-only patch. Per AGENTS.md "Version coordination", the
coordinated-bump rule applies only to X.Y.0 breaking releases;
the other eight workspace crates stay at 0.4.0. No API change.

### Fixed
- Code-spanned the feature-gated intra-doc link in the
  module-level "Phase 0 surface" doc block of `src/lib.rs` that
  failed
  `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p voxora-registry --no-default-features`
  (issue #88):
  - `[`RegistryHfExt`]` → `` `RegistryHfExt` `` (the
    `RegistryHfExt` re-export and the `hf` module are both
    behind the `hf` feature, so the link resolves only when
    the feature is on).

## [0.4.0] — 2026-09-03

### Changed
- First coordinated-release participation for this crate
  (issue #42). Skipped 0.3.0 because the central model
  resolver had no Fase-3 surface change. Now aligned with the
  rest of the workspace at 0.4.0 per the policy in
  `AGENTS.md` → Version coordination. No API change. The
  `voxora-core` shim is removed; this crate now consumes the
  trait surface via `voxora-traits` directly.

## [0.2.0] — 2026-09-03

### Changed
- Bumped to 0.2.0 as part of the Fase 2 coordinated release. No
  functional change; this release aligns the version with the
  breaking changes in `voxora-core` / `voxora-hf` / `voxora-whisper`
  / `voxora-qwen3asr` / `voxora-bridge` / `voxora-cli`.

## [0.1.2] — 2026-09-03

### Added
- New crate. `Registry::with_builtin_descriptors()` ships a default
  registry populated with the canonical Qwen3-ASR and Whisper
  descriptors, resolved through the voxora-hf cascade when the `hf`
  feature is on (default).

### Fixed
- The builtin Qwen3-ASR descriptor accepts `-suffix` siblings
  (`Qwen/Qwen3-ASR-0.6B` / `-1.7B` / `-2.0B` / `-old`) so every
  official Qwen3-ASR release resolves to the same engine family.

## [0.1.0] — 2026-07-12

Initial published release (central model registry, builtin
descriptors, HF adapter behind the `hf` feature).