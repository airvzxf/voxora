# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.3] — 2026-09-06

Coordinated patch release for
[EPIC #148](https://github.com/airvzxf/voxora/issues/148)
(closes #143, #144). The 3 participating crates
(`voxora-traits` 0.5.3, `voxora-local` 0.5.2,
`voxora-registry` 0.5.4) ship at this coordinated set; the
remaining 8 stay at their current versions per `AGENTS.md` §
"Version coordination".

### Added
- **`ResolveOptions::max_bytes` and `max_id_length`** (closes
  #143, #144, EPIC #148). Two `Option`-wrapped caps that
  callers can set to bound the byte size of resolved files and
  the length of model ids. `LocalSource` honours both:
  `max_bytes` rejects a resolved regular file whose
  `metadata().len()` exceeds the cap (closes the unbounded-
  read TOCTOU in #144), and `max_id_length` tightens the
  intrinsic 4 KiB model-id cap when the caller knows a smaller
  ceiling is appropriate. `HuggingFaceSource` honours
  `max_bytes` for streamed downloads; `max_id_length` is
  ignored there (the HF parser already enforces segment shape
  upstream). `#[non_exhaustive]` on `ResolveOptions` keeps the
  addition non-breaking; existing call sites using
  `ResolveOptions::default()` are unaffected (the new fields
  default to `None`). Two new helper constructors —
  `ResolveOptions::with_max_bytes(u64)` and
  `ResolveOptions::with_max_id_length(usize)` — mirror the
  existing `with_revision` / `with_token` style so consumers
  do not need to construct the struct manually.

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
0.5.0 per `AGENTS.md` § "Version coordination". **No public API
change** for `voxora-traits` itself — the trait surface
(`AsrEngine`, `ModelSource`, `StreamingAsrEngine`, the value
types, and `AsrError`) is unchanged from 0.4.x. This is a
purely additive coordinated bump: the workspace pins the
version so consumers writing
`voxora-traits = "0.5", voxora-bridge = "0.5", voxora-local = "0.5", voxora-vad = "0.5"`
get the matching set.

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
