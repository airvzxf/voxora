//! The public surface: [`HuggingFaceSource`] implementing
//! [`voxora_core::ModelSource`].

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use voxora_core::{
    AsrError, ModelCapabilities, ModelDescriptor, ModelDir, ModelSource, ModelSourceKind,
    Quantization, ResolveOptions,
};

use crate::api::{Api, Sibling};
use crate::cache;
use crate::capabilities;
use crate::client::HfClientBuilder;
use crate::error::HfError;
use crate::quantization;
use crate::source::cache_resolver::CacheResolver;

/// Hugging Face implementation of [`voxora_core::ModelSource`].
///
/// Cheap to clone (every field is `Arc`-shared), so callers should
/// hold it as `Arc<HuggingFaceSource>` and pass clones around.
#[derive(Clone)]
pub struct HuggingFaceSource {
    inner: Arc<Inner>,
}

struct Inner {
    api: Api,
    cache_root: PathBuf,
    default_revision: String,
}

impl std::fmt::Debug for HuggingFaceSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HuggingFaceSource")
            .field("cache_root", &self.inner.cache_root)
            .field("default_revision", &self.inner.default_revision)
            .finish_non_exhaustive()
    }
}

impl HuggingFaceSource {
    /// Construct a source with sensible defaults.
    ///
    /// - Cache root: `$XDG_CACHE_HOME/voxora/models/huggingface`.
    /// - Base URL: `https://huggingface.co`.
    /// - Token: `HF_TOKEN` then `HUGGING_FACE_HUB_TOKEN` (first non-empty wins).
    /// - User-Agent: `voxora-hf/<version>`.
    /// - Timeout: 600 s per request.
    pub fn new() -> Result<Self, AsrError> {
        Self::builder().build()
    }

    /// Start configuring a source with non-default options.
    pub fn builder() -> HuggingFaceSourceBuilder {
        HuggingFaceSourceBuilder::default()
    }
}

#[async_trait]
impl ModelSource for HuggingFaceSource {
    fn name(&self) -> &'static str {
        "huggingface"
    }

    async fn resolve(&self, model_id: &str, opts: &ResolveOptions) -> Result<ModelDir, AsrError> {
        let revision = opts
            .revision
            .clone()
            .unwrap_or_else(|| self.inner.default_revision.clone());

        let dir = cache::model_dir(&self.inner.cache_root, model_id, &revision);

        // Fast path: already cached with the marker in place.
        if cache::is_complete(&dir) {
            let quantization = self
                .detect_quantization_from_cache(model_id, &dir, &revision)
                .await
                .unwrap_or(Quantization::F16);
            return Ok(ModelDir::new(
                dir,
                ModelSourceKind::HuggingFace,
                quantization,
            ));
        }

        // Slow path: ensure dir, fetch siblings, download required files.
        cache::ensure_dir(&dir).map_err(HfError::into_asr)?;
        cache::clear_marker(&dir).map_err(HfError::into_asr)?;

        let resolver = CacheResolver::new(
            self.inner.api.clone(),
            model_id.to_string(),
            revision.clone(),
            dir.clone(),
        );
        resolver
            .run(|siblings| self.pick_required_files(siblings))
            .await
            .map_err(HfError::into_asr)?;

        let quantization = self
            .detect_quantization_from_cache(model_id, &dir, &revision)
            .await
            .unwrap_or(Quantization::F16);

        Ok(ModelDir::new(
            dir,
            ModelSourceKind::HuggingFace,
            quantization,
        ))
    }

    async fn capabilities_for(&self, model_id: &str) -> Result<ModelCapabilities, AsrError> {
        // The metadata endpoint exposes a *summary* of `config.json`
        // that drops fields like `support_languages`, so we always go
        // for the standalone file. Existence is verified by the 404
        // we get if the model id is wrong.
        let bytes = self
            .inner
            .api
            .fetch_file_text(model_id, "main", "config.json")
            .await
            .map_err(HfError::into_asr)?;
        let raw = capabilities::parse_config(bytes.as_bytes())
            .map_err(|e| AsrError::InvalidInput(format!("config.json: {e}")))?;
        Ok(capabilities::from_config(&raw))
    }

    async fn list_available(&self) -> Result<Vec<ModelDescriptor>, AsrError> {
        Ok(crate::known_models::curated())
    }
}

