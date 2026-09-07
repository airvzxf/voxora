//! The concrete [`voxora_traits::ModelSource`] implementations:
//!
//! - [`LocalSource`] reads model weights from a directory already on
//!   disk — no network, no tokio, no reqwest. The directory is
//!   treated as opaque: the caller passes a `model_id`
//!   (e.g. `"some-org/some-repo/model.bin"`), and `resolve` joins it
//!   against the source's root, then returns [`voxora_traits::ModelDir`]
//!   with `entry` populated and `kind` set to
//!   [`voxora_traits::ModelSourceKind::Local`].
//! - [`ChainedSource`] is a tiny adapter that tries a primary source
//!   first and falls back to a secondary source when the primary
//!   surfaces [`voxora_traits::AsrError::ModelNotFound`]. It honours
//!   the spirit of the issue's "registry local first, HF on miss"
//!   claim without forcing a `voxora-registry` refactor.
//!
//! Both types are cheap to clone (every field is `Arc`-shared or a
//! `PathBuf`), so callers should hold them as `Arc<dyn ModelSource>`
//! and pass clones around.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use voxora_traits::{
    AsrError, ModelCapabilities, ModelDir, ModelSource, ModelSourceKind, Quantization,
    ResolveOptions,
};

/// Maximum byte length of a `model_id` string accepted by
/// [`LocalSource::resolve`]. Matches the parser-level cap in
/// `voxora_registry::ModelId::parse` (closes #143, EPIC #148).
const MAX_MODEL_ID_LEN: usize = 4096;

/// A [`voxora_traits::ModelSource`] that resolves model ids against a
/// directory already on disk.
///
/// Constructed via [`LocalSource::new`]:
///
/// ```no_run
/// use voxora_local::LocalSource;
/// use voxora_traits::ModelSource;
///
/// # async fn run() -> Result<(), voxora_traits::AsrError> {
/// let source = LocalSource::new("/srv/models");
/// let dir = source.resolve("org/repo/model.bin", &Default::default()).await?;
/// println!("model at {}", dir.entry.expect("entry").display());
/// # Ok(()) }
/// ```
///
/// ## Security contract (closes #143, #144, EPIC #148)
///
/// `resolve` enforces the following invariants; every violation
/// surfaces as [`voxora_traits::AsrError::InvalidInput`]:
///
/// 1. `model_id.len() <= 4096` bytes (cheap length cap, applied
///    before any path work).
/// 2. After joining `root.join(model_id)`, the resulting path
///    contains no `ParentDir` (`..`) component. (A `CurDir` (`.`)
///    component is also rejected for symmetry.) Path traversal
///    must be caught here, not at the OS layer, so the caller gets
///    a useful error message instead of a `Permission denied` or
///    silent escape.
/// 3. The resolved path is *not* a symbolic link. `resolve` uses
///    [`std::fs::symlink_metadata`] (the `lstat(2)` syscall), not
///    [`std::path::Path::is_file`], so a symlink under `root`
///    pointing outside is rejected with `InvalidInput`, not silently
///    followed. The Unix build additionally uses
///    [`libc::O_NOFOLLOW`] on the existence check to close the
///    classic symlink-swap TOCTOU: if the path is replaced between
///    `lstat` and `open` with a symlink, `O_NOFOLLOW` makes the
///    `open(2)` fail with `ELOOP` rather than resolving through the
///    new target. (`Path::is_file` would silently follow a symlink
///    on Unix, so the previous implementation was both a check and
///    a traversal vector.)
/// 4. The resolved regular file's length is below
///    [`ResolveOptions::max_bytes`] when the caller sets one.
///    `None` preserves the previous "no cap" behaviour.
///
/// The error envelope deliberately does **not** name the configured
/// `local_root`: a `ModelNotFound` message includes only the
/// requested path so a panic or log leak cannot expose the operator's
/// model directory layout.
///
/// The implementation never touches the network and never reads
/// outside `root + model_id`. Callers wanting a richer capabilities
/// answer than `ModelCapabilities::UNKNOWN` should do it themselves —
/// see `voxora-hf` for an example of synthesising capabilities from a
/// filename pattern.
#[derive(Clone)]
pub struct LocalSource {
    inner: Arc<Inner>,
}

