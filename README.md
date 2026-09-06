# voxora

**Model-agnostic Speech-to-Text for Rust.**

A candle-native bridge that unifies Whisper, Qwen3-ASR, and future Hugging Face
audio models behind one trait, so any Rust application can swap engines
without touching inference code.

> **Status**: pre-alpha / engine adapters shipped. The CLI is in
> `voxora-cli/`. The investigation
> recap and the phased roadmap are in [`docs/`](docs/). The
> cross-engine hardware / GPU support matrix is in
> [`docs/GPU_SUPPORT.md`](docs/GPU_SUPPORT.md).

---

## Why voxora?

The name is a Latin portmanteau, in the same construction style as
[Telora](https://github.com/airvzxf/telora):

| Root | Language | Meaning |
|---|---|---|
| **vox** | Latin | voice |
| **ora** | Latin | mouth, speech, utterance |

`vox + ora = voxora` — "voice by mouth" — a name that describes what a
speech-to-text engine does (turn voice into uttered text) and that sounds
right when spoken, because it ends in the open vowel **ah**, the sound
produced by opening the mouth to speak.

It was chosen from a curated list of ~60 candidates across Latin, Greek,
and modern coinages; the criteria were:

- Free on both crates.io and GitHub (`airvzxf/voxora`).
- Phonetically pronounceable in Spanish and English.
- Mirrors the construction of Telora (tele + ora), so the brand family reads.
- No hyphens (ergonomics in `use voxora::...`), no `-rs` suffix
  (convention for native Rust crates), no domain suffix like `-asr`
  (the name describes the brand, not the function).

## Why this project exists

Telora is a local Speech-to-Text assistant for Linux, written in Rust and
originally bound to `whisper-rs`. As the maintainer explored alternative
models (specifically Qwen3-ASR, via
[huggingface/candle#3509](https://github.com/huggingface/candle/pull/3509)
and the [qwen3-asr-rs](https://github.com/airvzxf/qwen3-asr-rs) fork),
two problems surfaced:

1. **candle's maintainers prioritize NVIDIA-native optimization over new
   model architectures**. The Qwen3-ASR PR has been waiting for review
   for months despite CPU/GPU benchmarks that match or beat PyTorch on
   RTX 3090.
2. **The Rust ASR ecosystem is split between ONNX Runtime and whisper.cpp**.
   [transcribe-rs](https://github.com/cjpais/transcribe-rs) (233⭐)
   already provides a `SpeechModel` trait that unifies nine engines,
   but every engine except whisper.cpp goes through ORT — never candle.

**voxora** is the candle-native sibling: one trait that wraps
candle-native inference engines (Qwen3-ASR today, Voxtral/Granite-Speech
tomorrow), with auto-resolution from Hugging Face and a hardware
dispatcher (CUDA → Metal → CPU) that lets downstream apps pick the
best available accelerator at runtime.

Telora will eventually depend on `voxora` instead of `whisper-rs`
directly, becoming model-agnostic the way the maintainer originally
envisioned.

## How it fits in the stack

```
┌─────────────────────────────────────────────────────────────┐
│ Applications: Telora today, any future Rust STT consumer     │
└────────────────────────┬────────────────────────────────────┘
                         │ depends on
                         ▼
┌─────────────────────────────────────────────────────────────┐
│ **voxora** — the candle-native ASR bridge                   │
│   • AsrEngine trait, TranscribeOptions, TranscriptionResult │
│   • Hugging Face model resolution + quantization selection │
│   • Per-engine adapters (whisper-rs, qwen3-asr-rs, …)       │
│   • Hardware dispatcher: CUDA → Metal → CPU                 │
└────────────────────────┬────────────────────────────────────┘
                         │ depends on
        ┌────────────────┼────────────────┐
        ▼                ▼                ▼
   whisper-rs     qwen3-asr-rs     future engines
   (whisper.cpp   (candle, via     (Voxtral, Granite-
    bindings)      airvzxf fork)    Speech, Parakeet…)
```

The hardware backend each engine ships today (CUDA / Metal /
Vulkan / CPU) is documented per-engine in
[`docs/GPU_SUPPORT.md`](docs/GPU_SUPPORT.md).

## Crates in this workspace

The workspace is a set of small crates that together form the
bridge. The first three rows are the public-API crates a downstream
consumer most commonly depends on; the rest are the implementation
crates behind them. Pick a row, follow the docs.rs link for the
API reference; the publishable crates also ship per-crate
example binaries under `voxora-{name}/examples/` (e.g.
`cargo run --example transcribe_wav_whisper -p voxora-whisper`,
`cargo run --example basic_transcribe -p voxora-bridge --features voxora-bridge/whisper`,
`cargo run --example registry_resolve -p voxora-registry`).

| Crate | crates.io | docs.rs | Role |
|-------|-----------|---------|------|
| `voxora-traits` | [crates.io](https://crates.io/crates/voxora-traits) | [docs.rs](https://docs.rs/voxora-traits) | Canonical traits (`AsrEngine`, `ModelSource`) |
| `voxora-engine` | [crates.io](https://crates.io/crates/voxora-engine) | [docs.rs](https://docs.rs/voxora-engine) | Adapter contract (`EngineAdapter`, `EngineFamily`) |
| `voxora-bridge` | [crates.io](https://crates.io/crates/voxora-bridge) | [docs.rs](https://docs.rs/voxora-bridge) | Umbrella crate — re-exports traits + engines behind feature flags |
| `voxora-hf` | [crates.io](https://crates.io/crates/voxora-hf) | [docs.rs](https://docs.rs/voxora-hf) | Hugging Face resolver |
| `voxora-whisper` | [crates.io](https://crates.io/crates/voxora-whisper) | [docs.rs](https://docs.rs/voxora-whisper) | whisper.cpp adapter (`whisper-rs` binding) |
| `voxora-qwen3asr` | [crates.io](https://crates.io/crates/voxora-qwen3asr) | [docs.rs](https://docs.rs/voxora-qwen3asr) | Qwen3-ASR adapter (`qwen3-asr-rs` binding) |
| `voxora-registry` | [crates.io](https://crates.io/crates/voxora-registry) | [docs.rs](https://docs.rs/voxora-registry) | Central model resolver |
| `voxora-backend` | [crates.io](https://crates.io/crates/voxora-backend) | [docs.rs](https://docs.rs/voxora-backend) | Hardware backend selection (CPU / Metal / CUDA) |
| `voxora-config` | [crates.io](https://crates.io/crates/voxora-config) | [docs.rs](https://docs.rs/voxora-config) | Env-var cascade (cache dir, HF token) |
| `voxora-cli` | build only | n/a | CLI binary (`voxora list` / `download` / `run`); `publish = false` |
| `voxora-testkit` | dev-only | n/a | Shared fixtures and mocks; `publish = false` |
| `voxora-core` | (removed) | (removed) | Deprecated shim around `voxora-traits` (last release 0.3.1); removed in 0.4.0 |

## Status and roadmap

The phased plan is in [`docs/ROADMAP.md`](docs/ROADMAP.md). The short
version:

| Phase | Goal | State |
|---|---|---|
| 0 | Repo scaffolding, docs | **done** |
| 1 | `voxora-traits` trait + types (originally `voxora-core`; split out in 0.3.0) | done |
| 2 | `voxora-hf` HF model resolver | done |
| 3 | `voxora-whisper` engine adapter | done |
| 4 | `voxora-qwen3asr` engine adapter | done |
| 5 | `voxora-cli` (list / download / run) | done |
| 6 | Telora integration | pending |

## Coordinated releases

voxora publishes a **unified version** across every workspace crate
that participates in a release. When `voxora X.Y.0` ships, all
participating crates ship at `X.Y.0` — you can depend on them with
matching versions and trust they stay in lockstep:

```toml
voxora-traits   = "X.Y.0"
voxora-engine  = "X.Y.0"
voxora-hf      = "X.Y.0"
voxora-whisper = "X.Y.0"
```

Picking a single version `X.Y.0` and using it across the voxora-*
crates in your `Cargo.toml` is the supported way to consume voxora.
If you depend on, say, `voxora-traits = "X.Y.0"` and
`voxora-engine = "X.Y.0"`, you can be confident they were released
together and that their public surfaces are wire-compatible. No need
to read per-crate changelogs to figure out which combinations of
minor versions are compatible — the workspace version *is* the
compatibility promise.

The full invariant (including the additive-change exception) is
documented in [`AGENTS.md` → Version coordination](AGENTS.md#version-coordination).

## Upgrade guide — 0.3.x → 0.4.0

`voxora-core` was removed in 0.4.0; depend on `voxora-traits`
directly. The trait surface (`AsrEngine`, `TranscribeOptions`,
`ModelSource`, etc.) has lived in `voxora-traits` since 0.3.0; the
shim crate only existed to ease the 0.3.x transition.

In your `Cargo.toml`, swap `voxora-core = "0.3"` for
`voxora-traits = "0.4"`:

```toml
# Before (0.3.x)
voxora-core = "0.3"

# After (0.4.0)
voxora-traits = "0.4"
```

If you used the `voxora-core/serde` Cargo feature, switch to the
matching `voxora-traits/serde` feature.

In your source, replace `use voxora_core::*;` with the matching
`voxora-traits` import:

```rust
// 0.3.x
use voxora_core::AsrEngine;
use voxora_core::TranscribeOptions;

// 0.4.0
use voxora_traits::AsrEngine;
use voxora_traits::TranscribeOptions;
```

A single `cargo update` against every participating crate brings
the whole workspace to 0.4.0 in lockstep:

```text
cargo update -p voxora-traits   \
            -p voxora-config   \
            -p voxora-hf       \
            -p voxora-whisper  \
            -p voxora-qwen3asr \
            -p voxora-engine   \
            -p voxora-backend  \
            -p voxora-registry \
            -p voxora-bridge
```

The `voxora-bridge` umbrella crate is unaffected by the removal
and continues to re-export `voxora-traits` for one-stop
consumption. Per-crate notes on what changed in 0.4.0 are in each
crate's `CHANGELOG.md`:

- [`voxora-traits/CHANGELOG.md`](voxora-traits/CHANGELOG.md)
- [`voxora-config/CHANGELOG.md`](voxora-config/CHANGELOG.md)
- [`voxora-hf/CHANGELOG.md`](voxora-hf/CHANGELOG.md)
- [`voxora-whisper/CHANGELOG.md`](voxora-whisper/CHANGELOG.md)
- [`voxora-qwen3asr/CHANGELOG.md`](voxora-qwen3asr/CHANGELOG.md)
- [`voxora-engine/CHANGELOG.md`](voxora-engine/CHANGELOG.md)
- [`voxora-backend/CHANGELOG.md`](voxora-backend/CHANGELOG.md)
- [`voxora-registry/CHANGELOG.md`](voxora-registry/CHANGELOG.md)
- [`voxora-bridge/CHANGELOG.md`](voxora-bridge/CHANGELOG.md)

## Quickstart

```text
# Build:
cargo build --release -p voxora-cli

# Download a model:
./target/release/voxora download Qwen/Qwen3-ASR-0.6B

# Transcribe a WAV (engine auto-detected from config.json):
./target/release/voxora run Qwen/Qwen3-ASR-0.6B samples/jfk.wav

# Or pin a specific engine:
./target/release/voxora run ggerganov/whisper.cpp samples/jfk.wav \
    --engine whisper --language en
```

See `voxora --help` for the full surface. Engine selection falls
back to a `--engine <whisper|qwen3-asr>` override when `config.json`
doesn't disambiguate. Hardware flags mirror the engines
(`--features cpu` (default), `metal`, `cuda`).


## Investigation

Why this repo exists, the gap it fills, and the options we considered
are documented in [`docs/INVESTIGATION.md`](docs/INVESTIGATION.md).
Read it before opening an issue — most "why not just X?" questions are
answered there.

## License

Apache License, Version 2.0. See [`LICENSE`](LICENSE).

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) and our
[Code of Conduct](CODE_OF_CONDUCT.md).