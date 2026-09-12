//! MiniMax hosted ASR engine adapter for voxora.
//!
//! This crate implements [`voxora_traits::AsrEngine`] against the
//! [MiniMax `/v1/speech_to_text` API](https://platform.minimax.io/docs/api-reference/speech-to-text.md).
//! It is the **first hosted-API** engine adapter in the voxora
//! workspace — every prior engine (`voxora-whisper`,
//! `voxora-qwen3asr`) loads a model file from disk and runs
//! inference locally. MiniMax runs the inference server-side; the
//! engine wraps the bearer-token-authenticated
//! `multipart/form-data` upload and parses the `verbose_json`
//! response.
//!
//! # Loading the engine
//!
//! The primary constructor is [`MiniMaxEngine::new`], which takes
//! a [`MiniMaxConfig`] with an API key:
//!
//! ```no_run
//! use voxora_minimax::{MiniMaxConfig, MiniMaxEngine};
//! use voxora_traits::{AsrEngine, TranscribeOptions};
//!
//! # fn run() -> Result<(), voxora_traits::AsrError> {
//! let config = MiniMaxConfig::new("sk-...")?;
//! let engine = MiniMaxEngine::new(config)?;
//!
//! let samples: Vec<f32> = vec![0.0; 16_000]; // 1 s of silence @ 16 kHz
//! let result = engine.transcribe(&samples, &TranscribeOptions::default())?;
//! println!("{}", result.text);
//! # Ok(()) }
//! ```
//!
//! # Authentication
//!
//! MiniMax uses bearer-token auth. The token is read from any of:
//!
//! 1. The explicit `MiniMaxConfig::api_key` field (preferred for
//!    library callers — the key never enters process env).
//! 2. `VOXORA_MINIMAX_API_KEY` env var.
//! 3. `MINIMAX_API_KEY` env var (canonical alias).
//!
//! All three are consulted by
//! [`voxora_config::VoxoraConfig::minimax_api_key`]; the engine
//! itself only sees the resolved value.
//!
//! # Sample-rate contract
//!
//! **Soft contract.** The engine accepts any sample rate / channel
//! layout from the caller and wraps the input `&[f32]` in an
//! in-memory WAV (`f32` LE PCM) before upload. MiniMax docs
//! recommend mono 16 kHz; high-sample-rate stereo uploads eat the
//! 50 MB / 500 s server caps without improving transcription
//! quality. The engine does not enforce the rate — the caller is
//! responsible for downsampling / channel mixing. No trait change.
//!
//! # Feature flags
//!
//! | Flag | Default | Enables |
//! |---|---|---|
//! | (none) | yes | the base `MiniMaxConfig` + `MiniMaxEngine` surface |
//! | `engine-adapter` | no | `MiniMaxAdapter` (`voxora_engine::EngineAdapter` impl) |
//!
//! No GPU backend features — compute happens server-side.
//! No `hf` feature — MiniMax has no on-disk model.
//!
//! # Error envelope
//!
//! On failure the MiniMax endpoint returns an
//! [OpenAI-style envelope](https://platform.minimax.io/docs/api-reference/speech-to-text.md)
//! with `type: error`, a nested `error: { type, message, http_code }`,
//! and a top-level `request_id`. The full mapping onto
//! [`voxora_traits::AsrError`] — the full mapping lives in the private `client` module.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod client;
mod config;
mod engine;
mod language;
mod params;
mod wav;

pub use client::{AsrResp, OaiError, OaiErrorDetail};
pub use config::MiniMaxConfig;
pub use engine::MiniMaxEngine;
pub use language::{known_languages_bcp47, validate_lang_bcp47};

#[cfg(feature = "engine-adapter")]
#[cfg_attr(docsrs, doc(cfg(feature = "engine-adapter")))]
pub use engine::MiniMaxAdapter;