impl HuggingFaceSource {
    /// Decide which siblings are required for a full resolve.
    ///
    /// Required files: `config.json`, `tokenizer.json` (or
    /// `vocab.json` + `merges.txt` + `tokenizer_config.json`),
    /// `preprocessor_config.json` (best effort), the safetensors
    /// weights (single or sharded).
    fn pick_required_files<'a>(&self, siblings: &'a [Sibling]) -> Vec<&'a Sibling> {
        let names: Vec<&str> = siblings.iter().map(|s| s.rfilename.as_str()).collect();

        let mut required: Vec<&str> = Vec::new();
        let push = |v: &mut Vec<&'a Sibling>, name: &str, all: &'a [Sibling]| {
            if let Some(s) = all.iter().find(|s| s.rfilename == name) {
                v.push(s);
            }
        };

        // Always-present.
        let mut local = Vec::new();
        push(&mut local, "config.json", siblings);
        push(&mut local, "preprocessor_config.json", siblings);
        push(&mut local, "tokenizer_config.json", siblings);

        // Tokenizer: prefer the unified file, else the trio.
        if names.contains(&"tokenizer.json") {
            push(&mut local, "tokenizer.json", siblings);
        } else if names.contains(&"vocab.json") && names.contains(&"merges.txt") {
            push(&mut local, "vocab.json", siblings);
            push(&mut local, "merges.txt", siblings);
        }

        // Weights: sharded vs single.
        if names.contains(&"model.safetensors.index.json") {
            push(&mut local, "model.safetensors.index.json", siblings);
            // Add every shard listed in the index (we'll resolve them
            // explicitly below).
            let _ = required;
        } else if names.contains(&"model.safetensors") {
            push(&mut local, "model.safetensors", siblings);
        } else if names
            .iter()
            .any(|n| n.starts_with("model-") && n.ends_with(".safetensors"))
        {
            for s in siblings {
                if s.rfilename.starts_with("model-") && s.rfilename.ends_with(".safetensors") {
                    local.push(s);
                }
            }
        }

        // Deduplicate while preserving order.
        for s in local {
            if !required.iter().any(|r: &&str| r == &s.rfilename) {
                required.push(s.rfilename.as_str());
            }
        }
        required
            .into_iter()
            .filter_map(|n| siblings.iter().find(|s| s.rfilename == n))
            .collect()
    }

    /// Inspect the cached `config.json` (and shards' filenames) to
    /// figure out the quantization we actually downloaded.
    async fn detect_quantization_from_cache(
        &self,
        _model_id: &str,
        dir: &std::path::Path,
        _revision: &str,
    ) -> Result<Quantization, HfError> {
        // Try `torch_dtype` first.
        let config_path = dir.join("config.json");
        if let Ok(bytes) = std::fs::read(&config_path) {
            if let Ok(raw) = capabilities::parse_config(&bytes) {
                if let Some(s) = raw.primary_model_type().or(raw.primary_arch()) {
                    if s.to_ascii_lowercase().contains("qwen3asr")
                        || s.to_ascii_lowercase().contains("qwen3_asr")
                    {
                        // Qwen3-ASR is BF16 in the official release.
                        return Ok(Quantization::Bf16);
                    }
                }
                if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    if let Some(t) = value.get("torch_dtype").and_then(|v| v.as_str()) {
                        return Ok(quantization::from_torch_dtype(t));
                    }
                }
            }
        }
        // Fall back to filename-based detection (GGUF).
        let entries = std::fs::read_dir(dir).map_err(|e| HfError::Io {
            path: dir.to_path_buf(),
            message: "read_dir".into(),
            source: e,
        })?;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.ends_with(".safetensors") || name.ends_with(".gguf") || name.ends_with(".bin") {
                return Ok(quantization::from_gguf_filename(&name));
            }
        }
        Ok(Quantization::F16)
    }
}

