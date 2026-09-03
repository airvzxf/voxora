//! Centralised list of `VOXORA_*` environment variables.
//!
//! Documenting the full set in one place so consumers and tests can
//! grep for them instead of string-typing in each callsite.

/// `VOXORA_CACHE_DIR` — explicit cache directory override.
pub const VOXORA_CACHE_DIR: &str = "VOXORA_CACHE_DIR";

/// `VOXORA_HF_TOKEN` — explicit Hugging Face token override.
pub const VOXORA_HF_TOKEN: &str = "VOXORA_HF_TOKEN";

/// `VOXORA_HF_BASE_URL` — explicit Hugging Face base URL override
/// (mostly useful for tests pointing at a local mock server).
pub const VOXORA_HF_BASE_URL: &str = "VOXORA_HF_BASE_URL";

/// `VOXORA_HF_REVISION` — default revision used by voxora-hf when
/// `ResolveOptions::revision` is `None`.
pub const VOXORA_HF_REVISION: &str = "VOXORA_HF_REVISION";
