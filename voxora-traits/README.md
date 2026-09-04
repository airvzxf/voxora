# voxora-traits

Canonical traits and value types for the [voxora](https://github.com/airvzxf/voxora)
model-agnostic ASR bridge.

This crate is the canonical home of the public API surface. It has
**zero runtime dependencies** beyond `async-trait` and `thiserror`
(no `tokio`, no `reqwest`, no `http`) so it builds offline and stays
small.

## Removed predecessor: `voxora-core`

`voxora-core` was a thin re-export shim from voxora 0.3.0 to
0.3.1 that allowed downstream code to import the trait surface via
`use voxora_core::*;` while the canonical home was this crate.
The shim was deprecated in 0.3.1, **removed in 0.4.0**, and the
two published shim releases (`0.3.0` and `0.3.1`) were
**yanked from crates.io on 2026-09-04** (issue #79). New code
has always depended, and continues to depend, on
`voxora-traits` directly.

### Upgrade guide — `voxora-core` (deprecated) → `voxora-traits` (canonical)

```rust
// before — voxora-core shim (0.3.0 / 0.3.1)
use voxora_core::AsrEngine;
use voxora_core::TranscribeOptions;

// after — voxora-traits directly (0.4.0+)
use voxora_traits::AsrEngine;
use voxora_traits::TranscribeOptions;
```

In your `Cargo.toml`, swap `voxora-core = "0.3"` for
`voxora-traits = "0.4"`. If you used the `voxora-core/serde` Cargo
feature, switch to `voxora-traits/serde`.

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