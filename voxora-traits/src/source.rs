//! The [`ModelSource`] trait and the value types that describe where a
//! model lives on disk and how it was acquired.

use std::path::PathBuf;

use async_trait::async_trait;

use crate::engine::ModelCapabilities;
use crate::error::AsrError;

/// A descriptor for a model that can be enumerated without downloading it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ModelDescriptor {
    /// Identifier as the source would accept it back
    /// (e.g. `Qwen/Qwen3-ASR-0.6B`).
    pub id: String,

    /// Human-readable name, if the source provides one.
    pub display_name: Option<String>,

    /// Reported capabilities, if the source can determine them without
    /// downloading the weights.
    pub capabilities: Option<ModelCapabilities>,
}

impl ModelDescriptor {
    /// Build a descriptor with just an id (no display name, no
    /// capabilities).
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            display_name: None,
            capabilities: None,
        }
    }

    /// Build a descriptor with id, display name, and capabilities.
    pub fn with_details(
        id: impl Into<String>,
        display_name: Option<String>,
        capabilities: Option<ModelCapabilities>,
    ) -> Self {
        Self {
            id: id.into(),
            display_name,
            capabilities,
        }
    }
}

/// Where a resolved model lives on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ModelDir {
    /// Root directory of the model on disk.
    pub path: PathBuf,

    /// Specific file inside `path` (e.g. `ggml-large-v3.bin` for
    /// single-file HF requests). `None` for whole-repo directories
    /// where the engine picks the right file from the directory
    /// listing. Populated by `voxora-hf` for 3-segment model ids
    /// (`org/repo/file`) starting in 0.1.2.
    pub entry: Option<PathBuf>,

    /// Which source provided this model.
    pub kind: ModelSourceKind,

    /// Concrete quantization this model was serialized in.
    pub quantization: Quantization,
}

impl ModelDir {
    /// Build a `ModelDir` from its four fields, with `entry` left as
    /// `None`. Whole-repo resolvers should keep using this constructor
    /// so the engine falls back to `locate_model_file`-style
    /// directory scanning.
    pub fn new(path: PathBuf, kind: ModelSourceKind, quantization: Quantization) -> Self {
        Self {
            path,
            entry: None,
            kind,
            quantization,
        }
    }

    /// Build a `ModelDir` with an explicit `entry` naming the specific
    /// file inside `path`. Used by `voxora-hf` for 3-segment model ids
    /// (`org/repo/file`) so the engine does not have to lex-sort a
    /// multi-file directory and accidentally pick the wrong file.
    pub fn with_entry(
        path: PathBuf,
        entry: PathBuf,
        kind: ModelSourceKind,
        quantization: Quantization,
    ) -> Self {
        Self {
            path,
            entry: Some(entry),
            kind,
            quantization,
        }
    }
}

/// Class of model provider.
///
/// `#[non_exhaustive]` so new sources can be added without breaking
/// downstream `match` arms.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ModelSourceKind {
    /// A directory already on disk; no download required.
    Local,
    /// Hugging Face Hub.
    HuggingFace,
}

impl ModelSourceKind {
    /// Stable string tag (`"local"`, `"huggingface"`, …) for logging.
    pub fn tag(&self) -> &'static str {
        match self {
            ModelSourceKind::Local => "local",
            ModelSourceKind::HuggingFace => "huggingface",
        }
    }
}

/// Concrete quantization variants a model was serialized in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Quantization {
    /// 32-bit IEEE float.
    F32,
    /// 16-bit brain float.
    Bf16,
    /// 16-bit IEEE float.
    F16,
    /// GGUF `Q4_K` (whisper.cpp).
    Q4K,
    /// GGUF `Q8_0` (whisper.cpp).
    Q8_0,
}

/// Caller's preferred quantization when one is not otherwise specified.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum QuantizationPreference {
    /// Let the source pick a sensible default.
    #[default]
    Auto,
    /// Prefer 32-bit float if available.
    F32,
    /// Prefer 16-bit brain float if available.
    Bf16,
    /// Prefer 16-bit IEEE float if available.
    F16,
    /// Prefer GGUF `Q4_K` if available.
    Q4K,
    /// Prefer GGUF `Q8_0` if available.
    Q8_0,
}

