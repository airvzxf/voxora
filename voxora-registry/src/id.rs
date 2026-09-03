//! Strict [`ModelId`] parsing.
//!
//! Three accepted shapes:
//! - `"org/repo"`           → HF whole-repo
//! - `"org/repo/file"`      → HF single-file (the fix path for #79)
//! - `"./local/path"` or absolute path → Local source
//!
//! Anything else is a parse error with a descriptive message.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::RegistryError;

/// Where the model lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SourceKind {
    /// Hugging Face Hub.
    HuggingFace,
    /// A directory already on disk.
    Local,
}

impl fmt::Display for SourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::HuggingFace => "huggingface",
            Self::Local => "local",
        })
    }
}

/// A fully-parsed model identifier.
///
/// Holds enough information for a [`crate::Registry`] to look up an
/// [`crate::descriptor::EngineDescriptor`] and resolve a
/// [`voxora_traits::ModelDir`] on disk. Parsing is strict — no
/// lex-sort fallbacks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ModelId {
    /// Which source the id belongs to.
    pub source: SourceKind,
    /// For HF: `org/repo`. For Local: absolute path. Never contains
    /// a file component.
    pub repo: String,
    /// For HF single-file: the exact filename inside `repo`. `None`
    /// for whole-repo HF requests or for Local sources.
    pub path: Option<Vec<String>>,
}

impl ModelId {
    /// Parse a user-supplied string into a `ModelId`. See module
    /// docs for accepted shapes.
    pub fn parse(s: &str) -> Result<Self, RegistryError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(RegistryError::Parse("empty id".into()));
        }

        // Local source: starts with `./`, `../`, or `/`.
        if trimmed.starts_with('/') || trimmed.starts_with("./") || trimmed.starts_with("../") {
            return Ok(Self {
                source: SourceKind::Local,
                repo: trimmed.to_string(),
                path: None,
            });
        }

        // HF: split into 2 or 3 segments by `/`.
        let parts: Vec<&str> = trimmed.split('/').collect();
        match parts.len() {
            2 => {
                let org = parts[0];
                let repo = parts[1];
                if org.is_empty() || repo.is_empty() {
                    return Err(RegistryError::Parse(format!(
                        "HF id must have non-empty org and repo: {trimmed:?}"
                    )));
                }
                if org.contains(' ') || repo.contains(' ') {
                    return Err(RegistryError::Parse(format!(
                        "HF id segments must not contain spaces: {trimmed:?}"
                    )));
                }
                Ok(Self {
                    source: SourceKind::HuggingFace,
                    repo: format!("{org}/{repo}"),
                    path: None,
                })
            }
            3 => {
                let org = parts[0];
                let repo = parts[1];
                let file = parts[2];
                if org.is_empty() || repo.is_empty() || file.is_empty() {
                    return Err(RegistryError::Parse(format!(
                        "HF single-file id must have three non-empty segments: {trimmed:?}"
                    )));
                }
                if file.contains(' ') {
                    return Err(RegistryError::Parse(format!(
                        "HF single-file file segment must not contain spaces: {trimmed:?}"
                    )));
                }
                Ok(Self {
                    source: SourceKind::HuggingFace,
                    repo: format!("{org}/{repo}"),
                    path: Some(vec![file.to_string()]),
                })
            }
            n => Err(RegistryError::Parse(format!(
                "HF id must have 2 or 3 segments, got {n}: {trimmed:?}"
            ))),
        }
    }

    /// Render back into the canonical string form.
    pub fn canonical(&self) -> String {
        match (&self.source, &self.path) {
            (SourceKind::HuggingFace, None) => self.repo.clone(),
            (SourceKind::HuggingFace, Some(p)) if !p.is_empty() => {
                format!("{}/{}", self.repo, p.join("/"))
            }
            (SourceKind::Local, _) => self.repo.clone(),
            (SourceKind::HuggingFace, Some(p)) => {
                // Defensive: ModelId's parser never produces an empty vec,
                // but a programmatic construction could. Render it the
                // same way as None (just the repo) so we don't silently
                // lose information — and add a debug_assert so the
                // invariant stays loud.
                debug_assert!(!p.is_empty(), "ModelId::path should not be empty");
                self.repo.clone()
            }
        }
    }
}

impl fmt::Display for ModelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.canonical())
    }
}

impl FromStr for ModelId {
    type Err = RegistryError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_two_segment_hf() {
        let id = ModelId::parse("Qwen/Qwen3-ASR-0.6B").unwrap();
        assert_eq!(id.source, SourceKind::HuggingFace);
        assert_eq!(id.repo, "Qwen/Qwen3-ASR-0.6B");
        assert!(id.path.is_none());
    }

    #[test]
    fn parse_three_segment_hf() {
        let id = ModelId::parse("ggerganov/whisper.cpp/ggml-large-v3.bin").unwrap();
        assert_eq!(id.source, SourceKind::HuggingFace);
        assert_eq!(id.repo, "ggerganov/whisper.cpp");
        assert_eq!(id.path, Some(vec!["ggml-large-v3.bin".to_string()]));
    }

    #[test]
    fn parse_local_absolute() {
        let id = ModelId::parse("/cache/models/qwen").unwrap();
        assert_eq!(id.source, SourceKind::Local);
        assert_eq!(id.repo, "/cache/models/qwen");
    }

    #[test]
    fn parse_local_relative() {
        let id = ModelId::parse("./local-model").unwrap();
        assert_eq!(id.source, SourceKind::Local);
    }

    #[test]
    fn parse_rejects_too_many_segments() {
        assert!(ModelId::parse("a/b/c/d").is_err());
    }

    #[test]
    fn parse_rejects_empty_segments() {
        // `/repo/file` starts with `/` so the parser correctly treats
        // it as a local path; the empty-segment check is only about
        // HF ids (org//file → empty repo segment).
        assert!(ModelId::parse("org//file").is_err());
    }

    #[test]
    fn parse_rejects_empty_input() {
        assert!(ModelId::parse("").is_err());
        assert!(ModelId::parse("   ").is_err());
    }

    #[test]
    fn canonical_round_trips() {
        for s in [
            "Qwen/Qwen3-ASR-0.6B",
            "ggerganov/whisper.cpp/ggml-large-v3.bin",
            "/cache/models/qwen",
            "./local-model",
        ] {
            let id = ModelId::parse(s).unwrap();
            assert_eq!(id.canonical(), s, "round-trip for {s:?}");
        }
    }
}
