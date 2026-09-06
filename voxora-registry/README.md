# voxora-registry

Central model registry for voxora (ASR-specific).

Owns the answer to "given a model id, give me the engine descriptor
+ the exact on-disk file". Before this crate, that answer was
distributed across `voxora-hf` (cache key), `voxora-whisper`
(lex-sort over `*.bin`), and `voxora-cli` (heuristic). Now it lives
here.

Structural fix for [voxora#79](https://github.com/airvzxf/voxora/issues/79)
(`ggerganov/whisper.cpp/ggml-large-v3.bin` incorrectly resolving to
`ggml-base.bin`): `ModelDir::entry`, populated by `voxora-hf 0.1.2`
and surfaced through `Registry::resolve`, names the exact file.
Consumers no longer need to lex-sort.

## Phase 0 surface

- [`ModelId::parse`] — strict parser for `org/repo`,
  `org/repo/file`, `/local/path`, `./local-path`.
- [`Registry::resolve`] → `(EngineDescriptor, ModelDir)`.
- [`builtin_whisper_descriptor`] / [`builtin_qwen3asr_descriptor`]
  default descriptors.
- [`CacheManifest`] — `.voxora-manifest.json` sidecar.
- [`hf::hf_registry`] convenience builder (behind the `hf` feature).

ASR-specific: descriptors model only ASR engines (Whisper,
Qwen3-Asr). Adding `parakeet`, `voxtral`, or `granite-speech` is a
0.2.x change.

## Hardware backends

Model resolution is engine-agnostic; the hardware backend the
resolved model runs on is selected downstream by the engine
adapter. The cross-engine hardware / GPU support matrix (CUDA /
Metal / Vulkan / CPU per engine, compute capability requirements,
`voxora-bridge` forwarding rules) is documented in
[`docs/GPU_SUPPORT.md`](../docs/GPU_SUPPORT.md).