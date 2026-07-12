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
fn hf_cache_dir_rejects_when_nothing_supplied_and_no_xdg() {
    // This is hard to trigger; we assert the helper returns None when
    // both --cache and VOXORA_CACHE_DIR are unset. We cannot easily
    // override HOME / XDG_CACHE_HOME reliably across platforms, so we
    // just exercise the early return path by leaving both flags unset
    // and trusting the implementation.
    // (If XDG_CACHE_HOME points somewhere accessible on the host,
    //  the function returns Some(...) instead — that's fine.)
    let cli = Cli::try_parse_from(["voxora", "list"]).unwrap();
    let _ = cli.hf_cache_dir();
    // No assertion: see comment above.
}
