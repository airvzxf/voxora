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

### Added
- **`benches/transcribe_wav.rs`** (criterion stub): an end-to-end
  Qwen3-ASR RTF harness scaffolded against the
  `voxora-testkit::fixtures` loader. `#[ignore]`-gated; the actual
  measurements require real Qwen3-ASR weights and a CUDA-capable
  runner. The compile-only check lands in this release as part of
  the workspace-wide bench lane that closes #56.

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
## [0.4.3] — 2026-09-06

Coordinated patch release for [EPIC #109](https://github.com/airvzxf/voxora/issues/109)
(PR [#115](https://github.com/airvzxf/voxora/pull/115)). All 11
workspace crates ship at 0.4.3. No public API change, no SemVer
break. Per `AGENTS.md` § "Version coordination".

### Security
- **CI supply-chain pin** (issue #104, workspace-wide):
  `Swatinem/rust-cache@v2` is now SHA-pinned in
  `.github/workflows/ci.yml`.

### Fixed
- **`transcribe_wav_qwen3asr` example WAV decode** (F2
  finding): see the matching entry in `voxora-bridge`'s
  CHANGELOG. Inlined a self-contained bit-depth-aware
  decoder mirroring `voxora-cli/src/audio.rs`.
- **Cargo.toml header drift**: stale "currently 0.4.0"
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
- `src/engine.rs`: `use voxora_traits::{...}` block unwrapped
  from multi-line to single-line form (now 100 chars on one
  line). This is the documented side effect of the workspace
  adopting `reorder_imports = false` in `rustfmt.toml` (issue #77).

## [0.4.0] — 2026-09-03

### Changed
- First coordinated-release participation for this crate
  (issue #42). Skipped 0.3.0 because the qwen3-asr adapter
  had no Fase-3 surface change. Now aligned with the rest of
  the workspace at 0.4.0 per the policy in `AGENTS.md` →
  Version coordination. No API change. The `voxora-core`
  shim is removed; this engine adapter consumes the trait
  surface via `voxora-traits` directly.

## [0.2.0] — 2026-09-03

### Added
- Opt-in `engine-adapter` Cargo feature that adopts
  `voxora_engine::EngineAdapter` so introspectable adapters can be
  composed in `AnyEngine` wrappers (PR A).

### Changed
- Engine crate now carries an opt-in dependency on `voxora-engine`
  via the `engine-adapter` feature. The default feature set is
  unchanged.

## [0.1.0] — 2026-07-12

Initial published release (`QwenAsrEngine` over `qwen3-asr-rs`,
20-language validation list, `from_hf` behind the `hf` feature,
`#[ignore]`-gated parity + concurrency tests).

### Fixed
- `from_hf` synthesises a `tokenizer.json` from
  `vocab.json` + `merges.txt` + `tokenizer_config.json` so the
  official Qwen3-ASR HF release loads without manual packaging.