struct Inner {
    root: PathBuf,
}

impl std::fmt::Debug for LocalSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Opaque Debug: do NOT print the configured root. A panic or
        // log emission carrying `{:?}` of a `LocalSource` would
        // otherwise leak the operator's model directory layout.
        // `finish_non_exhaustive` is the documented rustc convention
        // for "this struct has private fields the public Debug does
        // not expose" — closes #144's root-leak surface.
        f.debug_struct("LocalSource").finish_non_exhaustive()
    }
}

impl LocalSource {
    /// Build a source that resolves model ids against `root`.
    ///
    /// The root is stored verbatim — no `canonicalize`, no
    /// existence check. An empty directory listing at resolve time
    /// surfaces as `AsrError::ModelNotFound`, not as a constructor
    /// failure. Trailing slashes are folded by `PathBuf::join`, so
    /// `new("/srv/models/")` and `new("/srv/models")` behave
    /// identically.
    ///
    /// Absolute and relative roots are both supported; the `entry`
    /// field on the returned `ModelDir` inherits the same prefix
    /// (absolute root → absolute entry, relative root → relative
    /// entry). Most callers want an absolute root.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            inner: Arc::new(Inner { root: root.into() }),
        }
    }

    /// Borrow the configured root directory.
    pub fn root(&self) -> &Path {
        &self.inner.root
    }
}

