//! Unit tests for `voxora run`.

use super::*;
use clap::Parser;

#[tokio::test]
async fn rejects_model_id_without_slash() {
    let cli = crate::Cli::try_parse_from(["voxora", "run", "noslash", "a.wav"]).unwrap();
    let opts = RunOpts {
        model_id: "noslash".into(),
        audio: std::path::PathBuf::from("a.wav"),
        revision: None,
        engine: None,
        language: None,
        translate: false,
        timestamps: false,
        force_redownload: false,
    };
    let err = run(&cli, &opts).await.unwrap_err();
    assert!(matches!(err, CliError::InvalidInput(_)));
    assert_eq!(err.exit_code(), 2);
}
