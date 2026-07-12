//! Filesystem layout for the on-disk model cache.
//!
//! Layout per the Phase 2 roadmap:
//!
//! ```text
//! $XDG_CACHE_HOME/voxora/models/huggingface/<org>/<name>/<revision>/
//! ├── .complete             ← marker; presence means all files present
//! ├── .lock                 ← advisory lockfile during active download
//! ├── .capabilities.json    ← cached ModelCapabilities (optional)
//! ├── config.json
//! ├── tokenizer.json
//! ├── preprocessor_config.json
//! ├── model.safetensors.index.json
//! ├── model-00001-of-00002.safetensors
//! └── model-00002-of-00002.safetensors
//! ```
//!
//! All downloads happen in two phases:
//!
//! 1. Each file is written to `<file>.partial` via the streaming HTTP
//!    path in [`crate::client::HfClient::get_to_file`]. The partial
//!    file is `fsync`-ed and atomically renamed over the target path.
//! 2. Once every required file is present, the empty `.complete`
//!    marker is written last.
//!
//! A crash between (1) and (2) leaves the directory without a marker,
//! and the next [`crate::source::HuggingFaceSource::resolve`] call
//! will resume from the files that already exist.

use std::path::{Path, PathBuf};

use crate::error::HfError;

const COMPLETE_MARKER: &str = ".complete";
#[allow(dead_code)] // planned for Phase 2.x (advisory download locks)
const LOCK_FILE: &str = ".lock";
const CAPABILITIES_CACHE: &str = ".capabilities.json";
const SOURCE_DIR: &str = "huggingface";

/// Resolve the cache root, honouring `XDG_CACHE_HOME`.
pub(crate) fn default_cache_root() -> PathBuf {
    if let Ok(custom) = std::env::var("VOXORA_CACHE_DIR") {
        return PathBuf::from(custom);
    }
    let base = dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".cache"));
    base.join("voxora").join("models").join(SOURCE_DIR)
}

/// Compute the directory for `(model_id, revision)` inside `cache_root`.
///
/// `model_id` must already be in `org/name` form (validated upstream).
pub(crate) fn model_dir(cache_root: &Path, model_id: &str, revision: &str) -> PathBuf {
    cache_root.join(model_id).join(revision)
}

/// True iff the marker file exists, meaning the previous download
/// finished successfully.
pub(crate) fn is_complete(dir: &Path) -> bool {
    dir.join(COMPLETE_MARKER).is_file()
}

/// Drop the `.complete` marker without touching downloaded files.
///
/// Used before resuming a partially-cached download.
pub(crate) fn clear_marker(dir: &Path) -> Result<(), HfError> {
    let marker = dir.join(COMPLETE_MARKER);
    if marker.exists() {
        std::fs::remove_file(&marker).map_err(|e| HfError::Io {
            path: marker,
            message: "remove marker".into(),
            source: e,
        })?;
    }
    Ok(())
}

/// Write the empty `.complete` marker.
pub(crate) fn mark_complete(dir: &Path) -> Result<(), HfError> {
    let marker = dir.join(COMPLETE_MARKER);
    std::fs::write(&marker, b"").map_err(|e| HfError::Io {
        path: marker,
        message: "write marker".into(),
        source: e,
    })?;
    Ok(())
}

/// Path to the advisory lockfile. Reserved for the cross-process
/// download lock planned for a follow-up phase.
#[allow(dead_code)]
pub(crate) fn lock_path(dir: &Path) -> PathBuf {
    dir.join(LOCK_FILE)
}

/// Path to the capabilities cache file.
pub(crate) fn capabilities_cache_path(dir: &Path) -> PathBuf {
    dir.join(CAPABILITIES_CACHE)
}

/// Ensure `dir` exists. If `dir` is missing, create it. If `dir`
/// exists but is incomplete (no marker), it is reused for a resume.
///
/// Returns `true` if a fresh directory was created, `false` if we
/// are resuming an incomplete download.
pub(crate) fn ensure_dir(dir: &Path) -> Result<bool, HfError> {
    if dir.exists() {
        if !dir.is_dir() {
            return Err(HfError::Io {
                path: dir.to_path_buf(),
                message: "exists but is not a directory".into(),
                source: std::io::Error::new(std::io::ErrorKind::AlreadyExists, "not a dir"),
            });
        }
        // Either complete (caller will skip) or incomplete (resume).
        return Ok(false);
    }
    std::fs::create_dir_all(dir).map_err(|e| HfError::Io {
        path: dir.to_path_buf(),
        message: "create_dir_all".into(),
        source: e,
    })?;
    Ok(true)
}

