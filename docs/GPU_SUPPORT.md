# voxora — GPU / Hardware Support Matrix

This document describes the hardware backends each voxora engine crate
ships today, how to enable them, and what the cross-cutting
`voxora-backend` dispatcher does (and does not) do. It is the single
source of truth for the per-backend feature flags documented in
`voxora-whisper/Cargo.toml`, `voxora-qwen3asr/Cargo.toml`, and
`voxora-bridge/Cargo.toml`.

> ASR-specific: this matrix covers only the backends voxora needs for
> speech recognition. Generic LLM / vision backends are out of scope.

---

## TL;DR — the matrix

| Backend      | `voxora-whisper` | `voxora-qwen3asr` | `voxora-cli` | Notes |
|--------------|------------------|-------------------|--------------|-------|
| CPU          | default          | default           | default      | Always built; works on every target. |
| CUDA (NVIDIA)| `cuda`           | `cuda`            | `cuda`       | Requires NVIDIA driver + matching toolkit on the host. |
| Metal (Apple)| `metal`          | `metal`           | `metal`      | macOS only. |
| Vulkan       | `vulkan`         | —                 | `vulkan`     | `qwen3-asr` upstream has no Vulkan backend. The CLI's `vulkan` flag is whisper-only (mirrors `voxora-bridge`'s shape). |

Every backend is an **opt-in Cargo feature**. The default
(`cpu`-only) build is portable and ships with no SDK dependency,
which is what the docs.rs sandbox, CI, and hermetic test environments
rely on.

The `voxora-bridge` umbrella crate forwards each backend as a
top-level Cargo feature (`cuda`, `cuda-whisper`, `cuda-qwen3asr`,
`metal`, `vulkan`) so a downstream consumer does not need to know
which engine owns which flag. See
[`voxora-bridge/Cargo.toml`](../voxora-bridge/Cargo.toml) for the
canonical mapping.

