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