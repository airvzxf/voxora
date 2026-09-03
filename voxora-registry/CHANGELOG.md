# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] — 2026-09-03

### Added
- `Registry::with_builtin_descriptors()` ships a default registry
  populated with the canonical Qwen3-ASR and Whisper descriptors,
  resolved through the voxora-hf cascade when the `hf` feature is
  on (default).

### Fixed
- The builtin Qwen3-ASR descriptor now accepts `-suffix` siblings
  (`Qwen/Qwen3-ASR-0.6B` / `-1.7B` / `-2.0B` / `-old`) so every
  official Qwen3-ASR release resolves to the same engine family
  (PR B).

## [0.1.0] — 2026-07-12

Initial published release (central model registry, builtin
descriptors, HF adapter behind the `hf` feature).