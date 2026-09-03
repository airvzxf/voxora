//! Integration test: `WhisperAdapter` implements
//! `voxora_engine::EngineAdapter`.
//!
//! This is a compile-time contract test — there is no real
//! `ggml-tiny.bin` model in CI, so we cannot exercise
//! `transcribe`. The goal is to lock the public surface so future
//! refactors cannot silently drop the `EngineAdapter` impl.

#![cfg(feature = "engine-adapter")]

#[test]
fn adapter_implements_engine_adapter() {
    fn assert_adapter<T: voxora_engine::EngineAdapter>() {}
    assert_adapter::<voxora_whisper::WhisperAdapter>();
}

#[test]
fn adapter_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<voxora_whisper::WhisperAdapter>();
}

#[test]
fn adapter_engine_accessor_reachable() {
    // Verify the `engine()` accessor exists and returns the
    // expected concrete type. We cannot construct a real
    // `WhisperEngine` here (no model file in CI), so this is a
    // signature-only check.
    fn _assert_accessor(
        _adapter: &voxora_whisper::WhisperAdapter,
    ) -> &voxora_whisper::WhisperEngine {
        _adapter.engine()
    }
}
