# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.3] — 2026-09-07

Additive patch release for [issue #145](https://github.com/airvzxf/voxora/issues/145).
No public API change, no SemVer break. The umbrella crate picks
up a new opt-in `registry` Cargo feature that re-exports
`voxora-registry`'s surface; per `AGENTS.md` § "Version
coordination", the bump is `voxora-bridge`-only because the
change is purely additive and confined to this crate's
re-export surface.

### Added
- **`registry` Cargo feature (closes #145)** — re-exports
  `voxora_registry::{Registry, RegistryHfExt,
  builtin_local_descriptor}` at the umbrella level so
  `voxora-bridge` consumers can build a
  `Registry::with_builtin_descriptors_and_chained_source` chain
  (the helper shipped in `voxora-registry 0.5.3`) without adding
  `voxora-registry` as a direct dependency. NOT in `default` —
  the slim `whisper` + `qwen3asr` build stays slim.
  `dep:voxora-registry` keeps the transitive footprint zero when
  this feature is off. `voxora-registry`'s own `hf` default
  feature comes along automatically (the umbrella's `local`
  feature does NOT pull `voxora-registry` transitively), so
  `RegistryHfExt::with_builtin_descriptors` is reachable from a
  consumer that turns on `voxora-bridge/registry` alone.
- **`registry_resolve_local` example** — end-to-end smoke for
  the new feature, gated on `required-features = ["registry",
  "whisper"]`. Run with
  `cargo run --example registry_resolve_local -p voxora-bridge --features registry,whisper`.
  Mirrors `voxora-registry/examples/registry_resolve.rs` but
  uses bridge-level imports (`voxora_bridge::Registry`,
  `voxora_bridge::RegistryHfExt`,
  `voxora_bridge::HuggingFaceSource`) to demonstrate the
  re-export resolves end-to-end.
- **`registry_reexport` integration test** — compile-only
  assertion that the re-exported symbols (`Registry`,
  `RegistryHfExt`, `builtin_local_descriptor`) resolve at the
  bridge level. Gated on `required-features = ["registry"]`.

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
- **`parity` Cargo feature (new, opt-in)** — gates
  `voxora-bridge/tests/cross_engine_parity.rs`, an end-to-end
  Whisper × Qwen3-ASR WER parity test (`#[ignore]`-d). Pulls
  `whisper` + `qwen3asr` features implicitly. Run with
  `cargo test -p voxora-bridge --test cross_engine_parity --features parity -- --ignored`.
  Closes #59.
- **Internal** — added `voxora_bridge::normalize` module (string
  normalization for the parity WER helper; always compiled, no
  public surface change).

### Notes
- No API change in `voxora-bridge 0.5.3`; only the `registry`
  Cargo feature added (closes #145). No participating-crate bump
  beyond `voxora-bridge` itself — purely additive, scoped to
  the umbrella's re-export surface.

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
0.5.0 per `AGENTS.md` § "Version coordination". Two new crates
(`voxora-local`, `voxora-vad`) join the workspace; this umbrella
crate re-exports `voxora-local` and adopts a new `local` Cargo
feature. The cross-engine hardware matrix is now documented at
[`docs/GPU_SUPPORT.md`](../docs/GPU_SUPPORT.md) and linked from
each per-crate README.

### Added
- **`local` Cargo feature** (closes #49): enables re-export of
  `voxora_local::{LocalSource, ChainedSource}` behind the
  `local = ["dep:voxora-local"]` forward. Not in `default` —
  the canonical happy-path consumer (telora-daemon) does not
  need it; consumers who vendor their own weights opt in
  explicitly. `dep:voxora-local` keeps the transitive footprint
  zero when this feature is off.
- **`basic_transcribe` example** (closes #57): a minimal
  `WhisperEngine::from_hf` + transcribe-on-silence library-API
  smoke, gated on the existing `whisper` feature. Run with
  `cargo run --example basic_transcribe -p voxora-bridge --features voxora-bridge/whisper`.
- **`docs/GPU_SUPPORT.md` link** from this crate's CI matrix
  (closes #55): feature-flag forwarding (`cuda` / `cuda-whisper`
  / `cuda-qwen3asr` / `metal` / `vulkan`) now has a single
  canonical reference for the CUDA sm split and runtime-picker
  guidance.

### Notes
- No public API change for the existing `whisper` / `qwen3asr`
  re-exports. The new `local` feature is purely additive; existing
  `cargo install voxora-bridge` commands keep working unchanged.

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
- **`bridge_demo` example WAV decode** (F2 finding): the
  canonical Telora-style example previously did
  `reader.samples::<i16>()` and divided by `i16::MAX`,
  which hound rejects on 24-bit / 32-bit Int / Float WAVs
  and produces a `[-1, 1]` range that is asymmetric on
  full-scale negative. Inlined a self-contained
  bit-depth-aware decoder (16/24/32-bit Int, 32-bit
  Float, downmix with i64-widened accumulator, symmetric
  `2^(bits-1)` divisor) mirroring `voxora-cli/src/audio.rs`.
  Also use the WAV's declared `sample_rate` for segment
  timestamps instead of a hard-coded 16 kHz.
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
- Code-spanned two feature-gated intra-doc links in the
  module-level "Example: load a Whisper model from Hugging Face"
  doc block of `src/lib.rs` that failed
  `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p voxora-bridge --no-default-features`
  (issue #88):
  - `[`WhisperEngine::from_hf`]` → `` `WhisperEngine::from_hf` ``
  - `[`QwenAsrEngine::from_hf`]` → `` `QwenAsrEngine::from_hf` ``
  - The other bracketed links on the same doc block
    (`HuggingFaceSource`, `AsrEngine::transcribe`) were left
    intact: those re-exports live behind no feature gate.
- Code-spanned the same latent instance of the bug in
  `examples/bridge_demo.rs` (`[`WhisperEngine`]` →
  `` `WhisperEngine` ``). `whisper` is already a default
  feature of this crate, so the example builds under
  `--features` defaults today; the bug is dormant under
  `--no-default-features`. Fixed while in the area so a
  future `--no-default-features` consumer does not hit a
  `--no-default-features` rustdoc failure.

## [0.4.0] — 2026-09-03

### Changed
- Bumped to 0.4.0 as part of the Fase 4 coordinated release
  (issue #42). The umbrella crate now re-exports the trait
  surface from `voxora-traits` exclusively — the `voxora-core`
  re-export that backed this crate in 0.3.0 is removed in
  this release. No API change for downstream consumers that
  used the `voxora_bridge::AsrEngine` style import (those
  resolve to `voxora_traits::AsrEngine` now).

## [0.3.0] — 2026-09-03

### Removed
- **Breaking**: `voxora-bridge::ModelKind` removed (was deprecated
  since 0.2.0). Use `voxora_engine::EngineFamily` (re-exported from
  voxora-bridge).
- **Breaking**: `voxora-bridge::InvalidModelKind` removed (paired with
  `ModelKind`'s `FromStr` impl). Use `voxora_engine::InvalidEngineFamily`.

## [0.2.0] — 2026-09-03

### Deprecated
- `voxora_bridge::ModelKind` is marked
  `#[deprecated(since = "0.2.0")]` in favour of
  `voxora_engine::EngineFamily`. The variant still resolves and
  forwards to the matching engine feature, but new code should
  import the engine-family type from `voxora-engine`.

## [0.1.1] — 2026-07-14

### Changed
- Forwarded `cuda` is now split into `cuda-whisper` /
  `cuda-qwen3asr` so downstream crates on hardware older than
  sm_70 (Volta) can enable `cuda-whisper` only. Added forwarding
  `metal` and `vulkan` features to match.

## [0.1.0] — 2026-07-12

Initial published release (umbrella crate re-exporting
`voxora-core`, `voxora-hf`, and the engine adapters behind
`whisper` / `qwen3asr` feature flags).