# voxora-engine

Canonical adapter contract for voxora engine crates (ASR-specific).

Every `voxora-<engine>` crate (voxora-whisper, voxora-qwen3asr, future
voxora-parakeet, voxora-voxtral, …) builds on top of this crate so
consumers see a uniform shape: an [`EngineAdapter`] trait that exposes
family, info, backend, and the wrapped [`voxora_traits::AsrEngine`].

The adapter wraps `voxora_traits::AsrEngine` — there is no generic
`Model` trait at this layer. The next planned engine family (parakeet,
voxtral, …) lands as a new variant on [`EngineFamily`] in 0.2.0; today
the enum covers `Whisper` and `Qwen3Asr` and is `#[non_exhaustive]`.

Re-exports: [`AnyEngine`], [`BackendDescriptor`], [`BackendKind`],
[`EngineFamily`], [`EngineInfo`], [`InvalidEngineFamily`], and the
`testing::MockAdapter` helper for downstream test suites.

## Hardware backends

[`BackendKind`] is the canonical enum shared by every engine
adapter (`Cpu` / `Cuda` / `Metal` / `Vulkan`); [`BackendDescriptor`]
is what an adapter returns from [`EngineAdapter::backend`](https://docs.rs/voxora-engine).
The cross-engine hardware / GPU support matrix — which engine ships
which backend, CUDA compute capability per backend, the
`voxora-bridge` forwarding rules — is documented in
[`docs/GPU_SUPPORT.md`](../docs/GPU_SUPPORT.md). The runtime picker
("CUDA → Metal → CPU at process start") lives in
[`voxora-backend`](../voxora-backend/README.md) behind its opt-in
`candle` Cargo feature.

## Examples

- [`adapter_dispatch`](examples/adapter_dispatch.rs) — wrap a
  [`MockAdapter`] behind [`AnyEngine`], dispatch on `family()`, and
  call the borrowed [`voxora_traits::AsrEngine`] synchronously. The
  smallest possible program that proves the adapter contract works
  without touching Hugging Face or loading a real model. Run with
  `cargo run --example adapter_dispatch -p voxora-engine`.
