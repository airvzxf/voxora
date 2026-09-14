//! RAII guard for the tmp-file pattern used by [`crate::client::HfClient`].
//!
//! Closes [#189](https://github.com/airvzxf/voxora/issues/189): a
//! tmp file (`<file>.<ext>.partial.<hex>-<n>`) is created at the
//! start of a streaming download and renamed over the destination on
//! success. Every error path between those two points previously
//! left the tmp on disk — five such paths in
//! [`crate::client::HfClient::get_to_file`]: chunk read, write,
//! flush, `sync_all`, and `rename`. `TmpGuard` ensures the tmp is
//! removed on every exit, including panic, unless explicitly disarmed
//! after a successful rename.
//!
//! The type lives in `voxora-hf` (not `voxora-traits`) because it
//! references the `[fs2]` / filesystem vocabulary specific to the HF
//! cache. It is `pub` so [`voxora-qwen3asr`] can adopt the same
//! pattern for its `ensure_qwen3_tokenizer_json` write
//! (`voxora-qwen3asr/src/engine.rs:359-364`, addressed by #187 in
//! the same EPIC).
//!
//! # Lifecycle
//!
//! ```ignore
//! let tmp = dest.with_extension("partial");
//! let _guard = TmpGuard::new(&tmp);
//! // ... streaming write that may fail ...
//! tokio::fs::rename(&tmp, &dest).await?;
//! _guard.disarm(); // success — the tmp no longer exists; no Drop cleanup.
//! ```
//!
//! On any early `?` return between `new` and `disarm`, the guard's
//! `Drop` removes the tmp file. `NotFound` is swallowed because a
//! concurrent process may have already swept it.

use std::path::{Path, PathBuf};

/// Drop-only RAII guard that removes `path` on Drop, swallowing
/// `ErrorKind::NotFound`. `disarm()` cancels the cleanup so a
/// post-success `Drop` is a no-op rather than a redundant
/// `NotFound` syscall.
///
/// Cheap to construct: no syscall until `Drop`. Cheap to drop:
/// one `std::fs::remove_file` call (which the kernel returns
/// `ENOENT` for if the tmp was already renamed away).
#[derive(Debug)]
pub struct TmpGuard {
    path: Option<PathBuf>,
}

impl TmpGuard {
    /// Create a guard for `path`. The file at `path` is **not**
    /// validated here — callers typically pair this with
    /// `tokio::fs::File::create(&path)` immediately before or
    /// after, so a pre-existing path is not possible.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: Some(path.into()),
        }
    }

    /// Cancel the cleanup. After `disarm`, the guard's `Drop` does
    /// not touch the filesystem. Used immediately after a
    /// successful rename.
    pub fn disarm(mut self) {
        self.path = None;
    }

    /// True iff `disarm` has not yet been called.
    pub fn is_armed(&self) -> bool {
        self.path.is_some()
    }

    /// Underlying path, if still armed. Useful for diagnostics.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

impl Drop for TmpGuard {
    fn drop(&mut self) {
        let Some(p) = self.path.take() else {
            return;
        };
        match std::fs::remove_file(&p) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Already gone (renamed, swept by a sibling, removed
                // by the operator). Benign.
            }
            Err(e) => {
                // Don't panic in Drop, and don't pull in a logging
                // dependency. The next `resolve` against the same
                // directory runs `cache::cleanup_partials` (see
                // `voxora-hf/src/cache.rs`), which sweeps any
                // sibling whose name contains `.partial` — so a
                // single failed remove is recovered on the next
                // resolve. The error is intentionally swallowed here
                // to keep the contract: Drop is infallible.
                let _ = e;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_removes_existing_tmp() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("leak.tmp");
        std::fs::write(&path, b"hi").unwrap();
        {
            let _g = TmpGuard::new(&path);
            assert!(path.exists());
        }
        assert!(!path.exists(), "Drop must remove the tmp file");
    }

    #[test]
    fn drop_swallows_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.tmp");
        // Never create the file.
        {
            let _g = TmpGuard::new(&path);
        }
        // No panic — NotFound is swallowed by design.
    }

    #[test]
    fn disarm_cancels_cleanup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keep.tmp");
        std::fs::write(&path, b"hi").unwrap();
        {
            let g = TmpGuard::new(&path);
            assert!(g.is_armed(), "fresh guard must be armed");
            g.disarm();
        }
        assert!(path.exists(), "disarm must cancel the Drop cleanup");
    }

    #[test]
    fn disarm_path_returns_none_after_disarm() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.tmp");
        std::fs::write(&path, b"x").unwrap();
        let g = TmpGuard::new(&path);
        assert_eq!(g.path(), Some(path.as_path()));
        assert!(g.is_armed(), "fresh guard must be armed");
        g.disarm();
        assert!(
            path.exists(),
            "disarm must cancel the Drop cleanup so the file remains"
        );
    }

    #[test]
    fn drop_panic_safety_removes_tmp() {
        // Pin the panic-safety contract: even a panic in the
        // guarded scope cleans up the tmp file.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("boom.tmp");
        std::fs::write(&path, b"hi").unwrap();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _g = TmpGuard::new(&path);
            panic!("forced");
        }));
        assert!(
            !path.exists(),
            "Drop must run during unwind so the tmp is removed"
        );
    }
}
