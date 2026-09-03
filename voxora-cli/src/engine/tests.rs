//! Unit tests for the engine auto-selection + dispatch helpers.

use super::*;
use voxora_traits::ModelCapabilities;

#[test]
fn engine_family_crate_labels() {
    assert_eq!(EngineFamily::Whisper.crate_label(), "voxora-whisper");
    assert_eq!(EngineFamily::Qwen3Asr.crate_label(), "voxora-qwen3asr");
}

#[test]
fn from_cli_label_accepts_canonical_spelling() {
    assert_eq!(from_cli_label("whisper").unwrap(), EngineFamily::Whisper);
    assert_eq!(from_cli_label("qwen3-asr").unwrap(), EngineFamily::Qwen3Asr);
}

#[test]
fn from_cli_label_accepts_underscore_and_collapsed() {
    assert_eq!(from_cli_label("qwen3_asr").unwrap(), EngineFamily::Qwen3Asr);
    assert_eq!(from_cli_label("qwen3asr").unwrap(), EngineFamily::Qwen3Asr);
}

#[test]
fn from_cli_label_is_case_insensitive() {
    assert_eq!(from_cli_label("WHISPER").unwrap(), EngineFamily::Whisper);
    assert_eq!(from_cli_label("QWEN3-Asr").unwrap(), EngineFamily::Qwen3Asr);
}

#[test]
fn from_cli_label_rejects_unknown() {
    let err = from_cli_label("parakeet").unwrap_err();
    assert!(matches!(err, CliError::InvalidInput(_)));
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn infer_kind_from_capabilities_picks_whisper_on_word_timestamps() {
    let caps = ModelCapabilities::new(true, true, false, vec!["en".into()]);
    assert_eq!(
        infer_kind_from_capabilities(&caps),
        Some(EngineFamily::Whisper)
    );
}

#[test]
fn infer_kind_from_capabilities_picks_qwen3_on_no_word_timestamps() {
    let caps = ModelCapabilities::new(true, false, false, vec!["english".into()]);
    assert_eq!(
        infer_kind_from_capabilities(&caps),
        Some(EngineFamily::Qwen3Asr)
    );
}

#[test]
fn infer_kind_from_capabilities_rejects_when_unknown() {
    let caps = ModelCapabilities::UNKNOWN;
    assert!(infer_kind_from_capabilities(&caps).is_none());

    let mono = ModelCapabilities::new(false, false, false, vec!["en".into()]);
    assert!(infer_kind_from_capabilities(&mono).is_none());
}
