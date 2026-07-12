//! Mapping from `qwen3_asr::AsrError` to [`voxora_core::AsrError`].
//!
//! The upstream error type is a thin `thiserror` enum around
//! `anyhow::Error`. voxora preserves the chain (we wrap the inner
//! message in the rendered string) but collapses all three variants
//! into a single `voxora_core::AsrError::Inference` so the public API
//! stays coarse-grained. Promoting specific variants (e.g. audio
//! decode → `AudioIo`) would require upstream to expose a stable
//! `Path`, which it does not.

use voxora_core::AsrError;

/// Convert a `qwen3_asr::AsrError` into a [`voxora_core::AsrError`].
///
/// The inner `anyhow::Error` is rendered to its display string and
/// re-wrapped under `Inference` so the chain survives one level deep.
/// Callers wanting the full chain should reach into the upstream error
/// directly; voxora only sees the message.
pub fn map_qwen_error(e: qwen3_asr::AsrError) -> AsrError {
    AsrError::Inference(format!("qwen3-asr: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_load_collapses_to_inference_with_chain_message() {
        // We can only construct the error by triggering a real failure
        // path, which is not available offline. Smoke-test the mapping
        // by reaching into anyhow directly: a synthetic anyhow error
        // should produce a non-empty Inference message.
        let inner = anyhow::anyhow!("weights missing: model.safetensors");
        let qwen_err = qwen3_asr::AsrError::ModelLoad(inner);
        let mapped = map_qwen_error(qwen_err);
        match mapped {
            AsrError::Inference(msg) => {
                assert!(
                    msg.contains("qwen3-asr"),
                    "message should retain the upstream prefix: {msg}"
                );
                assert!(
                    msg.contains("model.safetensors"),
                    "message should retain the inner chain: {msg}"
                );
            }
            other => panic!("expected Inference, got {other:?}"),
        }
    }

    #[test]
    fn audio_decode_collapses_to_inference() {
        let inner = anyhow::anyhow!("WAV header truncated");
        let mapped = map_qwen_error(qwen3_asr::AsrError::AudioDecode(inner));
        match mapped {
            AsrError::Inference(msg) => {
                assert!(msg.contains("WAV header truncated"), "{msg}");
            }
            other => panic!("expected Inference, got {other:?}"),
        }
    }

    #[test]
    fn inference_collapses_to_inference() {
        let inner = anyhow::anyhow!("shape mismatch");
        let mapped = map_qwen_error(qwen3_asr::AsrError::Inference(inner));
        match mapped {
            AsrError::Inference(msg) => {
                assert!(msg.contains("shape mismatch"), "{msg}");
            }
            other => panic!("expected Inference, got {other:?}"),
        }
    }
}
