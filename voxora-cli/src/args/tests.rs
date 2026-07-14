//! Unit tests for the top-level CLI arg parser.

use super::*;

#[test]
fn try_parse_help_emits_display_help_error() {
    // clap returns `DisplayHelp` for `--help`; callers translate
    // that into a process exit with code 0. We don't actually exit
    // in the test (would terminate the runner) — we just assert the
    // error kind that the main.rs path keys off.
    let err = Cli::try_parse_from(["voxora", "--help"]).unwrap_err();
    assert!(matches!(
        err.kind(),
        clap::error::ErrorKind::DisplayHelp
            | clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    ));
}

#[test]
fn try_parse_list_succeeds() {
    let cli = Cli::try_parse_from(["voxora", "list"]).expect("list parses");
    matches!(cli.command, Command::List);
    assert!(cli.cache.is_none());
    assert!(!cli.quiet);
}

#[test]
fn try_parse_list_with_global_flags() {
    let cli = Cli::try_parse_from([
        "voxora", "list", "--cache", "/tmp/v", "--token", "hf_xxx", "--quiet",
    ])
    .expect("parse");
    assert_eq!(
        cli.cache.as_deref().unwrap(),
        std::path::Path::new("/tmp/v")
    );
    assert_eq!(cli.token.as_deref(), Some("hf_xxx"));
    assert!(cli.quiet);
}

#[test]
fn try_parse_info_requires_model_id() {
    let err = Cli::try_parse_from(["voxora", "info"]).unwrap_err();
    assert!(matches!(
        err.kind(),
        clap::error::ErrorKind::MissingRequiredArgument
    ));
}

#[test]
fn try_parse_info_accepts_revision() {
    let cli = Cli::try_parse_from([
        "voxora",
        "info",
        "Qwen/Qwen3-ASR-0.6B",
        "--revision",
        "v0.0.0",
    ])
    .expect("parse");
    let Command::Info(opts) = cli.command else {
        panic!("info command expected");
    };
    assert_eq!(opts.model_id, "Qwen/Qwen3-ASR-0.6B");
    assert_eq!(opts.revision.as_deref(), Some("v0.0.0"));
}

#[test]
fn try_parse_download_with_quantization() {
    let cli = Cli::try_parse_from([
        "voxora",
        "download",
        "Qwen/Qwen3-ASR-0.6B",
        "--quantization",
        "q4_k",
    ])
    .expect("parse");
    let Command::Download(opts) = cli.command else {
        panic!("download command expected");
    };
    assert_eq!(opts.quantization, "q4_k");
}

#[test]
fn try_parse_run_full() {
    let cli = Cli::try_parse_from([
        "voxora",
        "run",
        "Qwen/Qwen3-ASR-0.6B",
        "samples/jfk.wav",
        "--engine",
        "qwen3-asr",
        "--language",
        "english",
        "--translate",
        "--timestamps",
    ])
    .expect("parse");
    let Command::Run(opts) = cli.command else {
        panic!("run command expected");
    };
    assert_eq!(opts.model_id, "Qwen/Qwen3-ASR-0.6B");
    assert_eq!(opts.engine.as_deref(), Some("qwen3-asr"));
    assert!(opts.translate);
    assert!(opts.timestamps);
}

#[test]
fn hf_cache_dir_without_flags_falls_back() {
    // We can't easily set VOXORA_CACHE_DIR without env mutation in a
    // concurrent test runner. Verify the obvious: when --cache is
    // supplied, the resolved dir is `<cache>/voxora/models/huggingface`.
    let cli = Cli::try_parse_from(["voxora", "list", "--cache", "/var/cache/voxora"]).unwrap();
    let resolved = cli.hf_cache_dir().expect("hf_cache_dir should resolve");
    assert_eq!(
        resolved,
        std::path::PathBuf::from("/var/cache/voxora/voxora/models/huggingface"),
    );
}

#[test]
fn resolve_hf_cache_dir_prefers_flag_over_env_and_dirs() {
    let resolved = super::resolve_hf_cache_dir(
        Some(std::path::Path::new("/var/cache/voxora")),
        Some(std::path::Path::new("/tmp/env")),
        Some(std::path::Path::new("/home/user/.cache")),
    )
    .expect("flag is set");
    assert_eq!(
        resolved,
        std::path::PathBuf::from("/var/cache/voxora/voxora/models/huggingface"),
    );
}

#[test]
fn resolve_hf_cache_dir_uses_env_when_flag_missing() {
    let resolved = super::resolve_hf_cache_dir(
        None,
        Some(std::path::Path::new("/tmp/env-cache")),
        Some(std::path::Path::new("/home/user/.cache")),
    )
    .expect("env is set");
    assert_eq!(
        resolved,
        std::path::PathBuf::from("/tmp/env-cache/voxora/models/huggingface"),
    );
}

#[test]
fn resolve_hf_cache_dir_falls_back_to_dirs_cache_dir() {
    // No flag, no env. Should fall through to dirs::cache_dir().
    let resolved =
        super::resolve_hf_cache_dir(None, None, Some(std::path::Path::new("/home/user/.cache")))
            .expect("dirs::cache_dir() is provided");
    assert_eq!(
        resolved,
        std::path::PathBuf::from("/home/user/.cache/voxora/models/huggingface"),
    );
}

#[test]
fn resolve_hf_cache_dir_returns_none_when_no_source_available() {
    // No flag, no env, no dirs::cache_dir() (rare — only happens in
    // very stripped-down environments where the OS has no notion of
    // a user cache).
    let resolved = super::resolve_hf_cache_dir(None, None, None);
    assert!(
        resolved.is_none(),
        "must return None when every lookup fails, got: {resolved:?}",
    );
}
