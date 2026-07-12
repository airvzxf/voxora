//! Unit tests for the `CliError` exit-code mapping.

use super::*;
use std::error::Error as _;
use voxora_core::AsrError;

#[test]
fn invalid_input_maps_to_exit_code_two() {
    let err = CliError::InvalidInput("bad model id".into());
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn build_error_maps_to_exit_code_two() {
    let err = CliError::Build("no `whisper` feature".into());
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn asr_runtime_error_maps_to_exit_code_one() {
    let err = CliError::Asr(AsrError::ModelNotFound("nope".into()));
    assert_eq!(err.exit_code(), 1);
}

#[test]
fn asr_error_source_chain_walks_through() {
    let err = CliError::Asr(AsrError::Inference("boom".into()));
    let source = err.source().expect("source chain");
    let msg = source.to_string();
    assert!(msg.contains("boom"));
}

#[test]
fn from_asr_error_round_trips() {
    let cli: CliError = AsrError::Inference("boom".into()).into();
    match cli {
        CliError::Asr(AsrError::Inference(s)) => assert_eq!(s, "boom"),
        other => panic!("expected Asr(Inference), got {other:?}"),
    }
}
