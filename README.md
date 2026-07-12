# voxora

**Model-agnostic Speech-to-Text for Rust.**

A candle-native bridge that unifies Whisper, Qwen3-ASR, and future Hugging Face
audio models behind one trait, so any Rust application can swap engines
without touching inference code.

> **Status**: pre-alpha / engine adapters shipped. The CLI is in
> `voxora-cli/`. The investigation
> recap and the phased roadmap are in [`docs/`](docs/).

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

## Status and roadmap

The phased plan is in [`docs/ROADMAP.md`](docs/ROADMAP.md). The short
version:

| Phase | Goal | State |
|---|---|---|
| 0 | Repo scaffolding, docs | **done** |
| 1 | `voxora-core` trait + types | done |
| 2 | `voxora-hf` HF model resolver | done |
| 3 | `voxora-whisper` engine adapter | done |
| 4 | `voxora-qwen3asr` engine adapter | done |
| 5 | `voxora-cli` (list / download / run) | done |
| 6 | Telora integration | pending |

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