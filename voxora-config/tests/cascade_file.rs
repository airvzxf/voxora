//! Integration test: TOML file overrides defaults.

use std::io::Write;

use voxora_config::VoxoraConfig;

#[test]
fn file_overrides_cache_root() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("voxora.toml");
    let mut f = std::fs::File::create(&path).expect("create");
    writeln!(
        f,
        r#"
        [cache]
        root = "/tmp/from-file-cache"
    "#
    )
    .expect("write");

    let cfg = VoxoraConfig::from_file(&path).expect("from_file");
    assert_eq!(
        cfg.cache_root(),
        std::path::PathBuf::from("/tmp/from-file-cache")
    );
}

#[test]
fn file_overrides_hf_token_and_base_url() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("voxora.toml");
    std::fs::write(
        &path,
        r#"
        [hf]
        token = "file-token"
        base_url = "http://file-host:8080"
        default_revision = "v0.0.1"
    "#,
    )
    .expect("write");

    let cfg = VoxoraConfig::from_file(&path).expect("from_file");
    assert_eq!(cfg.hf_token().as_deref(), Some("file-token"));
    assert_eq!(cfg.hf_base_url(), "http://file-host:8080");
    assert_eq!(cfg.hf_default_revision(), "v0.0.1");
}

#[test]
fn missing_file_errors() {
    let err = VoxoraConfig::from_file(std::path::Path::new("/nonexistent/voxora.toml"))
        .expect_err("missing file errors");
    assert!(matches!(err, voxora_config::ConfigError::FileNotFound(_)));
}

#[test]
fn invalid_toml_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("voxora.toml");
    std::fs::write(&path, "this is not = = toml").expect("write");

    let err = VoxoraConfig::from_file(&path).expect_err("invalid TOML errors");
    assert!(matches!(err, voxora_config::ConfigError::FileParse { .. }));
}
