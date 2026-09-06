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

#[test]
fn ensure_hardware_available_accepts_cpu() {
    // CPU is always compiled in.
    assert!(ensure_hardware_available(BackendKind::Cpu, "cpu").is_ok());
}

#[test]
fn ensure_hardware_compatible_with_engine_rejects_vulkan_qwen3asr() {
    let err = ensure_hardware_compatible_with_engine(
        BackendKind::Vulkan,
        EngineFamily::Qwen3Asr,
        "vulkan",
    )
    .unwrap_err();
    assert!(matches!(err, CliError::Build(_)));
    assert_eq!(err.exit_code(), 2);
    let rendered = format!("{err}");
    assert!(rendered.contains("qwen3-asr"));
    assert!(rendered.contains("Vulkan"));
}

#[test]
fn ensure_hardware_compatible_with_engine_allows_vulkan_whisper() {
    // Whisper + Vulkan is the supported combination (closes #121).
    assert!(
        ensure_hardware_compatible_with_engine(
            BackendKind::Vulkan,
            EngineFamily::Whisper,
            "vulkan"
        )
        .is_ok()
    );
}

#[test]
fn ensure_hardware_compatible_with_engine_allows_non_vulkan_qwen3asr() {
    // CUDA / Metal / CPU are still OK for qwen3-asr.
    for hw in [BackendKind::Cpu, BackendKind::Cuda, BackendKind::Metal] {
        assert!(
            ensure_hardware_compatible_with_engine(hw, EngineFamily::Qwen3Asr, hw.as_config())
                .is_ok(),
            "{hw:?} + qwen3-asr should be allowed"
        );
    }
}
