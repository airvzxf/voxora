# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
