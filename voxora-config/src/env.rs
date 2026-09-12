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

/// `VOXORA_MINIMAX_API_KEY` — explicit MiniMax bearer token override.
/// Part of the EPIC #153 hosted-API track (`voxora-minimax`,
/// closes #156).
pub const VOXORA_MINIMAX_API_KEY: &str = "VOXORA_MINIMAX_API_KEY";

/// `MINIMAX_API_KEY` — canonical MiniMax env var name (alias for
/// `VOXORA_MINIMAX_API_KEY`). Part of EPIC #153 (closes #156).
pub const MINIMAX_API_KEY: &str = "MINIMAX_API_KEY";

/// `VOXORA_MINIMAX_ENDPOINT` — explicit MiniMax API endpoint override.
/// Useful for tests pointing at a mock server. Default endpoint
/// (when the var is unset) is `https://api.minimax.io`. Part of
/// EPIC #153 (closes #156).
pub const VOXORA_MINIMAX_ENDPOINT: &str = "VOXORA_MINIMAX_ENDPOINT";