/// Remove a directory and all its contents (used for rollback on
/// failure). Kept as `#[allow(dead_code)]` because the public
/// [`crate::source::HuggingFaceSource`] does not yet roll back the
/// whole dir on partial failure — it relies on the marker file as a
/// signal instead.
#[allow(dead_code)]
pub(crate) fn rollback(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

/// Remove every `.partial` sibling left over by an interrupted
/// download. Called after a successful complete so the next run sees
/// no debris.
pub(crate) fn cleanup_partials(dir: &Path) -> Result<(), HfError> {
    let entries = std::fs::read_dir(dir).map_err(|e| HfError::Io {
        path: dir.to_path_buf(),
        message: "read_dir".into(),
        source: e,
    })?;
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) == Some("partial") {
            std::fs::remove_file(&p).map_err(|e| HfError::Io {
                path: p,
                message: "remove partial".into(),
                source: e,
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        tempfile::tempdir().unwrap().keep()
    }

    #[test]
    fn default_cache_root_uses_voxora_subdir() {
        let root = default_cache_root();
        assert!(root.ends_with("voxora/models/huggingface"));
    }

    #[test]
    fn model_dir_layout_is_org_name_revision() {
        let root = Path::new("/cache");
        assert_eq!(
            model_dir(root, "Qwen/Qwen3-ASR-0.6B", "main"),
            PathBuf::from("/cache/Qwen/Qwen3-ASR-0.6B/main")
        );
    }

    #[test]
    fn ensure_dir_creates_missing() {
        let dir = tmp().join("a/b/c");
        let created = ensure_dir(&dir).unwrap();
        assert!(created);
        assert!(dir.is_dir());
    }

    #[test]
    fn ensure_dir_reuses_existing() {
        let dir = tmp().join("d");
        ensure_dir(&dir).unwrap();
        let created = ensure_dir(&dir).unwrap();
        assert!(!created);
    }

    #[test]
    fn ensure_dir_rejects_file_at_path() {
        let dir = tmp().join("file");
        std::fs::write(&dir, b"x").unwrap();
        let err = ensure_dir(&dir).unwrap_err();
        assert!(matches!(err, HfError::Io { .. }));
    }

    #[test]
    fn marker_lifecycle() {
        let dir = tmp();
        assert!(!is_complete(&dir));
        mark_complete(&dir).unwrap();
        assert!(is_complete(&dir));
        clear_marker(&dir).unwrap();
        assert!(!is_complete(&dir));
    }

    #[test]
    fn cleanup_partials_removes_only_partials() {
        let dir = tmp();
        std::fs::write(dir.join("config.json"), b"{}").unwrap();
        std::fs::write(dir.join("model.partial"), b"abc").unwrap();
        std::fs::write(dir.join("model.safetensors.partial"), b"xyz").unwrap();
        cleanup_partials(&dir).unwrap();
        assert!(dir.join("config.json").exists());
        assert!(!dir.join("model.partial").exists());
        assert!(!dir.join("model.safetensors.partial").exists());
    }

    #[test]
    fn rollback_removes_directory() {
        let dir = tmp().join("victim");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("file"), b"x").unwrap();
        rollback(&dir);
        assert!(!dir.exists());
    }

    #[test]
    fn lock_and_capabilities_paths() {
        let dir = Path::new("/cache/Qwen/Qwen3-ASR-0.6B/main");
        assert_eq!(
            lock_path(dir),
            PathBuf::from("/cache/Qwen/Qwen3-ASR-0.6B/main/.lock")
        );
        assert_eq!(
            capabilities_cache_path(dir),
            PathBuf::from("/cache/Qwen/Qwen3-ASR-0.6B/main/.capabilities.json")
        );
    }
}
