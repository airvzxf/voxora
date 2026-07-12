//! Unit tests for `voxora info`.

use super::*;
use clap::Parser;

fn opts(model_id: &str) -> InfoOpts {
    InfoOpts {
        model_id: model_id.into(),
        revision: Some("main".into()),
    }
}

#[tokio::test]
async fn rejects_model_id_without_slash() {
    let cli = crate::Cli::try_parse_from(["voxora", "info", "noslash"]).unwrap();
    let err = run(&cli, &opts("noslash")).await.unwrap_err();
    assert!(matches!(err, CliError::InvalidInput(_)));
    assert_eq!(err.exit_code(), 2);
}
