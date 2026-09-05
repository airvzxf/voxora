//! Integration test: ModelId::parse accepts canonical inputs and
//! rejects ambiguous ones.

use voxora_registry::{ModelId, RegistryError, SourceKind};

#[test]
fn parses_org_repo() {
    let id = ModelId::parse("Qwen/Qwen3-ASR-0.6B").unwrap();
    assert_eq!(id.source, SourceKind::HuggingFace);
    assert!(id.path.is_none());
}

#[test]
fn parses_org_repo_file() {
    let id = ModelId::parse("ggerganov/whisper.cpp/ggml-large-v3.bin").unwrap();
    assert_eq!(id.source, SourceKind::HuggingFace);
    assert_eq!(
        id.path.as_deref(),
        Some(["ggml-large-v3.bin".to_string()].as_slice())
    );
}

#[test]
fn parses_local_absolute() {
    let id = ModelId::parse("/srv/models/qwen").unwrap();
    assert_eq!(id.source, SourceKind::Local);
}

#[test]
fn parses_local_relative() {
    let id = ModelId::parse("./local-model").unwrap();
    assert_eq!(id.source, SourceKind::Local);
}

#[test]
fn rejects_four_segment_id() {
    assert!(matches!(
        ModelId::parse("a/b/c/d"),
        Err(RegistryError::Parse(_))
    ));
}

#[test]
fn rejects_empty_segments() {
    // `/repo/file` starts with `/` so the parser treats it as a
    // local path; only `org//file` exercises the HF empty-segment
    // branch.
    assert!(ModelId::parse("org//file").is_err());
    assert!(ModelId::parse("").is_err());
}

#[test]
fn canonical_string_round_trip() {
    for s in [
        "Qwen/Qwen3-ASR-0.6B",
        "ggerganov/whisper.cpp/ggml-large-v3.bin",
        "./local",
    ] {
        let id = ModelId::parse(s).unwrap();
        assert_eq!(id.canonical(), s);
    }
}

#[test]
fn rejects_traversal_and_separator_file_segment() {
    // #102 — mirror of the unit-test coverage: parser must reject
    // `.`, `..`, embedded `\`, and embedded NUL bytes in the file
    // segment of a 3-segment HF id.
    for bad in [
        "foo/bar/..",
        "foo/bar/.",
        "foo/bar/foo\\bar",
        "foo/bar/with\0null",
    ] {
        assert!(
            matches!(ModelId::parse(bad), Err(RegistryError::Parse(_))),
            "must reject {bad:?}"
        );
    }
}

#[test]
fn accepts_dotfile_filename() {
    // HF permits dot-prefixed filenames (`.gitattributes`, etc.);
    // the parser must keep accepting them. Mirrors the unit test.
    let id = ModelId::parse("foo/bar/.hidden").unwrap();
    assert_eq!(id.repo, "foo/bar");
    assert_eq!(
        id.path.as_deref(),
        Some([".hidden".to_string()].as_slice())
    );
}
