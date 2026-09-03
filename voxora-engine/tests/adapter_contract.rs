//! Integration test: the adapter contract is the same regardless of
//! which engine family it wraps.

use voxora_traits::{AsrEngine, TranscribeOptions};
use voxora_engine::{
    AnyEngine, BackendDescriptor, BackendKind, EngineAdapter, EngineFamily, EngineInfo,
    MockAdapter, ModelCapabilities,
};

#[test]
fn whisper_mock_satisfies_adapter_contract() {
    let m = MockAdapter::new(EngineFamily::Whisper);
    let adapter: &dyn EngineAdapter = &m;
    assert_eq!(adapter.family(), EngineFamily::Whisper);
    assert_eq!(adapter.backend().kind, BackendKind::Cpu);
    let info: EngineInfo = adapter.info();
    assert_eq!(info.family, EngineFamily::Whisper);
}

#[test]
fn qwen_mock_satisfies_adapter_contract() {
    let m = MockAdapter::new(EngineFamily::Qwen3Asr);
    let adapter: &dyn EngineAdapter = &m;
    assert_eq!(adapter.family(), EngineFamily::Qwen3Asr);
}

#[test]
fn any_engine_preserves_capabilities() {
    let m = MockAdapter::new(EngineFamily::Whisper).with_capabilities(ModelCapabilities::new(
        false,
        false,
        false,
        vec!["en".into()],
    ));
    let any = AnyEngine::new(m);
    let caps = any.capabilities();
    assert!(!caps.multilingual);
    assert_eq!(caps.languages, vec!["en".to_string()]);
}

#[test]
fn any_engine_clone_shares_inner_state() {
    let any = AnyEngine::new(MockAdapter::new(EngineFamily::Whisper));
    let clone = any.clone();
    let r1 = any
        .transcribe(&[0.0_f32; 2], &TranscribeOptions::default())
        .expect("ok");
    let r2 = clone
        .transcribe(&[0.0_f32; 2], &TranscribeOptions::default())
        .expect("ok");
    assert_eq!(r1.text, r2.text);
}

#[test]
fn adapter_info_is_clonable() {
    let m = MockAdapter::new(EngineFamily::Qwen3Asr);
    let info = m.info();
    let cloned = info.clone();
    assert_eq!(info, cloned);
}

#[test]
fn backend_descriptor_defaults_to_cpu() {
    let d = BackendDescriptor::CPU;
    assert_eq!(d.kind, BackendKind::Cpu);
}
