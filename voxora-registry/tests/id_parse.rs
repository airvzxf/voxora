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
