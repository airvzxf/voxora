#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Single source of truth for every voxora runtime setting.
//!
//! Replaces the ad-hoc cascade currently duplicated across
//! `voxora-cli/src/args.rs::resolve_hf_cache_dir` and
//! `voxora-hf/src/cache.rs::default_cache_root`.
//!
//! Settings come from three layers — built-in defaults, an optional
//! TOML file, and the environment — and every getter resolves them in
//! the same order, first non-empty wins:
//!
//!   1. An explicit value on the config itself: either set by the
//!      caller ([`CacheConfig::root`], [`HfConfig::token`], …) or
//!      loaded from a TOML file via [`VoxoraConfig::from_file`].
//!   2. The matching `VOXORA_*` environment variable, listed in
//!      [`mod@env`] (plus the `HF_TOKEN` / `HUGGING_FACE_HUB_TOKEN`
//!      aliases for the token).
//!   3. The built-in default (`VoxoraConfig::default`).
//!
//! ASR-specific: this crate only models configuration concerns that
//! exist in the voxora speech-recognition stack (cache dir, HF token,
//! HF base URL, default revision). It does not model LLM / vision /
//! multimodal configuration.
//!
//! # Example
//!
//! ```no_run
//! use voxora_config::VoxoraConfig;
//!
//! # fn run() -> Result<(), voxora_config::ConfigError> {
//! let cfg = VoxoraConfig::from_file(std::path::Path::new("voxora.toml"))?;
//! println!("cache root: {}", cfg.cache_root().display());
//! println!("hub: {}", cfg.hf_base_url());
//! # Ok(()) }
//! ```

pub mod env;
pub mod file;

mod cache;
mod error;
mod hf;

pub use cache::CacheConfig;
pub use error::ConfigError;
pub use hf::HfConfig;

use serde::{Deserialize, Serialize};

/// Top-level voxora configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct VoxoraConfig {
    /// Cache directory configuration.
    pub cache: CacheConfig,
    /// Hugging Face source configuration.
    pub hf: HfConfig,
}

impl VoxoraConfig {
    /// Build a [`VoxoraConfig`] from its two sections. Provided because
    /// the type is `#[non_exhaustive]` and cannot be built with a
    /// struct expression from outside this crate.
    pub fn new(cache: CacheConfig, hf: HfConfig) -> Self {
        Self { cache, hf }
    }

    /// Resolve the cache directory honouring the cascade.
    pub fn cache_root(&self) -> std::path::PathBuf {
        self.cache.resolve()
    }

    /// Resolve the HF bearer token honouring the cascade.
    pub fn hf_token(&self) -> Option<String> {
        self.hf.token()
    }

    /// Resolve the HF base URL honouring the cascade.
    pub fn hf_base_url(&self) -> String {
        self.hf.base_url()
    }

    /// Resolve the default HF revision honouring the cascade.
    pub fn hf_default_revision(&self) -> String {
        self.hf.default_revision()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_roundtrip_via_toml() {
        let cfg = VoxoraConfig::default();
        let text = toml::to_string(&cfg).expect("toml::to_string");
        let parsed = VoxoraConfig::from_str(&text, std::path::Path::new("inline")).expect("parse");
        assert_eq!(cfg, parsed);
    }

    #[test]
    fn unknown_field_is_rejected() {
        let bad = r#"
            cache.root = "/tmp/cache"
            hf.token = "abc"
            bogus_field = "should-error"
        "#;
        let err = VoxoraConfig::from_str(bad, std::path::Path::new("inline"))
            .expect_err("deny_unknown_fields");
        assert!(matches!(err, ConfigError::FileParse { .. }));
    }

    #[test]
    fn cache_root_matches_inner_resolve() {
        let cfg = VoxoraConfig::default();
        assert_eq!(cfg.cache_root(), cfg.cache.resolve());
    }

    #[test]
    fn hf_token_propagates_from_inner() {
        let cfg = VoxoraConfig {
            hf: HfConfig {
                token: Some("xyz".into()),
                ..HfConfig::default()
            },
            ..VoxoraConfig::default()
        };
        assert_eq!(cfg.hf_token().as_deref(), Some("xyz"));
    }

    #[test]
    fn new_matches_struct_expression() {
        let cache = CacheConfig::new(Some(std::path::PathBuf::from("/tmp/c")));
        let hf = HfConfig::new(Some("t".into()), None, None);
        assert_eq!(
            VoxoraConfig::new(cache.clone(), hf.clone()),
            VoxoraConfig { cache, hf }
        );
    }
}
