# voxora-local

Local-directory [`voxora_traits::ModelSource`] for the
[voxora](https://github.com/airvzxf/voxora) model-agnostic ASR
bridge.

This crate ships:

- [`LocalSource`](https://docs.rs/voxora-local/latest/voxora_local/struct.LocalSource.html) —
  resolves a model id (e.g. `"some-org/some-repo/model.bin"`)
  against a directory already on disk. No network, no tokio, no
  reqwest.
- [`ChainedSource`](https://docs.rs/voxora-local/latest/voxora_local/struct.ChainedSource.html) —
  first-hit-wins adapter that tries a primary source and falls
  back to a secondary source on
  `voxora_traits::AsrError::ModelNotFound`. Useful for the
  "local first, Hugging Face on miss" composition.

## Use cases

- **Vendored weights.** Operators who ship model artefacts with
  their binary can resolve them via `LocalSource` without a HF
  token, a cache directory, or outbound network.
- **Hermetic CI.** Every test lane that wants a deterministic
  `ModelSource` can wire a `LocalSource` against a `tempdir`
  pre-populated by `cargo` build scripts. No wiremock, no live HF
  download, no flake.
- **Local-first registry.** Wrap
  `ChainedSource::new(LocalSource::new("/srv/models"), HuggingFaceSource::new()?)`
  and hand the result to a `voxora-registry::Registry`. Local
  artefacts win; only misses go on the wire.

## Quickstart

```rust,no_run
use std::sync::Arc;
use voxora_local::{ChainedSource, LocalSource};
use voxora_hf::HuggingFaceSource;
use voxora_traits::ModelSource;

# async fn run() -> Result<(), voxora_traits::AsrError> {
let chain = ChainedSource::new(
    Arc::new(LocalSource::new("/srv/models")),
    Arc::new(HuggingFaceSource::new()?),
);
let dir = chain
    .resolve("some-org/some-repo/model.bin", &Default::default())
    .await?;
println!("model at {}", dir.entry.expect("entry").display());
# Ok(()) }
```

## Limitations

- `voxora-registry`'s built-in descriptors only accept HF ids
  today. The `ChainedSource` adapter in this crate is the pattern
  that honours "local first" without forcing a registry refactor;
  pass it to `Registry::new` and the chain is transparent. A
  follow-up issue tracks extending the descriptor accept arm to
  `SourceKind::Local`.

## Surface

| Item | Kind | Description |
|---|---|---|
| `LocalSource` | struct | Reads model weights from a local directory; zero network deps |
| `ChainedSource` | struct | First-hit-wins adapter with `AsrError::ModelNotFound` fallback |

## License

Apache-2.0.
