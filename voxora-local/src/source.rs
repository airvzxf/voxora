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

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use voxora_traits::{
    AsrError, ModelCapabilities, ModelDir, ModelSource, ModelSourceKind, Quantization,
    ResolveOptions,
};

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
/// `resolve` joins `root_dir` with `model_id` (using `PathBuf::join`,
/// which already normalises trailing slashes), checks the resulting
/// path with [`std::path::Path::is_file`] (which follows symlinks on
/// Unix), and returns a [`voxora_traits::ModelDir`] with `entry` set
/// to the absolute-or-relative file path matching the input `root`'s
/// absoluteness. If the file is missing, the error is
/// [`voxora_traits::AsrError::ModelNotFound`] naming both the missing
/// path and the configured root.
///
/// The implementation never touches the network and never reads
/// outside `root_dir + model_id`. Callers wanting a richer
/// capabilities answer than `ModelCapabilities::UNKNOWN` should do it
/// themselves — see `voxora-hf` for an example of synthesising
/// capabilities from a filename pattern.
#[derive(Clone)]
pub struct LocalSource {
    inner: Arc<Inner>,
}

struct Inner {
    root: PathBuf,
}

impl std::fmt::Debug for LocalSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LocalSource")
            .field("root", &self.inner.root)
            .finish_non_exhaustive()
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

    async fn resolve(&self, model_id: &str, _opts: &ResolveOptions) -> Result<ModelDir, AsrError> {
        let entry = self.inner.root.join(model_id);
        if !entry.is_file() {
            return Err(AsrError::ModelNotFound(format!(
                "local model not found at {} (root: {})",
                entry.display(),
                self.inner.root.display(),
            )));
        }
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
    fn debug_includes_root() {
        let dbg = format!("{:?}", LocalSource::new("/srv/models"));
        assert!(dbg.contains("/srv/models"), "{dbg}");
    }
}
