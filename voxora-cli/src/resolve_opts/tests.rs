//! Unit tests for the `ResolveOptions` builder helper.

use super::*;
use voxora_core::QuantizationPreference;

#[test]
fn build_resolve_opts_yields_defaults_when_caller_does_nothing() {
    let opts = build_resolve_opts(None, None, |_| Ok(())).expect("ok");
    assert!(opts.token.is_none());
    assert!(opts.revision.is_none());
    assert_eq!(opts.quantization, QuantizationPreference::Auto);
}

#[test]
fn build_resolve_opts_forwards_token_and_revision() {
    let opts = build_resolve_opts(Some("hf_xxx"), Some("v1"), |_| Ok(())).expect("ok");
    assert_eq!(opts.token.as_deref(), Some("hf_xxx"));
    assert_eq!(opts.revision.as_deref(), Some("v1"));
}

#[test]
fn build_resolve_opts_passes_quantization_through_callback() {
    let opts = build_resolve_opts(None, None, |ro| {
        ro.quantization = QuantizationPreference::Q4K;
        Ok(())
    })
    .expect("ok");
    assert_eq!(opts.quantization, QuantizationPreference::Q4K);
}

#[test]
fn build_resolve_opts_propagates_callback_error() {
    let err =
        build_resolve_opts(None, None, |_| Err(CliError::InvalidInput("nope".into()))).unwrap_err();
    assert!(matches!(err, CliError::InvalidInput(_)));
    assert_eq!(err.exit_code(), 2);
}
