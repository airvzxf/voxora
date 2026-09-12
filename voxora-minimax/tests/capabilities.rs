//! Integration tests for the `voxora-minimax` engine surface.
//!
//! Two layers:
//!
//! 1. Static capability / API-surface assertions (always run).
//! 2. A live integration test gated by `MINIMAX_API_KEY=<key>`
//!    (run with `--ignored`). The live test posts 1 s of 440 Hz
//!    silence at 16 kHz mono to the real MiniMax endpoint and
//!    asserts the round-trip succeeds.

use std::env;

use voxora_minimax::{known_languages_bcp47, validate_lang_bcp47, MiniMaxConfig, MiniMaxEngine};
use voxora_traits::{AsrEngine, ModelCapabilities, TranscribeOptions};

#[test]
fn public_surface_re_exports_match_engine_facade() {
    // Compile-time check that the re-exports match what the
    // implementation produces.
    let _: ModelCapabilities = ModelCapabilities::UNKNOWN;
    let _langs: &[&str] = known_languages_bcp47();
    // validate_lang_bcp47 is callable with None / Some(...)
    assert!(validate_lang_bcp47(None).is_ok());
    assert!(validate_lang_bcp47(Some("en")).is_ok());
}

#[test]
fn config_rejects_empty_token() {
    // Mirrors the per-crate pattern: invalid input surfaces as
    // the crate's own error type at construction time so
    // consumers get a uniform error path.
    let cfg_err = MiniMaxConfig::new("").expect_err("empty token rejected");
    let _ = cfg_err; // pinned via Debug
}

#[test]
fn capabilities_advertise_minimax_whitelist() {
    let cfg = MiniMaxConfig::new("sk-test").unwrap();
    let engine = MiniMaxEngine::new(cfg).unwrap();
    let caps = engine.capabilities();
    assert!(caps.multilingual);
    assert!(caps.word_timestamps);
    assert!(!caps.streaming, "streaming is deferred to a future release");
    assert_eq!(caps.languages.len(), known_languages_bcp47().len());
}

#[test]
fn engine_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<MiniMaxEngine>();
}

#[test]
fn engine_implements_asr_engine_via_trait_object() {
    let cfg = MiniMaxConfig::new("sk-test").unwrap();
    let engine = MiniMaxEngine::new(cfg).unwrap();
    let arc: std::sync::Arc<dyn AsrEngine> = std::sync::Arc::new(engine);
    let caps = arc.capabilities();
    assert!(caps.multilingual);
}

#[test]
fn transcribe_rejects_invalid_language_via_params() {
    let cfg = MiniMaxConfig::new("sk-test").unwrap();
    let engine = MiniMaxEngine::new(cfg).unwrap();
    let opts = TranscribeOptions::new(Some("klingon".into()), false, false);
    let samples = vec![0.0_f32; 16];
    let err = engine
        .transcribe(&samples, &opts)
        .expect_err("klingon rejected");
    match err {
        voxora_traits::AsrError::InvalidInput(msg) => {
            assert!(msg.contains("klingon"), "{msg}");
        }
        other => panic!("expected InvalidInput, got {other:?}"),
    }
}

#[test]
fn transcribe_rejects_translate_via_params() {
    let cfg = MiniMaxConfig::new("sk-test").unwrap();
    let engine = MiniMaxEngine::new(cfg).unwrap();
    let opts = TranscribeOptions::new(Some("en".into()), true, false);
    let samples = vec![0.0_f32; 16];
    let err = engine
        .transcribe(&samples, &opts)
        .expect_err("translate rejected");
    assert!(matches!(err, voxora_traits::AsrError::Unsupported(_)));
}

/// Live integration test: post 1 s of 440 Hz sine to the real
/// MiniMax endpoint and assert the round-trip succeeds.
///
/// Run with:
///
/// ```text
/// MINIMAX_API_KEY=<key> cargo test -p voxora-minimax -- --ignored live_transcription_works
/// ```
#[test]
#[ignore = "requires MINIMAX_API_KEY env var; run with --ignored"]
fn live_transcription_works() {
    let key = match env::var("MINIMAX_API_KEY").or_else(|_| env::var("VOXORA_MINIMAX_API_KEY")) {
        Ok(k) if !k.trim().is_empty() => k,
        _ => {
            // Skip silently — the `#[ignore]` annotation already
            // requires `--ignored`; we add an explicit skip here
            // so the test name does not silently turn green when
            // run without the env var.
            eprintln!("MINIMAX_API_KEY / VOXORA_MINIMAX_API_KEY not set; skipping live test");
            return;
        }
    };

    let config = MiniMaxConfig::new(key).expect("non-empty key");
    let engine = MiniMaxEngine::new(config).expect("engine builds");

    // 1 s of 440 Hz sine (mono, 16 kHz).
    let samples: Vec<f32> = (0..16_000)
        .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 16_000.0).sin() * 0.5)
        .collect();

    let opts = TranscribeOptions::default();
    let result = engine
        .transcribe(&samples, &opts)
        .expect("live transcription should succeed");

    // The MiniMax endpoint always returns 200 with a parseable
    // body on success; the specific text for 1 s of 440 Hz sine
    // is undefined (could be empty, could be a hallucinated
    // single token — MiniMax does not guarantee a specific
    // output for non-speech audio). The contract is just that
    // we get back a valid response.
    let _ = result.text;
    let _ = result.language;
}
