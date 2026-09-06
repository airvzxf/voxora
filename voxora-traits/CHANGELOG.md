# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.3] — 2026-09-06

Coordinated patch release for [EPIC #109](https://github.com/airvzxf/voxora/issues/109)
(PR [#115](https://github.com/airvzxf/voxora/pull/115)). All 11
workspace crates ship at 0.4.3. No public API change, no SemVer
break. Per `AGENTS.md` § "Version coordination".

### Security
- **CI supply-chain pin** (issue #104): the six
  `Swatinem/rust-cache@v2` references in
  `.github/workflows/ci.yml` are now SHA-pinned to
  `6323deb102c322ba6fcbdcafc7e3dddab59af2b6` (the commit the
  v2.9.2 tag peels to) with a `# v2.9.2` version comment,
  matching the pin convention used for every other
  third-party action in the workflow. The cache step had
  write access to `~/.cargo` and `target/`; the moving
  major-tag was a meaningful retag vector.

### Fixed
- **Cargo.toml header drift**: 11 member `Cargo.toml` files
  had stale "currently 0.4.0" / "stay at 0.4.0" comments
  from the 0.4.1 partial release. All updated to 0.4.2
  (the post-EPIC-#100 version this 0.4.3 release supersedes).

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
- Resolved three pre-existing `cargo doc --no-deps --workspace`
  unresolved-link warnings in `src/streaming.rs`
  (issue #74; this block was previously stranded under
  `[Unreleased]` after c080c1b landed past the v0.4.0 tags):
  - `StreamingAsrEngine::transcribe_chunk` → `StreamingSession::transcribe_chunk`
    at the `StreamingOptions` and `StreamingResult` doc headers
    (the `transcribe_chunk` method is defined on
    `StreamingSession`, not on the `StreamingAsrEngine` trait).
  - `AsrEngine::transcribe` → `crate::AsrEngine::transcribe` at
    the `StreamingAsrEngine` trait doc (uses the crate-root
    re-export rather than a path that only resolves from the
    module-level docstring).
- Corrected the `StreamingSession` doc comment in
  `src/streaming.rs` to state the actual contract: plain
  `#[async_trait]` (without `?Send`) makes the futures returned
  by `transcribe_chunk` and `finalize_stream` `Future + Send`,
  so any state held in an implementor must itself be `Send`
  (issue #89). Relaxing to `#[async_trait(?Send)]` is tracked
  for the streaming engine adoption in issues #50 and #51.
  Callers still drive a single session from one thread at a
  time, and `StreamingAsrEngine` itself remains `Send + Sync`.

## [0.4.0] — 2026-09-03

### Changed
- Bumped to 0.4.0 as part of the Fase 4 coordinated release
  (issue #42). The `voxora-core` shim that re-exported this
  crate in 0.3.0 / 0.3.1 is removed in this release. The trait
  surface has lived here since voxora 0.3.0; downstream code
  that still imported `voxora_core::*` must now import
  `voxora_traits::*` (the workspace itself completed that
  migration in PR #66). The 0.4.0 cut is the coordinated
  release that aligns every voxora-* crate at the same
  version per the policy in `AGENTS.md` → Version coordination.

### Notes
- The voxora workspace migrated its internal `voxora_core::*`
  imports to `voxora_traits::*` ahead of the voxora-core 0.3.1
  deprecation warning (issue #44), as recorded in the PR #66
  commit message. This entry exists only so the audit trail
  shows the migration happened. No API change.

## [0.3.0] — 2026-09-03

### Added
- Initial published release. This crate is the canonical home
  of the `voxora` public API surface, split out of pre-3.0
  `voxora-core` as part of Fase 3 PR A.
- `AsrEngine` trait (synchronous, `Send + Sync` inference
  contract) and `ModelSource` trait (`async_trait`,
  `Send + Sync`, model acquisition contract).
- Inference value types: `ModelCapabilities`, `TranscribeOptions`,
  `TranscriptionResult`, `TranscriptionSegment`.
- Source value types: `ModelDescriptor`, `ModelDir`,
  `ModelSourceKind`, `Quantization`, `QuantizationPreference`,
  `ResolveOptions`.
- Error type: `AsrError`.
- Streaming extension trait: `StreamingAsrEngine`,
  `StreamingOptions`, `StreamingResult`, `StreamingSession`
  (added in PR D; default-implemented on `AsrEngine`).
- Optional `serde` Cargo feature for `Serialize`/`Deserialize`
  derives on the public value types.

### Notes
- The same trait surface previously shipped as `voxora-core`
  `0.1.0` and `0.2.0`. `voxora-core` `0.3.0` is a thin
  compatibility shim that re-exports this crate; new code
  should depend on `voxora-traits` directly.
