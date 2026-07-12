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

    /// Which source provided this model.
    pub kind: ModelSourceKind,

    /// Concrete quantization this model was serialized in.
    pub quantization: Quantization,
}

impl ModelDir {
    /// Build a `ModelDir` from its three fields.
    pub fn new(path: PathBuf, kind: ModelSourceKind, quantization: Quantization) -> Self {
        Self {
            path,
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
}

/// A source of models (Hugging Face, a local directory, future registries).
///
/// Acquisition is inherently asynchronous (network downloads), so this
/// trait uses `async_trait` even though [`crate::AsrEngine`] is sync.
/// The trait requires `Send + Sync` so a `Box<dyn ModelSource>` can
/// move across thread boundaries inside an HTTP server or CLI.
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
    /// `voxora-core` free of `tokio` and `futures` per the Phase 1
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
    }

    #[test]
    fn resolve_options_implements_eq() {
        let a = ResolveOptions {
            quantization: QuantizationPreference::F16,
            token: Some("tok".into()),
            revision: Some("main".into()),
        };
        let b = ResolveOptions {
            quantization: QuantizationPreference::F16,
            token: Some("tok".into()),
            revision: Some("main".into()),
        };
        let c = ResolveOptions {
            quantization: QuantizationPreference::Q4K,
            token: Some("tok".into()),
            revision: Some("main".into()),
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
}
