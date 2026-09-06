# voxora-config

Single source of truth for every [voxora](https://github.com/airvzxf/voxora)
runtime setting: the model cache root plus the Hugging Face token, base
URL, and default revision. ASR-specific by design — it models what the
voxora speech-recognition stack needs, not LLM or vision settings.

Every setting resolves through one cascade, first non-empty wins: an
explicit value on `VoxoraConfig` (set by the caller, or read from a TOML
file with `VoxoraConfig::from_file`), then the environment, then the
built-in default.

Variables honoured: `VOXORA_CACHE_DIR`, `VOXORA_HF_TOKEN`,
`VOXORA_HF_BASE_URL`, `VOXORA_HF_REVISION`, plus the `HF_TOKEN` and
`HUGGING_FACE_HUB_TOKEN` aliases. Per-setting rules: [module docs](https://docs.rs/voxora-config).

## Related: hardware backend selection

Hardware backend selection (CUDA / Metal / Vulkan / CPU) is a
compile-time Cargo feature per engine, with a runtime
`voxora_backend::best_device()` probe when the `candle` feature
is enabled. See [`docs/GPU_SUPPORT.md`](../docs/GPU_SUPPORT.md) for
the full cross-engine matrix and
[`voxora-backend/README.md`](../voxora-backend/README.md) for the
runtime picker.

## Examples

- [`env_cascade`](examples/env_cascade.rs) — print the resolved
  cache root, HF token, HF base URL, and default HF revision with
  every cascade layer applied. Pass an optional positional TOML
  path to verify a `voxora.toml` is read at the expected location.
  Run with
  `cargo run --example env_cascade -p voxora-config` (or
  `cargo run --example env_cascade -p voxora-config -- voxora.toml`).