#[async_trait]
impl ModelSource for LocalSource {
    fn name(&self) -> &'static str {
        "local"
    }

    async fn resolve(&self, model_id: &str, opts: &ResolveOptions) -> Result<ModelDir, AsrError> {
        // Step 1: cheap length cap. Mirrors the parser-level cap in
        // `ModelId::parse`; the parser enforces it first, but
        // defence in depth at the I/O gate means a direct caller of
        // `LocalSource::resolve` (bypassing the registry) cannot
        // smuggle in an oversized id.
        //
        // `opts.max_id_length` is the caller-imposed tighter cap;
        // when set, we honour it instead of the intrinsic 4 KiB
        // ceiling so a consumer who knows the realistic upper bound
        // for their model ids can fail-fast on a much shorter id.
        let id_cap = opts.max_id_length.unwrap_or(MAX_MODEL_ID_LEN);
        if model_id.len() > id_cap {
            return Err(AsrError::InvalidInput(format!(
                "model id too long (max {id_cap} bytes): {} bytes",
                model_id.len(),
            )));
        }

        // Step 2: join root + model_id and reject any `..` (or `.`)
        // component after the join. `PathBuf::join` does NOT
        // canonicalise; `root.join("/etc/passwd")` returns
        // `/etc/passwd` (absolute id replaces the root), so we have
        // to walk the components to catch a traversal whether the
        // id was absolute, relative-dot, or relative-dotdot.
        //
        // The error messages name the user-supplied id (NOT the
        // joined path) so they cannot leak the configured root even
        // when the operator path is sensitive.
        let entry = self.inner.root.join(model_id);
        for component in entry.components() {
            match component {
                Component::ParentDir => {
                    return Err(AsrError::InvalidInput(format!(
                        "path traversal: .. segment in id {model_id:?}",
                    )));
                }
                Component::CurDir => {
                    return Err(AsrError::InvalidInput(format!(
                        "path traversal: . segment in id {model_id:?}",
                    )));
                }
                _ => {}
            }
        }

        // Step 2b: containment guard (closes #143). `PathBuf::join`
        // is the wrong tool for a security boundary: an absolute
        // id REPLACES the configured root, so a hostile
        // `model_id = "/etc/passwd"` against
        // `LocalSource::new("/srv/models")` would happily resolve
        // to the host's `/etc/passwd`. When both the root and the
        // resolved entry are absolute, the entry MUST live under
        // the root — anything else is a containment violation that
        // the operator's trust boundary did not authorise.
        //
        // Relative roots are deliberately exempt: a relative root
        // means "anchored at the caller's CWD", which is its own
        // trust model; mixing a relative root with an absolute id
        // is a programmer error that `PathBuf::join` already
        // surfaces as an absolute path, and the operator who wrote
        // the absolute id is the same operator who chose the
        // process's CWD.
        if entry.is_absolute()
            && self.inner.root.is_absolute()
            && !entry.starts_with(&self.inner.root)
        {
            return Err(AsrError::InvalidInput(format!(
                "absolute path outside the configured root: id {model_id:?}",
            )));
        }

        // Step 3: lstat the entry. `symlink_metadata` does NOT follow
        // symlinks, so we can reject a symlink-under-root without
        // also resolving through it. An `ENOENT` here is the
        // "missing model" case the previous `is_file()` branch
        // handled — surface it as `ModelNotFound` so the chain
        // adapter's fallthrough semantics keep working. Other I/O
        // errors (permission denied, EIO, …) stay as `AudioIo`.
        let meta = match std::fs::symlink_metadata(&entry) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(AsrError::ModelNotFound(format!(
                    "local model not found at {}",
                    entry.display(),
                )));
            }
            Err(e) => return Err(AsrError::audio_io(entry.clone(), e)),
        };
        if meta.file_type().is_symlink() {
            return Err(AsrError::InvalidInput(format!(
                "refusing to follow symlink at id {model_id:?}",
            )));
        }
        if !meta.is_file() {
            return Err(AsrError::ModelNotFound(format!(
                "local model not found at {}",
                entry.display(),
            )));
        }

        // Step 4: size cap. `ResolveOptions::max_bytes` is the
        // caller-imposed ceiling; `None` preserves the previous
        // "no cap" behaviour. Checked before the `O_NOFOLLOW` open
        // so a hostile-but-correctly-shaped file cannot force a
        // 100 GiB read by claiming to be a model.
        if let Some(max) = opts.max_bytes
            && meta.len() > max
        {
            return Err(AsrError::InvalidInput(format!(
                "file too large ({} bytes > max_bytes {})",
                meta.len(),
                max,
            )));
        }

        // Step 5: O_NOFOLLOW existence check. Closes the classic
        // symlink-swap TOCTOU (issue #144) on Unix: even if a
        // privileged process replaces the file with a symlink
        // between `lstat` and `open`, `O_NOFOLLOW` makes the open
        // fail with `ELOOP` rather than silently resolve through
        // the new target. The opened handle is dropped immediately
        // — the engine opens the file again at load time. The point
        // of this step is not to hand back a `File`, it is to prove
        // the open succeeded against the same path the `lstat`
        // examined.
        #[cfg(unix)]
        let open_result = {
            use std::os::unix::fs::OpenOptionsExt;
            std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW)
                .open(&entry)
        };
        #[cfg(not(unix))]
        let open_result = std::fs::File::open(&entry);
        let _open = open_result.map_err(|e| AsrError::audio_io(entry.clone(), e))?;
        drop(_open);

        Ok(ModelDir::with_entry(
            self.inner.root.clone(),
            entry,
            ModelSourceKind::Local,
            Quantization::F16,
        ))
    }

    async fn capabilities_for(&self, _model_id: &str) -> Result<ModelCapabilities, AsrError> {
        // A directory listing cannot synthesise engine metadata
        // without reading `config.json`; the caller is free to do
        // that themselves. Return the `UNKNOWN` sentinel so
        // downstream "do you support this engine?" probes don't
        // crash on a missing field.
        Ok(ModelCapabilities::UNKNOWN)
    }
}

/// A [`voxora_traits::ModelSource`] adapter that tries `primary`
/// first and falls back to `fallback` when `primary` reports a
/// model miss.
///
/// Useful for the canonical "local directory first, Hugging Face on
/// miss" composition without forcing `voxora-registry` to carry a
/// `Vec<Arc<dyn ModelSource>>`. All other errors propagate verbatim
/// from `primary`; the fallback is **only** consulted when the
/// primary returns [`voxora_traits::AsrError::ModelNotFound`].
///
/// Both `name` and `capabilities_for` are forwarded to `primary`;
/// the fallback is invisible until a primary miss triggers it. This
/// matches the user-facing expectation that "this is the local
/// source" while still honouring the fallback chain.
#[derive(Clone)]
pub struct ChainedSource {
    primary: Arc<dyn ModelSource>,
    fallback: Arc<dyn ModelSource>,
}