/// Builder for [`HuggingFaceSource`].
#[derive(Debug, Clone)]
pub struct HuggingFaceSourceBuilder {
    client: HfClientBuilder,
    cache_root: Option<PathBuf>,
    default_revision: String,
    /// `Some(_)`: user explicitly set a token. `None`: defer to env.
    explicit_token: Option<Option<String>>,
}

impl Default for HuggingFaceSourceBuilder {
    fn default() -> Self {
        Self {
            client: HfClientBuilder::default(),
            cache_root: None,
            default_revision: "main".to_string(),
            explicit_token: None,
        }
    }
}

impl HuggingFaceSourceBuilder {
    /// Override the cache root directory. Defaults to
    /// `$XDG_CACHE_HOME/voxora/models/huggingface`.
    pub fn cache_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.cache_root = Some(path.into());
        self
    }

    /// Override the base URL. Defaults to `https://huggingface.co`.
    /// Mostly useful for tests pointing at a local mock server.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.client = self.client.base_url(url);
        self
    }

    /// Override the bearer token.
    ///
    /// - Pass `Some("hf_…")` to send that exact token.
    /// - Pass `Some("")` to force anonymous requests (ignore env).
    /// - Pass `None` (the default) to read `HF_TOKEN` then
    ///   `HUGGING_FACE_HUB_TOKEN` from the environment at build time.
    pub fn token(mut self, token: Option<String>) -> Self {
        self.explicit_token = Some(token);
        self
    }

    /// Override the User-Agent header.
    pub fn user_agent(mut self, ua: impl Into<String>) -> Self {
        self.client = self.client.user_agent(ua);
        self
    }

    /// Per-request timeout in seconds. Defaults to 600 s.
    pub fn timeout(mut self, secs: u64) -> Self {
        self.client = self.client.timeout(secs);
        self
    }

    /// Override the revision used when `ResolveOptions::revision`
    /// is `None`. Defaults to `"main"`.
    pub fn default_revision(mut self, rev: impl Into<String>) -> Self {
        self.default_revision = rev.into();
        self
    }

    /// Construct the source.
    pub fn build(self) -> Result<HuggingFaceSource, AsrError> {
        let resolved_token = match self.explicit_token {
            Some(t) => t,
            None => read_env_token(),
        };
        let client = self.client.token(resolved_token);
        let http = client.build().map_err(HfError::into_asr)?;
        let cache_root = self.cache_root.unwrap_or_else(cache::default_cache_root);
        Ok(HuggingFaceSource {
            inner: Arc::new(Inner {
                api: Api::new(http),
                cache_root,
                default_revision: self.default_revision,
            }),
        })
    }
}

/// Read `HF_TOKEN` then `HUGGING_FACE_HUB_TOKEN` from the
/// environment. Returns `None` if both are unset or empty.
fn read_env_token() -> Option<String> {
    for var in ["HF_TOKEN", "HUGGING_FACE_HUB_TOKEN"] {
        if let Ok(t) = std::env::var(var) {
            if !t.is_empty() {
                return Some(t);
            }
        }
    }
    None
}

/// Internal helpers shared with the [`CacheResolver`] submodule.
pub(crate) mod cache_resolver {
    use super::*;

    use futures_util::future::try_join_all;
    use std::future::Future;
    use std::path::PathBuf;

    /// Type alias for the boxed download future. Defined once so the
    /// two parallel `Vec`s below don't trip clippy::type_complexity.
    type DownloadFut = std::pin::Pin<Box<dyn Future<Output = Result<u64, HfError>> + Send>>;

