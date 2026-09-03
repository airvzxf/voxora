# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] — 2026-09-03

### Added
- New crate. Cross-cutting hardware backend selection so engines
  share a single CUDA → Metal → CPU dispatcher.
- `best_device()` resolves the best available device at process
  start without taking a `Device` at load time (qwen3-asr upstream
  already exposes this; we surface it for the rest of the
  workspace).
- `detect()` returns `Capabilities` describing what is actually
  present (CUDA toolkit, Metal runtime, etc.).
- `best_device_or_error()` is the strict variant for binaries that
  need to fail loudly when no hardware backend is available.