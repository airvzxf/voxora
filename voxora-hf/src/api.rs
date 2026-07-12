//! Thin REST bindings for the Hugging Face Hub.
//!
//! Three operations:
//!
//! - [`Api::model_metadata`] → metadata blob including `siblings[]`.
//! - [`Api::fetch_file_text`] → small JSON/text file (`config.json`,
//!   tokenizer config, …).
//! - [`Api::fetch_file_streamed`] → binary file (`model.safetensors`,
//!   shards, …), streamed to disk via [`crate::client::HfClient::get_to_file`].
//!
//! All paths are relative to the configured HF base URL. The client
//! prepends the base; this module never builds an absolute URL
//! directly.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::client::HfClient;
use crate::error::HfError;

/// Response of `GET /api/models/{model_id}/revision/{revision}`.
///
/// We only deserialize the fields we actually consume. The full
/// schema is much larger; see
/// <https://huggingface.co/docs/hub/api>.
#[derive(Debug, Deserialize)]
pub(crate) struct ModelMetadata {
    /// `rfilename` entries for every file in the repo at this revision.
    #[serde(default)]
    pub siblings: Vec<Sibling>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Sibling {
    pub rfilename: String,
}

#[derive(Clone)]
pub(crate) struct Api {
    client: HfClient,
}

impl Api {
    pub(crate) fn new(client: HfClient) -> Self {
        Self { client }
    }

    #[cfg(test)]
    pub(crate) fn with_client(client: HfClient) -> Self {
        Self { client }
    }

    /// Fetch the metadata document for `model_id` at `revision`.
    pub(crate) async fn model_metadata(
        &self,
        model_id: &str,
        revision: &str,
    ) -> Result<ModelMetadata, HfError> {
        validate_model_id(model_id)?;
        validate_revision(revision)?;
        let path = format!("/api/models/{model_id}/revision/{revision}");
        self.client.get_json(&path).await
    }

    /// Fetch a small text/JSON file from the repo into memory.
    pub(crate) async fn fetch_file_text(
        &self,
        model_id: &str,
        revision: &str,
        filename: &str,
    ) -> Result<String, HfError> {
        validate_model_id(model_id)?;
        validate_revision(revision)?;
        if filename.is_empty() || filename.contains("..") {
            return Err(HfError::InvalidInput(format!("bad filename: {filename:?}")));
        }
        let path = format!("/{model_id}/resolve/{revision}/{filename}");
        self.client.get_text(&path).await
    }

    /// Stream a binary file into `dest`. Atomic write happens inside
    /// the client.
    ///
    /// Kept as the borrowed counterpart of
    /// [`Self::fetch_file_streamed_owned`]; the source layer uses the
    /// owned variant so the returned future can be boxed.
    #[allow(dead_code)]
    pub(crate) async fn fetch_file_streamed(
        &self,
        model_id: &str,
        revision: &str,
        filename: &str,
        dest: &Path,
    ) -> Result<u64, HfError> {
        validate_model_id(model_id)?;
        validate_revision(revision)?;
        if filename.is_empty() || filename.contains("..") {
            return Err(HfError::InvalidInput(format!("bad filename: {filename:?}")));
        }
        let path = format!("/{model_id}/resolve/{revision}/{filename}");
        self.client.get_to_file(&path, dest).await
    }

    /// Same as [`Self::fetch_file_streamed`] but takes owned `String`s
    /// and `self` by value so the returned future is `'static` and
    /// can be moved into a `Box<dyn Future>`.
    pub(crate) async fn fetch_file_streamed_owned(
        self,
        model_id: String,
        revision: String,
        filename: String,
        dest: PathBuf,
    ) -> Result<u64, HfError> {
        validate_model_id(&model_id)?;
        validate_revision(&revision)?;
        if filename.is_empty() || filename.contains("..") {
            return Err(HfError::InvalidInput(format!("bad filename: {filename:?}")));
        }
        let path = format!("/{model_id}/resolve/{revision}/{filename}");
        self.client.get_to_file(&path, &dest).await
    }
}

fn validate_model_id(model_id: &str) -> Result<(), HfError> {
    if model_id.is_empty() {
        return Err(HfError::InvalidInput("empty model_id".into()));
    }
    if !model_id.contains('/') {
        return Err(HfError::InvalidInput(format!(
            "model_id {model_id:?} must be in 'org/name' form"
        )));
    }
    if model_id.starts_with('/') || model_id.ends_with('/') {
        return Err(HfError::InvalidInput(format!(
            "model_id {model_id:?} has leading or trailing slash"
        )));
    }
    if model_id.contains("..") {
        return Err(HfError::InvalidInput(format!(
            "model_id {model_id:?} contains '..'"
        )));
    }
    Ok(())
}

fn validate_revision(revision: &str) -> Result<(), HfError> {
    if revision.is_empty() {
        return Err(HfError::InvalidInput("empty revision".into()));
    }
    if revision.starts_with('/') || revision.contains("..") {
        return Err(HfError::InvalidInput(format!(
            "revision {revision:?} has invalid characters"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_model_id_accepts_org_name() {
        assert!(validate_model_id("Qwen/Qwen3-ASR-0.6B").is_ok());
        assert!(validate_model_id("openai/whisper-tiny").is_ok());
    }

    #[test]
    fn validate_model_id_rejects_garbage() {
        assert!(validate_model_id("").is_err());
        assert!(validate_model_id("nope").is_err());
        assert!(validate_model_id("/leading").is_err());
        assert!(validate_model_id("trailing/").is_err());
        assert!(validate_model_id("has/../escape").is_err());
    }

    #[test]
    fn validate_revision_rejects_garbage() {
        assert!(validate_revision("main").is_ok());
        assert!(validate_revision("5eb144179a02acc5e5ba31e748d22b0cf3e303b0").is_ok());
        assert!(validate_revision("").is_err());
        assert!(validate_revision("/etc/passwd").is_err());
        assert!(validate_revision("a..b").is_err());
    }

    #[test]
    fn fetch_file_text_rejects_path_traversal() {
        let api = Api::with_client(HfClient::builder().build().unwrap());
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let err = rt.block_on(api.fetch_file_text("Qwen/Qwen3-ASR-0.6B", "main", "../etc/passwd"));
        assert!(matches!(err, Err(HfError::InvalidInput(_))));
    }
}
