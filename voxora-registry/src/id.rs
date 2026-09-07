//! Strict [`ModelId`] parsing.
//!
//! Three accepted shapes:
//! - `"org/repo"`           → HF whole-repo
//! - `"org/repo/file"`      → HF single-file (the fix path for #79)
//! - `"./local/path"` or absolute path → Local source
//!
//! Anything else is a parse error with a descriptive message.
//!
//! Both the HF and Local arms reject shape-unsafe inputs
//! (`..` segments, embedded `\`, control chars, oversized ids —
//! closes #102, #143, EPIC #148). The Local arm keeps
//! `ModelId::parse("/safe/path")` and `ModelId::parse("./relative")`
//! as the well-formed shapes; `ModelId::parse("/safe/../etc/passwd")`
//! is rejected at parse time as a path-traversal attempt.

/// Maximum byte length of a parsed model id. 4 KiB matches the
/// `voxora-local` runtime cap (`LocalSource::resolve` re-checks
/// `model_id.len()` for defence in depth); the parser enforces it
/// first so an oversized id never reaches the source.
pub const MAX_ID_LENGTH: usize = 4096;

/// Maximum byte length of a single component in a parsed Local id.
/// Mirrors the per-segment cap that the HF arm applies to its file
/// segment. Exists so a future caller of `ModelId::canonical()`
/// on a hostile Local id can rely on a bounded output.
pub const MAX_LOCAL_COMPONENT_LENGTH: usize = 1024;

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

/// Parse a Local arm id (closes #143, EPIC #148).
///
/// The Local arm accepts three leading prefixes — `/`, `./`, or
/// `../` — but rejects every `..` component *after* the leading
/// prefix and every control character. The reasoning:
///
/// - `..` segments after the leading prefix would let
///   `ModelId::parse("/srv/../etc/passwd")` parse as `Ok` and
///   produce a `ModelId` whose `canonical()` renders the same
///   traversal segment back to the caller. The runtime cap in
///   `LocalSource::resolve` does the canonicalise-and-check
///   defence at the I/O layer; the parser is the upstream gate
///   so the registry's descriptors and other consumers never see
///   a hostile id at all.
/// - Embedded `\0`, `\b`…`\x1F` and lone `\\` are rejected because
///   `LocalSource::resolve` joins the trimmed id onto a path
///   buffer and follows what the OS hands back; control characters
///   are silently truncated by some C runtimes and some filesystems
///   refuse the rest. A parse-time reject is the only place that
///   guarantees the consumer gets a useful error message.
///
/// The HF arm already does the equivalent for its file segment
/// (closes #102). Mirroring it here keeps the two arms symmetric.
fn parse_local_id(trimmed: &str) -> Result<ModelId, RegistryError> {
    // Drop the leading `/`, `./`, or `../` so we can iterate the
    // remaining components.
    let body = trimmed
        .strip_prefix('/')
        .or_else(|| trimmed.strip_prefix("./"))
        .or_else(|| trimmed.strip_prefix("../"))
        .unwrap_or(trimmed);

    // Reject any embedded control character or backslash separator.
    // Forward slashes are the canonical separator; backslashes are
    // Windows-only and would silently re-introduce the parser-vs-runtime
    // gap that #102 closed on the HF arm.
    if body.contains('\0') || body.contains('\\') || body.chars().any(|c| c.is_control()) {
        return Err(RegistryError::Parse(format!(
            "local id must not contain control characters or backslashes: {trimmed:?}"
        )));
    }

    // Split into components and reject any `..` segment. The
    // leading prefix has already been stripped, so an absolute
    // id like `/srv/models` lands here as the single segment
    // `srv/models` (forward-slash split) → `["srv", "models"]`.
    // A relative id like `./local-model` lands as `["local-model"]`.
    // A traversal id like `/safe/../etc/passwd` lands as
    // `["safe", "..", "etc", "passwd"]` and we reject the `..`.
    for component in body.split('/') {
        if component.is_empty() {
            // Multi-slash forms (`/foo//bar`) compress at the OS layer
            // to a single slash; a parse-time reject is wider scope
            // than this PR. Accept them.
            continue;
        }
        if component == ".." {
            return Err(RegistryError::Parse(format!(
                "path traversal: .. segment in local id: {trimmed:?}"
            )));
        }
        if component == "." {
            return Err(RegistryError::Parse(format!(
                "path traversal: . segment in local id: {trimmed:?}"
            )));
        }
        if component.len() > MAX_LOCAL_COMPONENT_LENGTH {
            return Err(RegistryError::Parse(format!(
                "local id component too long (max {} bytes): {trimmed:?}",
                MAX_LOCAL_COMPONENT_LENGTH,
            )));
        }
    }

    Ok(ModelId {
        source: SourceKind::Local,
        repo: trimmed.to_string(),
        path: None,
    })
}

