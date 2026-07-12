//! Unit tests for `voxora download`.

use super::*;
use clap::Parser;
use voxora_core::QuantizationPreference;

fn opts(quantization: &str, model_id: &str) -> DownloadOpts {
    DownloadOpts {
        model_id: model_id.into(),
        revision: None,
        quantization: quantization.into(),
    }
}

#[tokio::test]
async fn rejects_model_id_without_slash() {
    let cli = crate::Cli::try_parse_from(["voxora", "download", "noslash"]).unwrap();
    let err = run(&cli, &opts("auto", "noslash")).await.unwrap_err();
    assert!(matches!(err, CliError::InvalidInput(_)));
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn parse_quantization_accepts_canonical_spelling() {
    use super::parse_quantization;
    assert_eq!(
        parse_quantization("auto").unwrap(),
        QuantizationPreference::Auto
    );
    assert_eq!(
        parse_quantization("f32").unwrap(),
        QuantizationPreference::F32
    );
    assert_eq!(
        parse_quantization("bf16").unwrap(),
        QuantizationPreference::Bf16
    );
    assert_eq!(
        parse_quantization("bfloat16").unwrap(),
        QuantizationPreference::Bf16,
    );
    assert_eq!(
        parse_quantization("f16").unwrap(),
        QuantizationPreference::F16
    );
    assert_eq!(
        parse_quantization("float16").unwrap(),
        QuantizationPreference::F16,
    );
    assert_eq!(
        parse_quantization("q4_k").unwrap(),
        QuantizationPreference::Q4K
    );
    assert_eq!(
        parse_quantization("Q4_K_M").unwrap(),
        QuantizationPreference::Q4K
    );
    assert_eq!(
        parse_quantization("q8_0").unwrap(),
        QuantizationPreference::Q8_0
    );
    assert_eq!(
        parse_quantization("Q8").unwrap(),
        QuantizationPreference::Q8_0
    );
}

#[test]
fn parse_quantization_rejects_unknown() {
    let err = parse_quantization("int4-quant").unwrap_err();
    assert!(matches!(err, CliError::InvalidInput(_)));
    assert_eq!(err.exit_code(), 2);
}