    /// Drives the actual download: given a `plan_fn` that turns a
    /// sibling list into the files we need, downloads them
    /// concurrently, then writes the `.complete` marker.
    ///
    /// Owns an [`Api`] clone so the download futures are `'static`
    /// and can live in a `Vec` without lifetime gymnastics.
    pub(crate) struct CacheResolver {
        api: Api,
        model_id: String,
        revision: String,
        dir: PathBuf,
    }

    impl CacheResolver {
        pub(crate) fn new(api: Api, model_id: String, revision: String, dir: PathBuf) -> Self {
            Self {
                api,
                model_id,
                revision,
                dir,
            }
        }

        pub(crate) async fn run<F>(&self, plan_fn: F) -> Result<Quantization, HfError>
        where
            F: FnOnce(&[Sibling]) -> Vec<&Sibling>,
        {
            // Local owned aliases so the futures we build below are
            // not bound to `&self`'s lifetime.
            let model_id = self.model_id.clone();
            let revision = self.revision.clone();
            let dir = self.dir.clone();

            let metadata = self.api.model_metadata(&model_id, &revision).await?;
            let mut plan = plan_fn(&metadata.siblings);

            // If a sharded index is in play, expand it into the
            // actual shard filenames.
            let mut extra_files: Vec<String> = Vec::new();
            if plan
                .iter()
                .any(|s| s.rfilename == "model.safetensors.index.json")
            {
                let index_text = self
                    .api
                    .fetch_file_text(
                        &self.model_id,
                        &self.revision,
                        "model.safetensors.index.json",
                    )
                    .await?;
                let parsed: serde_json::Value =
                    serde_json::from_str(&index_text).map_err(|e| HfError::Json {
                        context: "model.safetensors.index.json".into(),
                        source: e,
                    })?;
                let map = parsed
                    .get("weight_map")
                    .and_then(|v| v.as_object())
                    .ok_or_else(|| HfError::Protocol {
                        url: format!(
                            "https://huggingface.co/{}/resolve/{}/model.safetensors.index.json",
                            self.model_id, self.revision
                        ),
                        message: "missing weight_map".into(),
                    })?;
                let mut shards: Vec<String> = map
                    .values()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect();
                shards.sort();
                shards.dedup();
                extra_files.extend(shards);
            }

            // Download the explicitly planned files first.
            let mut downloads: Vec<DownloadFut> = Vec::new();
            for sibling in plan.drain(..) {
                let dest = dir.join(&sibling.rfilename);
                if dest.is_file() {
                    continue;
                }
                let dest_owned = dest.clone();
                let fut: DownloadFut = Box::pin(self.api.clone().fetch_file_streamed_owned(
                    model_id.clone(),
                    revision.clone(),
                    sibling.rfilename.clone(),
                    dest_owned,
                ));
                downloads.push(fut);
            }

            // Download the expanded shards in parallel.
            let mut shard_futs: Vec<DownloadFut> = Vec::new();
            for shard in extra_files {
                let dest = dir.join(&shard);
                if dest.is_file() {
                    continue;
                }
                let dest_owned = dest.clone();
                let fut: DownloadFut = Box::pin(self.api.clone().fetch_file_streamed_owned(
                    model_id.clone(),
                    revision.clone(),
                    shard,
                    dest_owned,
                ));
                shard_futs.push(fut);
            }
            let shard_results = try_join_all(shard_futs).await?;
            let plan_results = try_join_all(downloads).await?;
            let total: u64 = shard_results.iter().chain(plan_results.iter()).sum();

            // SHA256 verification (best-effort).
            verify_sha256_sidecars(&dir).await?;

            cache::mark_complete(&dir)?;
            cache::cleanup_partials(&dir)?;
            let _ = total; // logged only when tracing is wired in.
            Ok(Quantization::F16) // concrete quantization is refined later.
        }
    }

