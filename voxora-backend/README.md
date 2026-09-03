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

The real candle-backed runtime detection lands in 0.2.x once
voxora-backend gains a `candle` feature flag.