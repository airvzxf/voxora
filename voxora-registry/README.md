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

## Examples

- [`registry_resolve`](examples/registry_resolve.rs) — build a
  [`Registry`] with both built-in descriptors and resolve a model
  id, printing the descriptor and on-disk [`voxora_traits::ModelDir`]
  the resolver chose. Run with
  `cargo run --example registry_resolve -p voxora-registry -- <hf-model-id>`.