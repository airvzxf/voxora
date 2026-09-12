//! MiniMax configuration.
//!
//! Token cascade (first non-empty wins):
//! 1. `VoxoraConfig::minimax.api_key` (explicit override)
//! 2. `VOXORA_MINIMAX_API_KEY` env var
//! 3. `MINIMAX_API_KEY` env var (canonical name)
//! 4. `None` (no auth — calls will fail with `Config` error upstream)
//!
//! Endpoint cascade (first non-empty wins):
//! 1. `VoxoraConfig::minimax.endpoint` (explicit override)
//! 2. `VOXORA_MINIMAX_ENDPOINT` env var
//! 3. `https://api.minimax.io` (default)
//!
//! Closes #156 (part of EPIC #153, the
//! `voxora-minimax` hosted-API track). Mirrors the existing
//! `HfConfig` pattern.

use serde::{Deserialize, Serialize};

/// How voxora talks to the MiniMax ASR API.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct MiniMaxConfig {
    /// Explicit bearer token. `None` defers to the env vars.
    pub api_key: Option<String>,
    /// Explicit API endpoint. `None` defers to the env var, then to
    /// `https://api.minimax.io`.
    pub endpoint: Option<String>,
}

impl MiniMaxConfig {
    /// Build a [`MiniMaxConfig`] from its two fields. Provided
    /// because the type is `#[non_exhaustive]` and cannot be built
    /// with a struct expression from outside this crate.
    pub fn new(api_key: Option<String>, endpoint: Option<String>) -> Self {
        Self { api_key, endpoint }
    }

    /// Resolve the bearer token honouring the cascade, or `None`
    /// when no env var is set and no explicit value was supplied.
    pub fn api_key(&self) -> Option<String> {
        if let Some(t) = &self.api_key
            && !t.is_empty()
        {
            return Some(t.clone());
        }
        for var in [
            crate::env::VOXORA_MINIMAX_API_KEY,
            crate::env::MINIMAX_API_KEY,
        ] {
            if let Ok(t) = std::env::var(var)
                && !t.is_empty()
            {
                return Some(t);
            }
        }
        None
    }

    /// Resolve the API endpoint honouring the cascade.
    pub fn endpoint(&self) -> String {
        if let Some(u) = &self.endpoint
            && !u.is_empty()
        {
            return u.clone();
        }
        if let Ok(u) = std::env::var(crate::env::VOXORA_MINIMAX_ENDPOINT)
            && !u.is_empty()
        {
            return u;
        }
        crate::minimax::DEFAULT_MINIMAX_ENDPOINT.to_string()
    }
}

/// Canonical MiniMax API endpoint.
pub const DEFAULT_MINIMAX_ENDPOINT: &str = "https://api.minimax.io";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_endpoint_is_minimax_io() {
        assert_eq!(
            MiniMaxConfig::default().endpoint(),
            DEFAULT_MINIMAX_ENDPOINT
        );
        assert_eq!(
            DEFAULT_MINIMAX_ENDPOINT, "https://api.minimax.io",
            "endpoint constant drifted from the documented default"
        );
    }

    #[test]
    fn explicit_endpoint_wins_over_default() {
        let cfg = MiniMaxConfig {
            endpoint: Some("http://localhost:9999".into()),
            ..Default::default()
        };
        assert_eq!(cfg.endpoint(), "http://localhost:9999");
    }

    #[test]
    fn empty_explicit_endpoint_falls_through_to_default() {
        let cfg = MiniMaxConfig {
            endpoint: Some(String::new()),
            ..Default::default()
        };
        assert_eq!(cfg.endpoint(), DEFAULT_MINIMAX_ENDPOINT);
    }

    #[test]
    fn explicit_api_key_wins() {
        let cfg = MiniMaxConfig {
            api_key: Some("explicit-token".into()),
            ..Default::default()
        };
        assert_eq!(cfg.api_key().as_deref(), Some("explicit-token"));
    }

    #[test]
    fn empty_explicit_api_key_falls_through() {
        let cfg = MiniMaxConfig {
            api_key: Some(String::new()),
            ..Default::default()
        };
        assert!(cfg.api_key().is_none(), "empty string must not win");
    }

    #[test]
    fn new_matches_struct_expression() {
        let cfg = MiniMaxConfig::new(Some("k".into()), Some("e".into()));
        assert_eq!(
            cfg,
            MiniMaxConfig {
                api_key: Some("k".into()),
                endpoint: Some("e".into()),
            }
        );
    }
}