/// Options controlling how [`ModelSource::resolve`] acquires a model.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ResolveOptions {
    /// Preferred quantization; the source may pick a different one if
    /// the preferred is not available for this model.
    pub quantization: QuantizationPreference,

    /// Auth token, if the source requires one. For Hugging Face this
    /// overrides the `HF_TOKEN` environment variable.
    pub token: Option<String>,

    /// Specific git revision (branch, tag, or SHA) to pin the model to.
    pub revision: Option<String>,

    /// Caller-imposed maximum byte size for any single resolved file.
    ///
    /// `None` (the default) imposes no cap and preserves the
    /// pre-hardening behaviour. When set, sources honour this on
    /// their final resolved artifact: `LocalSource` rejects a
    /// resolved regular file whose `metadata().len()` exceeds the
    /// cap; `HuggingFaceSource` enforces the same ceiling on the
    /// downloaded bytes for a single-file resolve.
    ///
    /// Set this when the caller knows the upper bound of an expected
    /// model size (e.g. a Whisper ggml file is at most a few GiB) so
    /// an attacker who can plant a larger file under the local root
    /// or replace a HF cached file cannot force an unbounded read.
    /// Closes #144, EPIC #148.
    pub max_bytes: Option<u64>,

    /// Caller-imposed maximum byte length of the `model_id` string.
    ///
    /// `None` (the default) means "no cap", and each source applies
    /// its own intrinsic cap (`LocalSource` re-checks at 4 KiB;
    /// `ModelId::parse` enforces 4 KiB at the upstream gate).
    /// Setting this overrides the source's intrinsic cap with a
    /// tighter caller-specific ceiling; sources that do not honour
    /// `model_id` length (e.g. the HF arm, which splits on `/`) will
    /// ignore the field. Local sources honour it. Closes #143,
    /// EPIC #148.
    pub max_id_length: Option<usize>,
}

impl ResolveOptions {
    /// Construct a [`ResolveOptions`] with the given revision;
    /// everything else is `Auto` / `None`.
    pub fn with_revision(revision: impl Into<String>) -> Self {
        Self {
            revision: Some(revision.into()),
            ..Self::default()
        }
    }

    /// Construct a [`ResolveOptions`] with the given token;
    /// everything else is `Auto` / `None`.
    pub fn with_token(token: impl Into<String>) -> Self {
        Self {
            token: Some(token.into()),
            ..Self::default()
        }
    }

    /// Construct a [`ResolveOptions`] with the given `max_bytes`
    /// cap; everything else is `Auto` / `None`. Closes #144, EPIC
    /// #148 — callers needing a tighter cap than the source's
    /// intrinsic "no limit" default set this so the source
    /// rejects oversized resolved files (closes #144).
    pub fn with_max_bytes(max_bytes: u64) -> Self {
        Self {
            max_bytes: Some(max_bytes),
            ..Self::default()
        }
    }

    /// Construct a [`ResolveOptions`] with the given `max_id_length`
    /// cap; everything else is `Auto` / `None`. Closes #143, EPIC
    /// #148 — callers needing a tighter id-length cap than the
    /// source's intrinsic 4 KiB default set this so the source
    /// rejects oversized ids at the I/O gate.
    pub fn with_max_id_length(max_id_length: usize) -> Self {
        Self {
            max_id_length: Some(max_id_length),
            ..Self::default()
        }
    }
}

/// A source of models (Hugging Face, a local directory, future registries).
///
/// Acquisition is inherently asynchronous (network downloads), so this
/// trait uses `async_trait` even though [`crate::AsrEngine`] is
/// sync. The trait requires `Send + Sync` so a `Box<dyn ModelSource>`
/// can move across thread boundaries inside an HTTP server or CLI.
#[async_trait]
pub trait ModelSource: Send + Sync {
    /// Short, stable identifier for this source
    /// (`"huggingface"`, `"local"`, …).
    fn name(&self) -> &'static str;

    /// Resolve a model id (e.g. `Qwen/Qwen3-ASR-0.6B`) to a concrete
    /// [`ModelDir`] on disk, downloading if necessary.
    async fn resolve(&self, model_id: &str, opts: &ResolveOptions) -> Result<ModelDir, AsrError>;

    /// Query a model's capabilities without downloading the weights.
    async fn capabilities_for(&self, model_id: &str) -> Result<ModelCapabilities, AsrError>;

