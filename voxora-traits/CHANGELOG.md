# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
