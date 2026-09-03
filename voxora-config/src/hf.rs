//! Hugging Face configuration.
//!
//! Token cascade (first non-empty wins):
//! 1. `VoxoraConfig::hf.token` (explicit override)
//! 2. `VOXORA_HF_TOKEN` env var
//! 3. `HF_TOKEN` env var
//! 4. `HUGGING_FACE_HUB_TOKEN` env var (legacy alias)
//! 5. None (anonymous)
//!
//! Base URL cascade (first non-empty wins):
//! 1. `VoxoraConfig::hf.base_url` (explicit override)
//! 2. `VOXORA_HF_BASE_URL` env var
//! 3. `https://huggingface.co` (default)
//!
//! Default revision cascade (first non-empty wins):
//! 1. `VoxoraConfig::hf.default_revision` (explicit override)
//! 2. `VOXORA_HF_REVISION` env var
//! 3. `main` (default)

use serde::{Deserialize, Serialize};

/// How voxora talks to the Hugging Face Hub.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct HfConfig {
    /// Explicit bearer token. `None` defers to the env vars.
    pub token: Option<String>,
    /// Explicit base URL. `None` defers to the env var, then to
    /// `https://huggingface.co`.
    pub base_url: Option<String>,
    /// Explicit default revision. `None` defers to the env var, then
    /// to `main`.
    pub default_revision: Option<String>,
}

impl HfConfig {
    /// Build an [`HfConfig`] from its three fields. Provided because
    /// the type is `#[non_exhaustive]` and cannot be built with a
    /// struct expression from outside this crate.
    pub fn new(
        token: Option<String>,
        base_url: Option<String>,
        default_revision: Option<String>,
    ) -> Self {
        Self {
            token,
            base_url,
            default_revision,
        }
    }

    /// Resolve the bearer token, or `None` for anonymous access.
    pub fn token(&self) -> Option<String> {
        if let Some(t) = &self.token {
            if !t.is_empty() {
                return Some(t.clone());
            }
        }
        for var in [
            crate::env::VOXORA_HF_TOKEN,
            "HF_TOKEN",
            "HUGGING_FACE_HUB_TOKEN",
        ] {
            if let Ok(t) = std::env::var(var) {
                if !t.is_empty() {
                    return Some(t);
                }
            }
        }
        None
    }

    /// Resolve the Hub base URL.
    pub fn base_url(&self) -> String {
        if let Some(u) = &self.base_url {
            if !u.is_empty() {
                return u.clone();
            }
        }
        if let Ok(u) = std::env::var(crate::env::VOXORA_HF_BASE_URL) {
            if !u.is_empty() {
                return u;
            }
        }
        "https://huggingface.co".to_string()
    }

    /// Resolve the revision used when the caller does not pin one.
    pub fn default_revision(&self) -> String {
        self.default_revision
            .clone()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                std::env::var(crate::env::VOXORA_HF_REVISION)
                    .ok()
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| "main".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_base_url_is_huggingface_co() {
        assert_eq!(HfConfig::default().base_url(), "https://huggingface.co");
    }

    #[test]
    fn default_revision_is_main() {
        assert_eq!(HfConfig::default().default_revision(), "main");
    }

    #[test]
    fn explicit_base_url_wins() {
        let cfg = HfConfig {
            base_url: Some("http://localhost:8080".into()),
            ..Default::default()
        };
        assert_eq!(cfg.base_url(), "http://localhost:8080");
    }

    #[test]
    fn explicit_token_wins_over_env() {
        let cfg = HfConfig {
            token: Some("explicit-token".into()),
            ..Default::default()
        };
        assert_eq!(cfg.token().as_deref(), Some("explicit-token"));
    }

    #[test]
    fn explicit_default_revision_wins() {
        let cfg = HfConfig {
            default_revision: Some("v0.0.1".into()),
            ..Default::default()
        };
        assert_eq!(cfg.default_revision(), "v0.0.1");
    }

    #[test]
    fn new_matches_struct_expression() {
        let cfg = HfConfig::new(Some("t".into()), Some("u".into()), Some("r".into()));
        assert_eq!(
            cfg,
            HfConfig {
                token: Some("t".into()),
                base_url: Some("u".into()),
                default_revision: Some("r".into()),
            }
        );
    }
}