    /// List models known to this source. Defaults to
    /// [`AsrError::Unsupported`] because not every source can enumerate.
    async fn list_available(&self) -> Result<Vec<ModelDescriptor>, AsrError> {
        Err(AsrError::Unsupported("list_available"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In-memory `ModelSource` used to verify the trait is usable
    /// without network access. Exercises the default `list_available`.
    ///
    /// The async methods are intentionally not called from these tests
    /// so we do not need an executor in the test suite — that keeps
    /// `voxora-traits` free of `tokio` and `futures` per the Phase 1
    /// "build offline" rule. Behaviour of `resolve` /
    /// `capabilities_for` is covered in `voxora-hf` (Phase 2).
    struct FakeSource;

    #[async_trait]
    impl ModelSource for FakeSource {
        fn name(&self) -> &'static str {
            "fake"
        }

        async fn resolve(
            &self,
            model_id: &str,
            _opts: &ResolveOptions,
        ) -> Result<ModelDir, AsrError> {
            Ok(ModelDir {
                path: PathBuf::from(format!("/cache/{model_id}")),
                entry: None,
                kind: ModelSourceKind::Local,
                quantization: Quantization::F16,
            })
        }

        async fn capabilities_for(&self, _model_id: &str) -> Result<ModelCapabilities, AsrError> {
            Ok(ModelCapabilities::UNKNOWN)
        }
    }

    #[test]
    fn default_list_available_returns_unsupported_via_dispatch() {
        // We exercise the default-method dispatch without awaiting: a
        // `&dyn ModelSource` knows the vtable, and calling any async
        // method through it returns a future. We only assert that the
        // trait is constructible and `name()` works synchronously.
        let src: &dyn ModelSource = &FakeSource;
        assert_eq!(src.name(), "fake");
    }

    #[test]
    fn quantization_preference_default_is_auto() {
        assert_eq!(
            QuantizationPreference::default(),
            QuantizationPreference::Auto
        );
    }

    #[test]
    fn resolve_options_default_is_auto_no_token_no_revision() {
        let opts = ResolveOptions::default();
        assert_eq!(opts.quantization, QuantizationPreference::Auto);
        assert!(opts.token.is_none());
        assert!(opts.revision.is_none());
        // Closes #143, #144, EPIC #148 — the new caps default to
        // `None` so existing call sites using `ResolveOptions::default()`
        // continue to behave exactly as before.
        assert!(opts.max_bytes.is_none());
        assert!(opts.max_id_length.is_none());
    }

    #[test]
    fn resolve_options_implements_eq() {
        let a = ResolveOptions {
            quantization: QuantizationPreference::F16,
            token: Some("tok".into()),
            revision: Some("main".into()),
            max_bytes: None,
            max_id_length: None,
        };
        let b = ResolveOptions {
            quantization: QuantizationPreference::F16,
            token: Some("tok".into()),
            revision: Some("main".into()),
            max_bytes: None,
            max_id_length: None,
        };
        let c = ResolveOptions {
            quantization: QuantizationPreference::Q4K,
            token: Some("tok".into()),
            revision: Some("main".into()),
            max_bytes: None,
            max_id_length: None,
        };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn quantization_is_copy_and_eq() {
        let a = Quantization::F16;
        let b = a;
        assert_eq!(a, b);
        assert_ne!(a, Quantization::Q4K);
    }

    #[test]
    fn model_source_kind_implements_eq() {
        assert_eq!(ModelSourceKind::Local, ModelSourceKind::Local);
        assert_ne!(ModelSourceKind::Local, ModelSourceKind::HuggingFace);
    }

    #[test]
    fn model_dir_new_has_no_entry() {
        let dir = ModelDir::new(
            PathBuf::from("/cache/foo"),
            ModelSourceKind::Local,
            Quantization::F16,
        );
        assert!(
            dir.entry.is_none(),
            "ModelDir::new must default entry to None"
        );
    }

    #[test]
    fn model_dir_with_entry_records_specific_file() {
        let entry = PathBuf::from("/cache/foo/ggml-large-v3.bin");
        let dir = ModelDir::with_entry(
            PathBuf::from("/cache/foo"),
            entry.clone(),
            ModelSourceKind::HuggingFace,
            Quantization::F16,
        );
        assert_eq!(dir.entry.as_ref(), Some(&entry));
    }
}
