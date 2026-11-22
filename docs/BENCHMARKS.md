# Benchmarks

All benches use [criterion](https://docs.rs/criterion). CI only
compiles them (`cargo bench --workspace --no-run`), so a bench that
stops building fails the PR. Nothing is measured in CI.

## Offline benches

They need no model and no network:

| Crate | Bench | Measures |
|---|---|---|
| `voxora-engine` | `model_capabilities` | building a `ModelCapabilities` |
| `voxora-registry` | `resolve` | `Registry::resolve` against an in-memory source |

```text
cargo bench -p voxora-engine
cargo bench -p voxora-registry
```

## Real-Time Factor (RTF)

RTF = transcription time / audio duration. Below 1.0 the engine is
faster than real time: 0.25 transcribes 4 s of audio in 1 s.

The engine benches need a model and a speech clip (16 kHz, mono,
16-bit PCM WAV). Both come from the environment. Without them the
bench prints that it was skipped and measures nothing.

| Crate | Variables |
|---|---|
| `voxora-whisper` | `VOXORA_BENCH_WHISPER_MODEL` (a `ggml-*.bin` file), `VOXORA_BENCH_WAV` |
| `voxora-qwen3asr` | `VOXORA_BENCH_QWEN_DIR` (a model directory with `tokenizer.json`), `VOXORA_BENCH_WAV` |

`QwenAsrEngine::from_hf` (or `prepare_model_dir`) writes the
`tokenizer.json`, so a directory that voxora-hf has resolved once
is ready to use.

```text
VOXORA_BENCH_WHISPER_MODEL=path/to/ggml-tiny.bin \
VOXORA_BENCH_WAV=voxora-qwen3asr/tests/fixtures/audio/sample1.wav \
  cargo bench -p voxora-whisper --bench transcribe_wav

VOXORA_BENCH_QWEN_DIR=~/.cache/voxora/models/huggingface/Qwen/Qwen3-ASR-0.6B/main \
VOXORA_BENCH_WAV=voxora-qwen3asr/tests/fixtures/audio/sample1.wav \
  cargo bench -p voxora-qwen3asr --bench transcribe_wav
```

`sample1.wav` (3.4 s of English speech, public domain) is
downloaded by the qwen3asr parity test
(`cargo test -p voxora-qwen3asr --features hf --test parity -- --ignored`).

Criterion prints the time per transcription; the bench then prints
the RTF from the median of 5 more runs:

```text
whisper/transcribe_wav  time:   [799.67 ms 825.90 ms 848.18 ms]
whisper RTF: 0.247 (median 0.84 s of 5 runs for 3.40 s of audio)
```

That line is `ggml-tiny.bin` on CPU. Results depend on the model,
the build features (`cuda`, `metal`, …) and the machine, so only
compare runs made on the same setup.

Use speech, not silence or noise: on those Whisper can emit invented text
and run longer, which distorts the RTF.

The MiniMax engine has no bench: it calls a paid hosted API, so
its time is network latency and every run costs money.
