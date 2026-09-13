# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0] — 2026-09-12

Coordinated minor release for [EPIC #153](https://github.com/airvzxf/voxora/issues/153)
(closes [#159](https://github.com/airvzxf/voxora/issues/159)). The 6
participating crates (`voxora-engine`, `voxora-bridge`, `voxora-cli`,
`voxora-config`, `voxora-registry`, and the new `voxora-minimax`)
ship at 0.6.0 per `AGENTS.md` § "Version coordination".

### Added
- **`minimax` Cargo feature** (closes
  [#159](https://github.com/airvzxf/voxora/issues/159)): opt-in
  feature that pulls in `voxora-minimax` and activates the new
  CLI dispatch arm. Not in `default` — the canonical happy-path
  consumer does not need the hosted-API engine.
- **`--engine minimax` dispatch arm**: `voxora run --engine
  minimax <audio.wav>` builds a `MiniMaxEngine` directly from
  the resolved API key, skipping HF resolution entirely. The
  positional `model_id` is accepted but unused (no slash is
  required) so the surface matches the other engines.
- **`--minimax-api-key` global flag** (parallel to `--token`):
  overrides the `VOXORA_MINIMAX_API_KEY` / `MINIMAX_API_KEY`
  env-var cascade at the CLI layer.
- **`--hardware <non-cpu> --engine minimax` fails fast**:
  MiniMax compute happens server-side, so any non-CPU hardware
  flag is meaningless — the new check in
  `engine::ensure_hardware_compatible_with_engine` rejects the
  combination with a clear error message.

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
## [0.4.2] — 2026-09-05

Coordinated patch release for [EPIC #100](https://github.com/airvzxf/voxora/issues/100)
(PR [#105](https://github.com/airvzxf/voxora/pull/105)). All 11
workspace crates ship at 0.4.2 per `AGENTS.md` § "Version
coordination"; `voxora-cli` and `voxora-testkit` were omitted from
the original sweep because they are `publish = false`, but their
workspace version advanced to 0.4.2 in lockstep (closes
[#114](https://github.com/airvzxf/voxora/issues/114)). No public
API change, no SemVer break.

### Changed
- **MSRV bumped from 1.86 to 1.88.** Mirrors the
  `voxora-traits/CHANGELOG.md` rationale; consumers pinned to
  rustc 1.86 or 1.87 will need to update.
- **Cargo.toml header drift**: stale "currently 0.4.0" comment
  updated to 0.4.2 (matches the issue-#109 follow-up).

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
- **Stereo Float WAV downmix test** (issue #101): the
  issue's analysis claimed the Float path's
  `read_frames_float` truncates to half-amplitude because
  the i32 cast happens after the i64-widened sum "was
  never actually widened to i64 in the first place for the
  Float path". That claim is wrong: the Float path widens
  to i64 at `audio.rs:163` and the worst-case error is
  ~1 ULP (5×10⁻⁸), not half-amplitude. The path was
  correct; no source change was needed. Two regression
  tests now lock in the correct behavior so any future
  refactor that reintroduces a real precision bug (e.g.
  dropping the i64 widening on the Float path) fails
  CI.
- **Cargo.toml header drift**: stale "currently 0.4.0"
  comment updated to 0.4.2.

## [0.4.0] — 2026-09-03

### Changed
- Bumped to 0.4.0 as part of the Fase 4 coordinated release
  (issue #42). The `voxora-core` shim is removed; the CLI
  binary now consumes the trait surface via `voxora-traits`
  (the `voxora_engine::EngineFamily` integration introduced
  in 0.3.0 is unchanged). No API change.

## [0.3.0] — 2026-09-03

### Removed
- **Breaking**: `voxora-cli::BackendKind` removed (was deprecated
  since 0.2.0). The CLI now uses `voxora_engine::EngineFamily`
  directly. `--engine whisper|qwen3-asr` parsing is unchanged — the
  same case-insensitive matching and aliases (`qwen3_asr`, `qwen3asr`)
  are accepted via `EngineFamily::from_config`.

## [0.2.0] — 2026-09-03

### Deprecated
- `voxora_cli::BackendKind` is marked
  `#[deprecated(since = "0.2.0")]` in favour of
  `voxora_engine::EngineFamily`. Existing `--engine` flag parsing
  keeps working — `BackendKind::from_cli_label` is independent of
  the canonical `EngineFamily`.

## [0.1.0] — 2026-07-12

Initial release (`voxora list | info | download | run | serve`
subcommands, `cpu` / `metal` / `cuda` hardware features,
`make build-cli` + `make build-musl` artefacts).

### Fixed
- `hf_cache_dir()` honours `XDG_CACHE_HOME` /
  `$HOME/.cache/` via `dirs::cache_dir()` so the CLI's default cache
  root matches the platform convention.
- `decode_wav` honours the declared bit depth and uses
  `2^(bits-1)` as the PCM divisor for 16/24/32-bit WAVs (16-bit
  audio was 65536× too quiet previously).