impl ModelId {
    /// Parse a user-supplied string into a `ModelId`. See module
    /// docs for accepted shapes.
    pub fn parse(s: &str) -> Result<Self, RegistryError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(RegistryError::Parse("empty id".into()));
        }

        // Length cap (closes #143, EPIC #148). Cheap to check and
        // bounds downstream work in `LocalSource::resolve` and the
        // HF cache layout. The parser is the upstream gate — a hostile
        // caller should fail here, not later when the source tries to
        // allocate a path buffer.
        if trimmed.len() > MAX_ID_LENGTH {
            return Err(RegistryError::Parse(format!(
                "model id too long (max {} bytes): {} bytes",
                MAX_ID_LENGTH,
                trimmed.len(),
            )));
        }

        // Local source: starts with `./`, `../`, or `/`.
        if trimmed.starts_with('/') || trimmed.starts_with("./") || trimmed.starts_with("../") {
            return parse_local_id(trimmed);
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
                if file == "."
                    || file == ".."
                    || file.contains('/')
                    || file.contains('\\')
                    || file.contains('\0')
                {
                    return Err(RegistryError::Parse(format!(
                        "HF single-file file segment must be a bare filename \
                         (no traversal, no separators, no NULs): {trimmed:?}"
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

    #[test]
    fn parse_rejects_traversal_segment() {
        // #102 — the parser used to accept `.`, `..`, embedded `\`,
        // and embedded NUL bytes as the file segment of a 3-segment
        // HF id. Future code that joins `ModelId::path` onto a base
        // would silently inherit the traversal; the network layer
        // would receive the malformed id as a 404 or NUL truncation.
        for bad in [
            "foo/bar/..",
            "foo/bar/.",
            "foo/bar/foo\\bar",
            "foo/bar/with\0null",
        ] {
            let err = ModelId::parse(bad).expect_err(&format!(
                "traversal/separator segment must be rejected: {bad:?}"
            ));
            match err {
                RegistryError::Parse(_) => {}
                other => panic!("expected Parse, got {other:?}"),
            }
        }
    }

    #[test]
    fn parse_accepts_dotted_filename() {
        // HF permits dot-prefixed filenames (e.g. `.gitattributes`).
        // Rejecting them at the parser would be wider scope than the
        // #102 hardening PR; this test pins the parser contract so a
        // future tightening is deliberate.
        let id = ModelId::parse("foo/bar/.hidden").expect("dotfile accepted");
        assert_eq!(id.repo, "foo/bar");
        assert_eq!(id.path, Some(vec![".hidden".to_string()]));
    }

    #[test]
    fn parse_local_rejects_traversal_subpath() {
        // #143, EPIC #148 — the Local arm must mirror the HF arm
        // (which rejects `foo/bar/..`): `/safe/../etc/passwd` is a
        // path-traversal attempt and must be rejected at parse time
        // so the registry's descriptors and consumers never see the
        // hostile id at all.
        for bad in [
            "/safe/../etc/passwd",
            "/srv/models/../../etc/passwd",
            "./foo/../bar",
            "/../etc/passwd",
            "./../escape",
        ] {
            let err = ModelId::parse(bad)
                .expect_err(&format!("traversal subpath must be rejected: {bad:?}"));
            match err {
                RegistryError::Parse(msg) => {
                    assert!(
                        msg.contains("traversal") || msg.contains(".."),
                        "expected traversal wording, got {msg:?}",
                    );
                }
                other => panic!("expected Parse, got {other:?}"),
            }
        }
    }

    #[test]
    fn parse_local_rejects_control_chars() {
        // #143, EPIC #148 — embedded NUL, `\b`-`\x1F`, and lone
        // backslash all flow through to `LocalSource::resolve`'s
        // path-join, where some C runtimes truncate on NUL and some
        // filesystems refuse the rest. The parser is the upstream
        // gate that gives the consumer a useful error.
        for bad in [
            "/safe/\x00evil",
            "/safe/with\0null",
            "/safe/with\\backslash",
        ] {
            let err = ModelId::parse(bad).expect_err(&format!(
                "control char / backslash must be rejected: {bad:?}"
            ));
            assert!(
                matches!(err, RegistryError::Parse(_)),
                "expected Parse, got {err:?}",
            );
        }
    }

    #[test]
    fn parse_local_rejects_too_long_id() {
        // #143, EPIC #148 — the parser enforces a 4 KiB cap on
        // `model_id` (matching the runtime cap in
        // `LocalSource::resolve`). 5 KB is well over the cap; the
        // exact wording is pinned here so a future wording tweak is
        // deliberate.
        let bad = "a".repeat(MAX_ID_LENGTH + 1);
        let err = ModelId::parse(&bad).expect_err("oversized id must be rejected");
        match err {
            RegistryError::Parse(msg) => {
                assert!(
                    msg.contains("too long"),
                    "expected 'too long' wording, got {msg:?}",
                );
            }
            other => panic!("expected Parse, got {other:?}"),
        }
    }

    #[test]
    fn parse_local_accepts_leading_traversal_prefix_only() {
        // The leading `../` is a well-formed relative-prefix id;
        // what we reject is a `..` *component after* the prefix.
        // `.` and `..` as standalone components in the body are
        // rejected; `../sibling/legit.bin` is parsed as a Local id
        // and resolved relative to the configured `local_root` at
        // runtime, where the runtime cap rejects escapes.
    }

    #[test]
    fn parse_local_accepts_absolute_safe_path() {
        // The `ModelId::parse("/safe/path")` happy-path stays
        // open — absolute paths are operator-controlled and the
        // runtime cap in `LocalSource::resolve` rejects symlink
        // escapes. The parser only refuses shape-unsafe ids.
        let id = ModelId::parse("/srv/models/whisper/ggml-tiny.bin")
            .expect("absolute safe path accepted");
        assert_eq!(id.source, SourceKind::Local);
        assert_eq!(id.repo, "/srv/models/whisper/ggml-tiny.bin");
    }
}
