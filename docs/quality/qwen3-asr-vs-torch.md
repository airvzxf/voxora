# Qwen3-ASR: candle (Rust) vs PyTorch

A one-off comparison, run on 2026-07-06/07, of the Rust Qwen3-ASR
engine that `voxora-qwen3asr` wraps (the `qwen3-asr` crate) against
the official Python package (`qwen_asr` on PyTorch). Same model, same
five audio clips, same greedy decoding.

The raw per-run data is in [`qwen3-asr-vs-torch/`](qwen3-asr-vs-torch/).
Every number on this page is computed from those JSON files.

## Scope and limits

- **This is a historical record, not a reproducible benchmark.** The
  Rust harness was a `benchmark` binary in a development branch of
  `qwen3-asr` that no longer exists. voxora's own RTF benches are
  still stubs (see [`benchmarks.md`](benchmarks.md)).
- The Rust model code was identical to `qwen3-asr` 0.2.2 on
  crates.io, except for one weight-loader patch that only affects
  CUDA. That patch was active only in the `rust-cuda-f32` run (see
  [CUDA](#cuda-rtx-3090)).
- Model: `Qwen/Qwen3-ASR-0.6B` safetensors (1789 MiB).
- 1 warmup run, then 10 measured runs per clip.
- Audio: 102.85 s in total, 5 clips (3.4 s, 4.0 s and 36.4 s
  English; 30.4 s Chinese; 28.7 s mixed Chinese/English).
- "GT" is an exact match of the transcript against the expected text.
  The only miss (torch, mixed clip) is one dropped comma.
- RTF is total inference time divided by total audio duration.
  Lower is better.

## Hardware

| Run | Machine |
|---|---|
| `rust-cpu`, `torch-cpu` | Intel i7-7820HK (4C/8T), 64 GB, Arch Linux. Rust 1.90.0-nightly, torch 2.12.1+cu130 |
| `rust-cuda-*`, `torch-cuda` | RTX 3090 (sm_86, 24 GB), driver 570.133.20, AMD Threadripper 2920X, Ubuntu 24.04 (rented). Rust 1.96.1, torch 2.11.0+cu128 |

The laptop ran hot during `rust-cpu` (the 36 s clip has a 30 %
stddev), so treat that row as approximate.

## Summary

| Run | RTF | Model load | RSS after load | GT |
|---|---:|---:|---:|:--:|
| `rust-cpu` | 1.089 | 4.36 s | 3.7 GB | 5/5 |
| `torch-cpu` | 0.435 | 1.70 s | 6.4 GB | 4/5 |
| `rust-cuda-bf16` | **0.0313** | 1.66 s | — | 5/5 |
| `rust-cuda-f32` | 0.0339 | 1.68 s | — | 5/5 |
| `torch-cuda` | 0.1074 | 4.24 s | — | 4/5 |

- **CPU:** torch is about 2.5× faster per inference than candle.
- **CUDA:** candle is about 3.4× faster than torch on the same GPU,
  and transcribes about 32× faster than real time.
- **Memory on CPU:** candle uses about 40 % less RSS.
- VRAM was not recorded in the raw data.

## CPU (i7-7820HK)

Mean of 10 runs, in milliseconds.

| Clip | Duration | `rust-cpu` | `torch-cpu` |
|---|---:|---:|---:|
| sample1 (en) | 3.40 s | 3 646 | 2 495 |
| sample2 (en) | 4.01 s | 4 673 | 2 573 |
| sample4 (en) | 36.43 s | 45 628 | 16 617 |
| sample5 (zh) | 30.35 s | 32 298 | 12 193 |
| sample6 (zh/en) | 28.66 s | 25 712 | 10 824 |

## CUDA (RTX 3090)

Mean of 10 runs, in milliseconds.

| Clip | Duration | `rust-cuda-bf16` | `rust-cuda-f32` | `torch-cuda` |
|---|---:|---:|---:|---:|
| sample1 (en) | 3.40 s | 165 | 176 | 509 |
| sample2 (en) | 4.01 s | 166 | 177 | 559 |
| sample4 (en) | 36.43 s | 1 080 | 1 186 | 3 807 |
| sample5 (zh) | 30.35 s | 958 | 1 022 | 3 257 |
| sample6 (zh/en) | 28.66 s | 847 | 922 | 2 917 |

The candle runs are steady (stddev ≤ 11 ms per clip). The torch runs
vary between 43 and 787 ms per clip.

**What voxora gets today.** `qwen3-asr` 0.2.2 converts BF16/F16
weights to F32 only on CPU, and keeps them in BF16 on CUDA and
Metal. That is the `rust-cuda-bf16` row. The `rust-cuda-f32` row came
from a patched loader that also converted to F32 on CUDA. That patch
is not in any published `qwen3-asr` release, and voxora cannot
enable it.

## Cold start

The harnesses measured time to the first result differently, so the
numbers below can't be compared across implementations:

- `rust-cpu`: 8.0 s from process start to the first transcript.
  This is model load (4.4 s) plus the first transcription, before
  any warmup.
- `torch-cpu`: 57.2 s. This includes interpreter start, imports,
  model load and a warmup pass over all five clips.

Use the "Model load" column in the summary for an apples-to-apples
comparison.

## Raw data

| File | Run |
|---|---|
| [`rust-cpu.json`](qwen3-asr-vs-torch/rust-cpu.json) | candle, CPU, i7-7820HK |
| [`torch-cpu.json`](qwen3-asr-vs-torch/torch-cpu.json) | `qwen_asr` + torch, CPU, i7-7820HK |
| [`rust-cuda-bf16.json`](qwen3-asr-vs-torch/rust-cuda-bf16.json) | candle, CUDA, native BF16 weights |
| [`rust-cuda-f32.json`](qwen3-asr-vs-torch/rust-cuda-f32.json) | candle, CUDA, weights converted to F32 (patched loader) |
| [`torch-cuda.json`](qwen3-asr-vs-torch/torch-cuda.json) | `qwen_asr` + torch, CUDA |

Each file holds the hardware, build, model load time and memory,
and every measured run (elapsed time, RTF and transcript), plus an
`aggregate` block.