The `voxora-cli` binary mirrors the umbrella crate: same feature
names (`cpu` / `cuda` / `metal` / `vulkan`), same whisper-only
shape for `vulkan`. Pair with `--hardware <cpu|cuda|metal|vulkan>`
on `voxora run` to validate the binary was built with the matching
feature (the flag is build-time validation, not a runtime GPU
switch — see [§ Compile-time vs runtime](#compile-time-vs-runtime-selection)
below). Closes #121.

---

## Engine-by-engine reference

### `voxora-whisper` (whisper.cpp via `whisper-rs` 0.16)

Mirrors the upstream `whisper-rs` feature flags.

| Feature | Upstream crate | Backend |
|---|---|---|
| `cpu` (default) | — | CPU. No SDK needed. |
| `metal` | `whisper-rs/metal` | Apple Metal. macOS only. Needs Xcode command-line tools. |
| `cuda` | `whisper-rs/cuda` | NVIDIA CUDA. Needs `nvcc` at build time and a CUDA driver at runtime. Works back to **sm_50 (Pascal)**. |
| `vulkan` | `whisper-rs/vulkan` | Vulkan compute. Needs the Vulkan loader. |
| `hf` | `dep:voxora-hf` | Pulls in `voxora-hf` for `WhisperEngine::from_hf`. |
| `engine-adapter` | `dep:voxora-engine` | Opt-in to the `voxora_engine::EngineAdapter` contract. |

Compile one backend only:

```text
cargo build -p voxora-whisper --no-default-features --features cpu
cargo build -p voxora-whisper --no-default-features --features metal
cargo build -p voxora-whisper --no-default-features --features cuda
cargo build -p voxora-whisper --no-default-features --features vulkan
```

The default `cpu` build is the no-op backend; it is omitted from
explicit `features = [...]` lists in `[package.metadata.docs.rs]`
because it carries no SDK gate. The `metal` / `cuda` / `vulkan`
features stay off the docs.rs build for the same reason
([issue #93](https://github.com/airvzxf/voxora/issues/93)).

### `voxora-qwen3asr` (candle-native via `qwen3-asr` 0.2)

Mirrors the upstream `qwen3-asr` feature flags.

| Feature | Upstream crate | Backend |
|---|---|---|
| `cpu` (default) | — | CPU. No SDK needed. |
| `metal` | `qwen3-asr/metal` | Apple Metal. macOS only. |
| `cuda` | `qwen3-asr/cuda` | NVIDIA CUDA. Needs `nvcc` at build time. **Requires `sm_70+` (Volta)** because the candle CUDA backend pulls in WMMA kernels that did not exist on Pascal. |
| `hf` | `dep:voxora-hf`, `serde_json`, `anyhow` | Pulls in `voxora-hf` for `QwenAsrEngine::from_hf`. |
| `engine-adapter` | `dep:voxora-engine` | Opt-in to the `voxora_engine::EngineAdapter` contract. |

There is **no Vulkan backend** for `voxora-qwen3asr`. `qwen3-asr`
upstream does not ship one; `voxora-bridge/vulkan` therefore only
forwards to `voxora-whisper`.

Compile one backend only:

```text
cargo build -p voxora-qwen3asr --no-default-features --features cpu
cargo build -p voxora-qwen3asr --no-default-features --features metal
cargo build -p voxora-qwen3asr --no-default-features --features cuda
```

#### CUDA compute capability

- **Pascal (`sm_60`) and earlier** are not supported on
  `voxora-qwen3asr/cuda`. Use `voxora-whisper/cuda` (which works back
  to `sm_50`) or fall back to CPU.
- **Volta (`sm_70`)** through **Hopper (`sm_90`)** are the supported
  range. Blackwell (`sm_100+`) is forward-compatible via PTX but is
  not exercised by CI.
- The optional `QWEN3_ASR_CUDA_NATIVE_BF16` env var (read by
  upstream `qwen3-asr` at load time) keeps BF16 weights on CUDA
  instead of converting to F16/F32. Useful on **`sm_80+`** for
  benchmarking; voxora does not touch this variable. Set it before
  calling `QwenAsrEngine::load` if you want the fast path.

### `voxora-backend` (cross-cutting dispatcher)

| Feature | Behaviour |
|---|---|
| (none, default) | `best_device()` and `detect()` always return `Cpu`. No SDK dependency pulled in. |
| `candle` | Enables `candle-core`'s runtime probe; `best_device()` returns `Cuda` → `Metal` → `Cpu` based on what the host actually has at process start. |

The `candle` feature is **off by default** to keep the dependency
graph small for users who do not need runtime detection. Without it,
the dispatcher is a safe CPU fallback that compiles everywhere,
which is what the docs.rs sandbox and CI rely on.

Opt in:

```text
cargo build -p voxora-backend --features candle
```

> **CI caveat**: the GitHub Actions matrix runs with default features
> only. The `candle` feature is **not** exercised by CI; if you
> depend on it, please verify locally before bumping `voxora-backend`.

#### Detection environment variables

`voxora-backend::device::candle_compiled_backends()` consults one
environment variable when reporting the compiled list:

| Variable | Effect |
|---|---|
| `VOXORA_ENABLE_CUDA=1` | Adds `BackendKind::Cuda` to the `Capabilities::compiled` list, so callers can distinguish "CUDA feature not built" from "CUDA feature built but no driver". The runtime probe is still the source of truth for `available`. |

Without `VOXORA_ENABLE_CUDA=1`, the compiled list is `[Cpu]` plus
`Metal` on `target_os = "macos" | "ios"`. The runtime probe then
reorders `available` so the actually-detected backend comes first.

### `voxora-bridge` (umbrella crate)

The umbrella forwards each backend as its own Cargo feature so a
single line in the consumer's `Cargo.toml` covers every engine.

| Feature | Forwards to |
|---|---|
| `cuda` | `cuda-whisper` + `cuda-qwen3asr` |
| `cuda-whisper` | `voxora-whisper/cuda` |
| `cuda-qwen3asr` | `voxora-qwen3asr/cuda` |
| `metal` | `voxora-whisper/metal` + `voxora-qwen3asr/metal` |
| `vulkan` | `voxora-whisper/vulkan` only |

The split between `cuda-whisper` and `cuda-qwen3asr` exists because
the two engines have different minimum CUDA compute capabilities:
`voxora-whisper/cuda` works back to **`sm_50` (Pascal)** via
`ggml-cuda`, while `voxora-qwen3asr/cuda` requires **`sm_70+`
(Volta)** because of the candle WMMA kernels. On hardware older than
Volta, enable `cuda-whisper` only.

Defaults are `["whisper", "qwen3asr"]`, which together imply the
`cpu` build of each engine. To target a specific accelerator through
the umbrella:

```text
# Whisper only, CUDA, Pascal-friendly:
cargo build -p voxora-bridge --no-default-features \
  --features 'whisper cuda-whisper'

# Both engines, CUDA on Volta+:
cargo build -p voxora-bridge --no-default-features \
  --features 'whisper qwen3asr cuda'

# macOS Apple Silicon, both engines, Metal:
cargo build -p voxora-bridge --no-default-features \
  --features 'whisper qwen3asr metal'

# Vendor-neutral GPU (NVIDIA / AMD / Intel) via Vulkan (whisper only):
cargo build -p voxora-bridge --no-default-features \
  --features 'whisper vulkan'
```

### `voxora-cli` (CLI binary)

Mirrors `voxora-bridge`'s forwarding rules at the binary level. Each
hardware feature forwards to the matching engine feature; `vulkan`
is whisper-only because qwen3-asr upstream has no Vulkan backend
(see [`voxora-bridge/Cargo.toml`](../voxora-bridge/Cargo.toml) for
the same shape and the rationale comment).

| Feature | Forwards to |
|---|---|
| `cpu` (default) | `voxora-whisper/cpu` + `voxora-qwen3asr/cpu` |
| `cuda` | `voxora-whisper/cuda` + `voxora-qwen3asr/cuda` |
| `metal` | `voxora-whisper/metal` + `voxora-qwen3asr/metal` |
| `vulkan` | `voxora-whisper/vulkan` only |

The `--hardware <cpu|cuda|metal|vulkan>` flag on `voxora run`
validates that the binary was built with the matching Cargo feature
and surfaces a `voxora run: backend = ..., hardware = ...` log line
for observability. It does **not** influence engine dispatch —
whisper-rs picks its own runtime backend from the compiled Cargo
features today, and there is no upstream per-call GPU switch. The
flag is build-time validation, not a runtime selector. `--hardware
vulkan` against `--engine qwen3-asr` fails fast because the
combination is structurally impossible.

```text
# Build a Vulkan-enabled CLI (whisper only; needs the Vulkan loader):
cargo build -p voxora-cli --features vulkan

# Then validate the binary at runtime:
voxora run org/whisper-model audio.wav --hardware vulkan
```

If the binary was built without the matching `--features` flag,
the CLI rejects the invocation with exit code 2 *before* any
network round-trip:

```text
$ voxora run org/whisper-model audio.wav --hardware vulkan
error: build configuration error: --hardware "vulkan" requested but
       voxora-cli was built without the `vulkan` feature
```

---

## Compile-time vs runtime selection

voxora separates **which backends were compiled in** from **which
backend the binary actually uses at runtime**.

- **Compile time**: Cargo features (`cpu` / `cuda` / `metal` /
  `vulkan` per engine, plus `voxora-backend/candle` for the runtime
  probe). The defaults keep the build portable; the alternatives
  require the matching SDK on the build host.

- **Runtime**: `voxora_backend::best_device()` (with the `candle`
  feature) probes CUDA → Metal → CPU at process start and returns
  the first one that succeeds. `voxora_backend::Capabilities`
  reports both the compiled list and the available list so callers
  can distinguish "feature not built" from "feature built but no
  driver".

  Without the `candle` feature, `best_device()` always returns `Cpu`.
  This is the safe fallback that the default build and CI use.

`voxora-qwen3asr` adds its own runtime picker (`qwen3_asr::best_device()`)
that `QwenAsrEngine::load` calls internally. The order is identical
(CUDA → Metal → CPU). For explicit control, use
`QwenAsrEngine::load_with_device(model_dir, device)` and pass a
`candle_core::Device` re-exported as `voxora_qwen3asr::Device`.

`voxora-whisper` does not have a runtime picker; the engine picks
the only backend that was compiled in. If you enabled both `cuda`
and `metal` in the same build, the engine selects CUDA at load time
because whisper-rs prefers NVIDIA; on Apple Silicon it selects
Metal. The behaviour is inherited from `whisper-rs`.

---

## docs.rs sandbox caveat

The docs.rs build runner has **no CUDA SDK, no Metal SDK, and no
Vulkan loader**. Every voxora engine crate that has a
hardware-backend Cargo feature carries a
`[package.metadata.docs.rs]` block that lists the **doc-safe**
features explicitly:

- `voxora-whisper`: `features = ["hf", "engine-adapter"]` (CPU is
  the default and is implicit; `metal` / `cuda` / `vulkan` are
  SDK-gated and stay off).
- `voxora-qwen3asr`: `features = ["hf", "engine-adapter"]` (same
  rationale).
- `voxora-bridge`: `features = ["whisper", "qwen3asr"]` (same).
- `voxora-backend`: `all-features = true` is safe here because
  `candle-core` is the only gated dep and it builds on the docs.rs
  sandbox.

`all-features = true` would be unsafe on the engine crates because
the `cuda` / `metal` / `vulkan` features pull in SDK-gated
transitive deps. The explicit `features = [...]` list is the
load-bearing piece of the docs.rs metadata (closes
[issue #93](https://github.com/airvzxf/voxora/issues/93)).

---

## Performance — what GPU actually buys you

The headline numbers from the maintainer's benchmark suite on
`Qwen3-ASR-0.6B` (RTX 3090, rented; see
[`docs/INVESTIGATION.md`](INVESTIGATION.md)):

| Metric | Rust candle CPU | Rust candle GPU (RTX 3090) | Speedup |
|---|---:|---:|---:|
| RTF (mean across 5 audios) | 1.089 | **0.0313** | **~35×** |
| Cold start → first result | 8 s | **3.5 s** | **~2.3×** |
| VRAM peak | — | 4.2 GB | — |

GPU wins on inference latency, cold start, and (in the maintainer's
own measurement) **3.4× faster than PyTorch on the same RTX 3090**.
CPU is portable and 2.5× slower than PyTorch on the same audio;
this is the cost of running a 600M-parameter ASR model without
tensor cores.

Whisper.cpp's ggml-cuda is the same story at smaller scale: a
`ggml-tiny.bin` model finishes in well under a second on a modern
GPU and a few seconds on CPU. The `cuda` feature on
`voxora-whisper` is the right pick when you have an NVIDIA card and
need real-time streaming transcription on long audio.

---

## How to pick a backend in your own code

Three patterns, in increasing order of explicit control:

1. **Use `voxora-backend::best_device()` to print a diagnostic.**
   With the `candle` feature enabled, this is the canonical "what
   does the host actually have" check. Use it in `--info`-style
   commands and in startup logs.

   ```rust,ignore
   use voxora_backend::best_device;
   let backend = best_device();
   eprintln!("voxora will use {:?} for inference", backend);
   ```

2. **Let each engine pick its own default.**
   `QwenAsrEngine::load(model_dir)` calls `qwen3_asr::best_device()`
   internally; `WhisperEngine::load(model_dir)` picks whatever the
   `whisper-rs` build was compiled for. Both follow the
   CUDA → Metal → CPU ordering at runtime, so on a multi-backend
   build the GPU wins when the driver is present.

3. **Pin a specific device explicitly.**
   `QwenAsrEngine::load_with_device(model_dir, device)` takes a
   `candle_core::Device` re-exported as `voxora_qwen3asr::Device`.
   Use this to:

   - force CPU on a machine where Metal is also compiled in (e.g.
     for parity benchmarking),
   - pin a specific CUDA device index when the host has multiple
     GPUs,
   - bypass `best_device()` entirely when the host's runtime probe
     is wrong.

---

## Cross-references

- [`voxora-backend/README.md`](../voxora-backend/README.md) — the
  `candle` feature flag, `best_device()` / `detect()`, and the
  `Capabilities` snapshot.
- [`voxora-backend/src/device.rs`](../voxora-backend/src/device.rs)
  — the runtime probe implementation.
- [`voxora-bridge/Cargo.toml`](../voxora-bridge/Cargo.toml) — the
  umbrella crate's forwarding rules; `cuda` / `cuda-whisper` /
  `cuda-qwen3asr` / `metal` / `vulkan` all live there.
- [`voxora-whisper/Cargo.toml`](../voxora-whisper/Cargo.toml) —
  per-engine hardware features for whisper.cpp.
- [`voxora-qwen3asr/Cargo.toml`](../voxora-qwen3asr/Cargo.toml) —
  per-engine hardware features for candle-native Qwen3-ASR
  (incl. the `sm_70+` requirement).
- [`voxora-engine/src/backend.rs`](../voxora-engine/src/backend.rs)
  — the `BackendKind` enum (`Cpu` / `Cuda` / `Metal` / `Vulkan`)
  shared by every adapter.
- [`voxora-qwen3asr/src/engine.rs`](../voxora-qwen3asr/src/engine.rs)
  — `load` / `load_with_device` and the `QWEN3_ASR_CUDA_NATIVE_BF16`
  passthrough.
- [`docs/ROADMAP.md`](ROADMAP.md) — Phase 7's "Hardware dispatcher"
  design note (the runtime CUDA → Metal → CPU picker this doc
  describes is the first concrete step on that plan).

---

## Roadmap — what changes for 0.5.0 and beyond

The current matrix is **compile-time only for the engines** and
**runtime-only for `voxora-backend`**. Phase 7 of the roadmap
explicitly calls out a "Hardware dispatcher" that surfaces
`best_device()` as a single source of truth usable by every engine,
not just `voxora-backend`:

> Right now voxora-qwen3asr takes a device at load time. For
> consumers that want "pick whatever's available", we need a
> `best_device()` helper that resolves at process start. qwen3-asr
> already has one upstream; we just need to surface it.
> ([ROADMAP.md § Phase 7](ROADMAP.md#phase-7--more-engines))

Longer-term, Phase 7 also scopes:

- **`voxora-vad`** — a voice activity detection utility built on top
  of the same `VadSegmenter` trait pattern. Useful for trimming
  silence before ASR; will likely need its own streaming-aware
  trait extension (`StreamingAsrEngine`) which is a real breaking
  change, so this lands as phase 8.
- **Parakeet, Voxtral, Granite-Speech** adapters — each adds its own
  Candle backend matrix to this document when it lands. The
  cross-cutting story (compile-time feature per backend, runtime
  probe via `voxora-backend::best_device`) is the contract every
  new engine follows.

Open question tracked in
[issue #55](https://github.com/airvzxf/voxora/issues/55) and the
umbrella
[EPIC #117](https://github.com/airvzxf/voxora/issues/117): extend
this doc as new engines arrive, or split into per-engine sub-pages
once `voxora-parakeet` lands.

---

*Last updated: 2026-09-05 — initial GPU / hardware support matrix
introduced alongside the cross-engine `BackendKind` enum and
`voxora-backend::best_device()`. Closes issue #55 as part of the
0.5.0 coordinated release (EPIC #117).*
