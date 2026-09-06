//! Local copy of `voxora_testkit::InMemorySource` for the
//! `benches/resolve.rs` harness.
//!
//! Cargo's `cargo package` step resolves every dep in the
//! crates.io index, including `[dev-dependencies]`. `voxora-testkit`
//! is `publish = false`, so this crate cannot depend on it from a
//! publishable manifest (closes #140). The bench only needs the
//! `ModelSource` mock, so the file is inlined here. Mirrors
//! `voxora-testkit/src/fixtures/mod.rs:22-100` byte-for-byte; if
//! upstream changes, sync both.
//!
//! Bench-local; not a public surface.

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Mutex;

use async_trait::async_trait;
use voxora_traits::{
    AsrError, ModelCapabilities, ModelDir, ModelSource, ModelSourceKind, Quantization,
    ResolveOptions,
};

/// In-memory [`ModelSource`] used by the bench harness.
///
/// `resolve` returns a `ModelDir` whose `path` and `entry` are
/// synthesised from the requested `model_id`. The optional
/// `missing_files` list lets a scenario simulate "the requested
/// file is not in the cache" (the voxora#79 scenario).
pub struct InMemorySource {
    pub missing_files: Mutex<Vec<String>>,
}

impl InMemorySource {
    pub fn new() -> Self {
        Self {
            missing_files: Mutex::new(Vec::new()),
        }
    }

    pub fn with_missing(mut self, file: impl Into<String>) -> Self {
        self.missing_files.get_mut().unwrap().push(file.into());
        self
    }
}

impl Default for InMemorySource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModelSource for InMemorySource {
    fn name(&self) -> &'static str {
        "in-memory"
    }

    async fn resolve(&self, model_id: &str, _opts: &ResolveOptions) -> Result<ModelDir, AsrError> {
        let parts: Vec<&str> = model_id.split('/').collect();
        let leaf = parts.last().copied().unwrap_or("");
        let dir_path = if parts.len() >= 2 {
            let without_leaf = parts[..parts.len() - 1].join("/");
            format!("/fake-cache/{without_leaf}")
        } else {
            format!("/fake-cache/{model_id}")
        };
        let entry_path = if leaf.is_empty() || parts.len() < 2 {
            None
        } else {
            Some(format!("{dir_path}/{leaf}"))
        };
        let missing = self.missing_files.lock().unwrap().clone();
        let entry = entry_path
            .map(PathBuf::from)
            .filter(|p| !missing.iter().any(|m| p.to_string_lossy().ends_with(m)));
        if entry.is_none() && !missing.is_empty() && !leaf.is_empty() {
            return Err(AsrError::ModelNotFound(format!(
                "simulated missing file in cache for {model_id:?}"
            )));
        }
        let path = PathBuf::from(&dir_path);
        let kind = ModelSourceKind::Local;
        let quant = Quantization::F16;
        Ok(match entry {
            Some(e) => ModelDir::with_entry(path, e, kind, quant),
            None => ModelDir::new(path, kind, quant),
        })
    }

    async fn capabilities_for(&self, _model_id: &str) -> Result<ModelCapabilities, AsrError> {
        Ok(ModelCapabilities::UNKNOWN)
    }
}
