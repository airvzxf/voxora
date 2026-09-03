//! Cache directory configuration.
//!
//! Cascade (first non-empty wins):
//! 1. `VoxoraConfig::cache.root` (explicit override)
//! 2. `VOXORA_CACHE_DIR` env var
//! 3. `$XDG_CACHE_HOME/voxora` (or `dirs::cache_dir()/voxora`)
//! 4. `.voxora-cache` (relative; last-resort cross-platform fallback)

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Where voxora keeps downloaded models on disk.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct CacheConfig {
    /// Explicit override. `None` defers to env / XDG / fallback.
    pub root: Option<PathBuf>,
}

impl CacheConfig {
    /// Build a [`CacheConfig`] from its field. Provided because the
    /// type is `#[non_exhaustive]` and cannot be built with a struct
    /// expression from outside this crate.
    pub fn new(root: Option<PathBuf>) -> Self {
        Self { root }
    }

    /// Resolve the cache directory honouring the cascade documented at
    /// the top of this module.
    pub fn resolve(&self) -> PathBuf {
        if let Some(p) = &self.root {
            return p.clone();
        }
        if let Ok(custom) = std::env::var(crate::env::VOXORA_CACHE_DIR) {
            if !custom.is_empty() {
                return PathBuf::from(custom);
            }
        }
        if let Some(base) = dirs::cache_dir() {
            return base.join("voxora");
        }
        // Last-resort fallback: relative path so the code still works
        // even on platforms without $HOME (rare). On Linux/macOS/Windows
        // dirs::cache_dir() should never be None.
        PathBuf::from(".voxora-cache")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_resolves_via_xdg_or_fallback() {
        let cfg = CacheConfig::default();
        let resolved = cfg.resolve();
        assert!(!resolved.as_os_str().is_empty());
        // Either XDG-based or the relative fallback — both end in
        // "voxora" (XDG) or "voxora-cache" (last-resort).
        let s = resolved.to_string_lossy();
        assert!(
            s.ends_with("voxora") || s.ends_with("voxora-cache"),
            "got {s:?}"
        );
    }

    #[test]
    fn explicit_root_wins() {
        let cfg = CacheConfig {
            root: Some(PathBuf::from("/tmp/explicit")),
        };
        assert_eq!(cfg.resolve(), PathBuf::from("/tmp/explicit"));
    }

    #[test]
    fn new_matches_struct_expression() {
        let cfg = CacheConfig::new(Some(PathBuf::from("/tmp/via-new")));
        assert_eq!(
            cfg,
            CacheConfig {
                root: Some(PathBuf::from("/tmp/via-new")),
            }
        );
    }
}
