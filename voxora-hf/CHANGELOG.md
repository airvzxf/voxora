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
- **Concurrent-download tmp suffix uniqueness** (issue #103):
  `HfClient::get_to_file` now uses a per-process
  `AtomicU64` counter (`DOWNLOAD_COUNTER`,
  `Ordering::Relaxed`) for the per-call tmp-file suffix. The
  previous `SystemTime::now().as_nanos()` produced identical
  suffixes when two `try_join_all` shards sampled the clock
  in the same nanosecond; the second call's
  `tokio::fs::File::create` (`O_CREAT|O_TRUNC`) truncated the
  first call's in-flight bytes and the two `write_all`
  streams interleaved. The race fires today in sharded
  model downloads.
- **Path-traversal in `split_three_segment_id`** (issue #102):
  the HF single-file segment parser now rejects `.`, `..`,
  embedded `\`, and NUL bytes in the file component. The
  existing downstream `api.rs` `filename.contains("..")`
  check stays as the last-line defense.
- **CI supply-chain pin** (issue #104, workspace-wide): the
  six `Swatinem/rust-cache@v2` references in
  `.github/workflows/ci.yml` are now SHA-pinned to
  `6323deb102c322ba6fcbdcafc7e3dddab59af2b6` (the commit
  the v2.9.2 tag peels to) with a `# v2.9.2` version comment.

### Fixed
- **`HfError::Transport` URL diagnostic** (F2 finding): the
  URL field of the Transport variant now carries the
  request URL instead of `String::new()`. Threaded through
  `HfClient::execute`. The bearer token is set via the
  `Authorization` header (reqwest, not the URL), so the URL
  is safe to surface.
- **UTF-8 panic in error body truncation** (F2 finding):
  `voxora-hf/src/error.rs::truncate` walked the string by
  char boundary instead of slicing `s[..max]` directly, so
  HTTP response bodies containing non-ASCII characters
  (Chinese / Japanese / Korean proxy errors) no longer
  panic with "byte index N is not a char boundary". Adds
  three unit tests.
- **`cleanup_partials` no longer orphans tmp files**
  (F2 finding): the matcher now checks for the substring
  `.partial` in the file name (catches both the legacy
  `<file>.partial` shape and the current
  `<file>.<ext>.partial.<hex>-<n>` shape). The single-file
  resolve path (`resolve_single_file`) now also calls
  `cleanup_partials` before `mark_complete`, mirroring the
  whole-repo path. A process crash mid-download no longer
  leaves orphans on disk.
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

## [0.4.0] — 2026-09-03

### Changed
- First coordinated-release participation for this crate
  (issue #42). Skipped 0.3.0 because the HF resolver had no
  Fase-3 surface change. Now aligned with the rest of the
  workspace at 0.4.0 per the policy in `AGENTS.md` →
  Version coordination. No API change. The `voxora-core` shim
  is removed in this release; this crate now consumes the
  trait surface via `voxora-traits` directly.

## [0.2.0] — 2026-09-03

### Added
- `config` Cargo feature (now the default) that pulls in
  `voxora-config` and uses it as the single source of truth for the
  env-var cascade: cache root, Hugging Face token, HF base URL,
  default revision, and TOML overrides.

### Changed
- The default feature set now enables `config`; downstream consumers
  that need the pre-0.2.0 inline cascade (read-only `VOXORA_CACHE_DIR`,
  `HF_TOKEN`, `HUGGING_FACE_HUB_TOKEN`) must opt out with
  `default-features = false`.

## [0.1.0] — 2026-07-12

Initial published release (`HuggingFaceSource: ModelSource`,
wiremock integration tests, `#[ignore]`-gated live smoke test).