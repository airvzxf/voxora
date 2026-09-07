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
    assert_eq!(id.path.as_deref(), Some([".hidden".to_string()].as_slice()));
}

#[test]
fn parses_local_rejects_traversal_subpath() {
    // #143, EPIC #148 — the Local arm mirrors the HF arm's #102
    // guard. `ModelId::parse("/safe/../etc/passwd")` would, before
    // the hardening, return `Ok` and produce a `ModelId` whose
    // canonical string re-emits the traversal segment. The parser
    // now refuses any `..` component after the leading prefix.
    for bad in [
        "/safe/../etc/passwd",
        "/srv/models/../../etc/passwd",
        "./foo/../bar",
    ] {
        let err = ModelId::parse(bad).expect_err(&format!("must reject: {bad:?}"));
        match err {
            RegistryError::Parse(msg) => {
                assert!(
                    msg.contains("traversal") || msg.contains(".."),
                    "expected traversal wording for {bad:?}, got {msg:?}",
                );
            }
            other => panic!("expected Parse, got {other:?} for {bad:?}"),
        }
    }
}

#[test]
fn parses_local_rejects_control_chars() {
    // #143, EPIC #148 — embedded NUL, control characters, and
    // lone backslashes all flow through to `LocalSource::resolve`'s
    // path-join, where some C runtimes truncate on NUL and some
    // filesystems refuse the rest. The parser rejects them at
    // the upstream gate so the caller gets a useful error message.
    for bad in [
        "/safe/\x00evil",
        "/safe/with\0null",
        "/safe/with\\backslash",
    ] {
        assert!(
            matches!(ModelId::parse(bad), Err(RegistryError::Parse(_))),
            "must reject {bad:?}",
        );
    }
}

#[test]
fn parses_local_rejects_too_long_id() {
    // #143, EPIC #148 — the parser enforces a 4 KiB cap on
    // `model_id` (matching the runtime cap in
    // `LocalSource::resolve`). 5 KB is well over the cap.
    let bad = "a".repeat(5000);
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
fn parses_local_accepts_leading_traversal_prefix_only() {
    // The leading `../` is a well-formed relative-prefix id;
    // what we reject is a `..` *component after* the prefix.
    // `../sibling/legit.bin` parses as a Local id and is then
    // rejected by `LocalSource::resolve` if the resolved path
    // escapes the configured root.
    let id = ModelId::parse("../sibling/legit.bin").expect("leading ../ is OK");
    assert_eq!(id.source, SourceKind::Local);
    assert_eq!(id.repo, "../sibling/legit.bin");
}

#[test]
fn parses_local_accepts_safe_absolute_paths() {
    // Regression guard for the happy-path: `ModelId::parse`
    // continues to accept absolute paths that do not contain
    // traversal segments. Runtime-side symlink guards in
    // `LocalSource::resolve` catch symlink escapes; the parser
    // only refuses shape-unsafe ids.
    for good in [
        "/srv/models/qwen",
        "/srv/models/whisper/ggml-tiny.bin",
        "/srv/models/qwen-1.7B/model.safetensors",
    ] {
        let id = ModelId::parse(good).expect("absolute safe path accepted");
        assert_eq!(id.source, SourceKind::Local);
        assert_eq!(id.repo, good);
        assert!(id.path.is_none());
    }
}
