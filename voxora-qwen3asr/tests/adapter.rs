//! Integration test: `QwenAsrAdapter` implements
//! `voxora_engine::EngineAdapter`.
//!
//! This is a compile-time contract test — there is no real
//! Qwen3-ASR model in CI, so we cannot exercise `transcribe`. The
//! goal is to lock the public surface so future refactors cannot
//! silently drop the `EngineAdapter` impl.

#![cfg(feature = "engine-adapter")]

#[test]
fn adapter_implements_engine_adapter() {
    fn assert_adapter<T: voxora_engine::EngineAdapter>() {}
    assert_adapter::<voxora_qwen3asr::QwenAsrAdapter>();
}

#[test]
fn adapter_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<voxora_qwen3asr::QwenAsrAdapter>();
}

#[test]
fn adapter_engine_accessor_reachable() {
    // Verify the `engine()` accessor exists and returns the
    // expected concrete type. We cannot construct a real
    // `QwenAsrEngine` here (no model directory in CI), so this is
    // a signature-only check.
    fn _assert_accessor(
        _adapter: &voxora_qwen3asr::QwenAsrAdapter,
    ) -> &voxora_qwen3asr::QwenAsrEngine {
        _adapter.engine()
    }
}