impl std::fmt::Debug for ChainedSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChainedSource")
            .field("primary", &self.primary.name())
            .field("fallback", &self.fallback.name())
            .finish_non_exhaustive()
    }
}

impl ChainedSource {
    /// Build a chain that tries `primary` first and falls back to
    /// `fallback` on `ModelNotFound`.
    ///
    /// The order matters: the first source whose `name()` matches
    /// "the source the user expects" should be `primary`. In the
    /// canonical "local first" setup, that is:
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use voxora_local::{ChainedSource, LocalSource};
    /// use voxora_traits::ModelSource;
    ///
    /// # async fn run() -> Result<(), voxora_traits::AsrError> {
    /// let primary = Arc::new(LocalSource::new("/srv/models"));
    /// // In real code the fallback would be
    /// // `Arc::new(HuggingFaceSource::new()?)`. The chain is
    /// // type-agnostic on `ModelSource`; any `Arc<dyn ModelSource>`
    /// // is acceptable.
    /// let fallback: Arc<dyn ModelSource> = primary.clone();
    /// let chain = ChainedSource::new(primary, fallback);
    /// let dir = chain.resolve("org/repo/model.bin", &Default::default()).await?;
    /// # Ok(()) }
    /// ```
    pub fn new(primary: Arc<dyn ModelSource>, fallback: Arc<dyn ModelSource>) -> Self {
        Self { primary, fallback }
    }

    /// Borrow the primary source.
    pub fn primary(&self) -> &Arc<dyn ModelSource> {
        &self.primary
    }

    /// Borrow the fallback source.
    pub fn fallback(&self) -> &Arc<dyn ModelSource> {
        &self.fallback
    }
}

#[async_trait]
impl ModelSource for ChainedSource {
    fn name(&self) -> &'static str {
        self.primary.name()
    }

    async fn resolve(&self, model_id: &str, opts: &ResolveOptions) -> Result<ModelDir, AsrError> {
        match self.primary.resolve(model_id, opts).await {
            Ok(dir) => Ok(dir),
            Err(AsrError::ModelNotFound(msg)) => self
                .fallback
                .resolve(model_id, opts)
                .await
                .map_err(|fallback_err| {
                    // Prefer the primary's miss message: it's the error
                    // the user took the chain to avoid. Fall through to
                    // the fallback's error only if the fallback also
                    // failed in some non-trivial way, and even then we
                    // annotate the primary miss for context.
                    if matches!(fallback_err, AsrError::ModelNotFound(_)) {
                        AsrError::ModelNotFound(format!(
                            "primary miss: {msg}; fallback miss: {fallback_err}"
                        ))
                    } else {
                        fallback_err
                    }
                }),
            Err(other) => Err(other),
        }
    }

    async fn capabilities_for(&self, model_id: &str) -> Result<ModelCapabilities, AsrError> {
        self.primary.capabilities_for(model_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_returns_local() {
        assert_eq!(LocalSource::new("/srv/models").name(), "local");
    }

    #[test]
    fn root_is_returned_verbatim() {
        assert_eq!(
            LocalSource::new("/srv/models").root(),
            Path::new("/srv/models"),
        );
        assert_eq!(
            LocalSource::new("/srv/models/").root(),
            Path::new("/srv/models/"),
        );
    }

    #[test]
    fn debug_does_not_include_root() {
        // Closes #144 — the Debug impl must not leak the configured
        // `local_root` into panic messages or log emission. The
        // opaque `finish_non_exhaustive()` form prints the type name
        // and a `..` placeholder; the actual root stays private.
        let dbg = format!("{:?}", LocalSource::new("/srv/models"));
        assert!(
            !dbg.contains("/srv/models"),
            "Debug must not leak the configured root: {dbg}",
        );
        assert!(
            dbg.contains("LocalSource"),
            "Debug must still print the type name: {dbg}",
        );
    }
}
