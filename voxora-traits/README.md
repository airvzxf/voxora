# voxora-traits

Canonical traits and value types for the [voxora](https://github.com/airvzxf/voxora)
model-agnostic ASR bridge.

This crate is the canonical home of the public API surface. It has
**zero runtime dependencies** beyond `async-trait` and `thiserror`
(no `tokio`, no `reqwest`, no `http`) so it builds offline and stays
small.

## Relationship with `voxora-core`

Since voxora 0.3.0, `voxora-core` is a thin shim that re-exports this
crate for backwards compatibility. New code should depend on
`voxora-traits` directly.

## Surface

| Item | Kind | Description |
|---|---|---|
| `AsrEngine` | trait | Synchronous, `Send + Sync` inference contract |
| `ModelSource` | trait | Asynchronous, `Send + Sync` model acquisition contract |
| `AsrError` | enum | Every error a voxora operation may return |
| `ModelCapabilities` / `TranscribeOptions` / `TranscriptionResult` / `TranscriptionSegment` | struct | Inference value types |
| `ModelDescriptor` / `ModelDir` / `ModelSourceKind` / `Quantization` / `QuantizationPreference` / `ResolveOptions` | struct / enum | Source value types |

## ASR-specific

The design is ASR-specific per operator direction. The `AsrEngine`
trait is purpose-built for automatic speech recognition. New domains
(LLM, vision, multimodal) are out of scope per the architecture
handoff.

## License

Apache-2.0.