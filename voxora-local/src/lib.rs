//! Local-directory [`voxora_traits::ModelSource`] for the voxora
//! model-agnostic ASR bridge.
//!
//! This crate ships two [`voxora_traits::ModelSource`]
//! implementations targeted at offline-friendly consumer flows:
//!
//! - [`LocalSource`] — resolves a model id (e.g.
//!   `"some-org/some-repo/model.bin"`) against an already-vendored
//!   directory on disk. No network, no tokio, no reqwest, no
//!   `voxora-config`. Reads only what the caller asks for.
//! - [`ChainedSource`] — first-hit-wins adapter that tries a
//!   primary source and falls back to a secondary source on
//!   [`voxora_traits::AsrError::ModelNotFound`]. Useful for the
//!   "local first, Hugging Face on miss" composition.
//!
//! # Use cases
//!
//! 1. **Vendored weights** — operators who ship model artefacts
//!    with their binary can resolve them via `LocalSource` without
//!    a HF token, a cache directory, or outbound network. The
//!    matching engine (e.g. `voxora_whisper::WhisperEngine` or
//!    `voxora_qwen3asr::QwenAsrEngine`) loads the result of
//!    `LocalSource::resolve` directly from
//!    `ModelDir::entry` — no registry needed. Neither engine
//!    crate is re-exported from `voxora-local`; reference them
//!    directly from your `Cargo.toml` when you wire an engine
//!    against a `LocalSource`.
//! 2. **Hermetic CI** — every test lane that wants a deterministic
//!    `ModelSource` can wire a `LocalSource` against a `tempdir`
//!    pre-populated by `cargo` build scripts. No wiremock, no live
//!    HF download, no flake.
//! 3. **Local-first registry** — wrap
//!    `ChainedSource::new(LocalSource::new("/srv/models"),
//!    HuggingFaceSource::new()?)` and hand the result to
//!    `voxora_registry::Registry::new`. Local artefacts win;
//!    only misses go on the wire.
//!
//! # Example
//!
//! ```no_run
//! use voxora_local::LocalSource;
//! use voxora_traits::{ModelSource, ResolveOptions};
//!
//! # async fn run() -> Result<(), voxora_traits::AsrError> {
//! let source = LocalSource::new("/srv/models");
//! let dir = source
//!     .resolve("some-org/some-repo/model.bin", &ResolveOptions::default())
//!     .await?;
//! println!("model at {}", dir.entry.expect("entry").display());
//! # Ok(()) }
//! ```
//!
//! # Engine integration
//!
//! Engines consume [`voxora_traits::ModelDir`] directly:
//!
//! - `voxora_whisper::WhisperEngine::load` reads
//!   `ModelDir::entry` (or falls back to lex-sorting `ModelDir::path`
//!   if `entry` is `None`). A `LocalSource`-resolved dir works
//!   unchanged.
//! - `voxora_qwen3asr::QwenAsrEngine::load` reads
//!   `ModelDir::path` and looks for the canonical Qwen3-ASR file
//!   trio inside it; pass the resolved `path` straight through.
//!   Neither crate is re-exported from `voxora-local`; reference
//!   them directly from your `Cargo.toml` if you wire an engine
//!   against a `LocalSource`.
//!
//! # Limitations
//!
//! - **No recursive walker.** `LocalSource::resolve` joins
//!   `model_id` against `root` with a single `PathBuf::join` call
//!   and checks the resulting path with `symlink_metadata` (closes
//!   #143, EPIC #148). Vendored trees that spread weights across
//!   nested directories are out of scope; callers wanting that
//!   should compose their own walker.
//! - **Path-traversal & symlink guards.** `LocalSource::resolve`
//!   refuses `..` components, symlinks under `root` (even if the
//!   target is a real file the caller had every right to read —
//!   the operator's local root is treated as a trust boundary),
//!   and ids longer than 4 KiB. `O_NOFOLLOW` closes the
//!   symlink-swap TOCTOU on Unix. The configured `local_root` is
//!   deliberately not echoed in the error envelope (closes #144).
//!   These guards are tight by design; if a caller needs a less
//!   strict surface, the registry layer above already wraps the
//!   source in a parser that rejects the same shapes upstream.
//! - **Registry integration is opt-in.** The built-in descriptors
//!   shipped by `voxora-registry` still accept HF ids only by
//!   default; for a chain that resolves `SourceKind::Local` first
//!   with HF on miss, use
//!   `voxora_registry::Registry::with_builtin_descriptors_and_chained_source(local_root)`
//!   behind the `voxora-registry` `local` feature (closes #120).
//!
//! # Cargo features
//!
//! This crate has no Cargo features. The whole point of the
//! `voxora-local` crate is to stay minimal — adding a feature would
//! re-introduce the HF-token / cache-root cascade that
//! `voxora-hf` already owns.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod source;

pub use source::{ChainedSource, LocalSource};
