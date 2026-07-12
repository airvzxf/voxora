#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Hugging Face model source for voxora.
//!
//! This crate implements [`voxora_core::ModelSource`] against the
//! public Hugging Face Hub REST API. It turns a model identifier such
//! as `"Qwen/Qwen3-ASR-0.6B"` into a [`voxora_core::ModelDir`] on
//! disk, downloading only what is missing and verifying integrity
//! when the repo ships a sidecar checksum.
//!
//! # Example
//!
//! ```no_run
//! use voxora_core::ResolveOptions;
//! use voxora_hf::HuggingFaceSource;
//!
//! # async fn run() -> Result<(), voxora_core::AsrError> {
//! let source = HuggingFaceSource::new()?;
//! let dir = source
//!     .resolve("Qwen/Qwen3-ASR-0.6B", &ResolveOptions::default())
//!     .await?;
//! println!("model cached at {}", dir.path.display());
//! # Ok(()) }
//! ```
//!
//! # Caching
//!
//! Files land under
//! `$XDG_CACHE_HOME/voxora/models/huggingface/<org>/<name>/<revision>/`
//! with a `.complete` marker file written **last**. A second call to
//! [`voxora_core::ModelSource::resolve`] for the same `(model_id, revision)`
//! returns immediately when the marker is present.
//!
//! # Auth
//!
//! Tokens are resolved in this order, the first non-empty wins:
//!
//! 1. [`voxora_core::ResolveOptions::token`]
//! 2. `HF_TOKEN` environment variable
//! 3. `HUGGING_FACE_HUB_TOKEN` environment variable (legacy alias)
//! 4. Anonymous
//!
//! # Quantization
//!
//! The crate detects the dtype from the model's `config.json`
//! (`torch_dtype` field) and from the file name for GGUF repositories
//! (e.g. `ggml-base.bin.q4_K_M`). The caller's
//! [`voxora_core::QuantizationPreference`] is consulted only when the
//! repo offers a choice.

mod api;
mod cache;
mod capabilities;
mod client;
mod error;
mod known_models;
mod quantization;
mod source;

pub use source::{HuggingFaceSource, HuggingFaceSourceBuilder};
