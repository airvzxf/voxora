# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.4] — 2026-09-06

Initial published release (issue
[#49](https://github.com/airvzxf/voxora/issues/49)).

### Added
- `LocalSource` — `voxora_traits::ModelSource` impl that resolves
  model ids against an already-vendored directory on disk. No
  network, no tokio, no reqwest, no `voxora-config`. `resolve`
  joins `model_id` against the configured root and returns a
  `ModelDir` with `entry` set; missing files surface as
  `AsrError::ModelNotFound` naming both the missing path and the
  configured root.
- `ChainedSource` — first-hit-wins adapter that tries a primary
  source and falls back to a secondary source on
  `AsrError::ModelNotFound`. Honours the spirit of the issue's
  "local first, Hugging Face on miss" claim without forcing a
  `voxora-registry` refactor.

### Notes
- Documented limitation: `voxora-registry`'s built-in descriptors
  do not yet accept `SourceKind::Local` ids; consumers wanting a
  fallback chain should pass a `ChainedSource` directly to
  `Registry::new` rather than relying on the descriptor accept
  arm. A follow-up issue tracks closing the loop.
- Coordinated workspace bump: per `AGENTS.md` § Version
  coordination, the new crate ships at 0.4.4 to align with the
  additive-change exception. The other 10 workspace crates stay at
  0.4.3; consumers depending on `voxora-local` should specify
  `voxora-local = "0.4.4"` directly.
