# voxora-backend

Cross-cutting hardware backend selection for the voxora ASR stack.

Centralises `Cpu / Cuda / Metal / Vulkan` selection so individual
engine crates (voxora-whisper, voxora-qwen3asr, future parakeet /
voxtral adapters, …) no longer each declare their own `cpu` / `cuda`
/ `metal` / `vulkan` feature wiring.

ASR-specific: only models hardware backends that exist in the voxora
speech-recognition stack. Does not abstract LLama.cpp or any other
non-ASR backend.

## Phase 0 surface

- [`BackendKind`] — re-exported from `voxora-engine`.
- [`Capabilities`] — compile-time + runtime snapshot.
- [`best_device`] / [`best_device_or_error`] — current
  "always returns `Cpu`" implementation.

## `candle` Cargo feature

`voxora-backend 0.3.0` adds an opt-in `candle` Cargo feature. When
enabled, [`best_device`] and [`detect`] use `candle-core`'s
runtime probe (CUDA → Metal → CPU) instead of always returning `Cpu`.

```toml
voxora-backend = { version = "0.3", features = ["candle"] }
```

The feature is off by default to avoid pulling in `candle-core`'s
transitive deps (cudarc, objc2-metal, …) for users who don't need
runtime detection. It will be promoted to default-on in `0.3.1`
once candle integration is validated on more platforms.

> **CI caveat**: the GitHub Actions matrix runs with default
> features only. The `candle` feature is **not** exercised by CI;
> if you depend on it, please verify locally before bumping
> `voxora-backend`.

Without the `candle` feature the behaviour is unchanged: both
[`best_device`] and [`detect`] return `Cpu` as the safe fallback.