    /// For every `<file>.sha256` sidecar published in the repo, check
    /// the local copy. Silently skips files without a sidecar.
    async fn verify_sha256_sidecars(dir: &std::path::Path) -> Result<(), HfError> {
        use sha2::{Digest, Sha256};
        let entries = std::fs::read_dir(dir).map_err(|e| HfError::Io {
            path: dir.to_path_buf(),
            message: "read_dir for sha256".into(),
            source: e,
        })?;
        for entry in entries.flatten() {
            let p = entry.path();
            let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !name.ends_with(".sha256") {
                continue;
            }
            let target_name = name.trim_end_matches(".sha256");
            let target_path = dir.join(target_name);
            if !target_path.is_file() {
                continue;
            }
            let expected = std::fs::read_to_string(&p)
                .map_err(|e| HfError::Io {
                    path: p.clone(),
                    message: "read sha256".into(),
                    source: e,
                })?
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            let bytes = std::fs::read(&target_path).map_err(|e| HfError::Io {
                path: target_path.clone(),
                message: "read target for sha256".into(),
                source: e,
            })?;
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let digest = hasher.finalize();
            let actual = digest
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            if expected != actual {
                return Err(HfError::Protocol {
                    url: target_path.display().to_string(),
                    message: format!(
                        "sha256 mismatch for {target_name}: expected {expected}, got {actual}"
                    ),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Sibling;

    fn src() -> HuggingFaceSource {
        // Don't construct a real client for the planning tests;
        // we only call the pick function which is `&self` only.
        let api = Api::with_client(crate::client::HfClient::builder().build().unwrap());
        HuggingFaceSource {
            inner: Arc::new(Inner {
                api,
                cache_root: std::env::temp_dir(),
                default_revision: "main".into(),
            }),
        }
    }

    fn s(name: &str) -> Sibling {
        Sibling {
            rfilename: name.into(),
        }
    }

    #[test]
    fn pick_required_single_file_model() {
        let siblings = vec![
            s("config.json"),
            s("preprocessor_config.json"),
            s("tokenizer_config.json"),
            s("tokenizer.json"),
            s("model.safetensors"),
            s("README.md"),
        ];
        let src = src();
        let plan: Vec<String> = src
            .pick_required_files(&siblings)
            .into_iter()
            .map(|s| s.rfilename.clone())
            .collect();
        let has = |needle: &str| plan.iter().any(|n| n == needle);
        assert!(has("config.json"));
        assert!(has("tokenizer.json"));
        assert!(has("model.safetensors"));
        assert!(has("preprocessor_config.json"));
        assert!(has("tokenizer_config.json"));
        assert!(!has("README.md"));
    }

    #[test]
    fn pick_required_sharded_model() {
        let siblings = vec![
            s("config.json"),
            s("model.safetensors.index.json"),
            s("model-00001-of-00002.safetensors"),
            s("model-00002-of-00002.safetensors"),
            s("tokenizer.json"),
        ];
        let plan: Vec<String> = src()
            .pick_required_files(&siblings)
            .into_iter()
            .map(|s| s.rfilename.clone())
            .collect();
        assert!(plan.iter().any(|n| n == "model.safetensors.index.json"));
        // Shards are handled by the resolver, not by pick; pick only
        // needs to include the index here.
        assert_eq!(
            plan.iter().filter(|n| n.ends_with(".safetensors")).count(),
            0
        );
    }

    #[test]
    fn pick_required_with_trio_tokenizer() {
        let siblings = vec![
            s("config.json"),
            s("vocab.json"),
            s("merges.txt"),
            s("tokenizer_config.json"),
            s("model.safetensors"),
        ];
        let plan: Vec<String> = src()
            .pick_required_files(&siblings)
            .into_iter()
            .map(|s| s.rfilename.clone())
            .collect();
        assert!(plan.iter().any(|n| n == "vocab.json"));
        assert!(plan.iter().any(|n| n == "merges.txt"));
        assert!(!plan.iter().any(|n| n == "tokenizer.json"));
    }
}
