//! Mappings between `config.json` `torch_dtype` strings, GGUF
//! filename suffixes, and the [`voxora_core::Quantization`] enum.
//!
//! Hugging Face never publishes a normalised "quantization" field;
//! the dtype is implicit in `config.json` for safetensors and in the
//! filename for GGUF (whisper.cpp) repos. This module is the single
//! place that decides which [`voxora_core::Quantization`] value
//! travels up to the caller in [`voxora_core::ModelDir`].

use voxora_core::Quantization;

/// Map a `torch_dtype` value from `config.json` to a
/// [`Quantization`]. Unknown values fall back to [`Quantization::F16`]
/// (the most common dtype on the Hub) and do **not** error: missing
/// dtype metadata is a routine occurrence.
pub(crate) fn from_torch_dtype(value: &str) -> Quantization {
    match value.trim().to_ascii_lowercase().as_str() {
        "bfloat16" | "bf16" => Quantization::Bf16,
        "float16" | "half" | "fp16" | "f16" => Quantization::F16,
        "float32" | "float" | "fp32" | "f32" => Quantization::F32,
        _ => Quantization::F16,
    }
}

/// Map a GGUF-style filename (e.g. `ggml-base.bin.q4_K_M`) to a
/// [`Quantization`]. Non-quantized filenames map to [`Quantization::F16`].
pub(crate) fn from_gguf_filename(name: &str) -> Quantization {
    let lower = name.to_ascii_lowercase();
    if lower.contains("q4_k") {
        Quantization::Q4K
    } else if lower.contains("q8_0") || lower.contains("q8.0") {
        Quantization::Q8_0
    } else if lower.contains("q5_k") || lower.contains("q6_k") {
        // We don't expose Q5K/Q6K in voxora-core today, so collapse
        // them to the closest cousin: Q4K. Callers needing the exact
        // variant should use a more specific ModelSource impl.
        Quantization::Q4K
    } else {
        Quantization::F16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torch_dtype_bf16() {
        assert_eq!(from_torch_dtype("bfloat16"), Quantization::Bf16);
        assert_eq!(from_torch_dtype("BF16"), Quantization::Bf16);
        assert_eq!(from_torch_dtype(" bf16 "), Quantization::Bf16);
    }

    #[test]
    fn torch_dtype_f16() {
        assert_eq!(from_torch_dtype("float16"), Quantization::F16);
        assert_eq!(from_torch_dtype("half"), Quantization::F16);
        assert_eq!(from_torch_dtype("fp16"), Quantization::F16);
        assert_eq!(from_torch_dtype("f16"), Quantization::F16);
    }

    #[test]
    fn torch_dtype_f32() {
        assert_eq!(from_torch_dtype("float32"), Quantization::F32);
        assert_eq!(from_torch_dtype("float"), Quantization::F32);
        assert_eq!(from_torch_dtype("fp32"), Quantization::F32);
    }

    #[test]
    fn torch_dtype_unknown_falls_back_to_f16() {
        assert_eq!(from_torch_dtype(""), Quantization::F16);
        assert_eq!(from_torch_dtype("int8"), Quantization::F16);
        assert_eq!(from_torch_dtype("????"), Quantization::F16);
    }

    #[test]
    fn gguf_q4k_variants() {
        assert_eq!(
            from_gguf_filename("ggml-base.bin.q4_K_M"),
            Quantization::Q4K
        );
        assert_eq!(
            from_gguf_filename("ggml-small.Q4_K_S.bin"),
            Quantization::Q4K
        );
    }

    #[test]
    fn gguf_q8_0_variants() {
        assert_eq!(from_gguf_filename("ggml-base.bin.q8_0"), Quantization::Q8_0);
        assert_eq!(from_gguf_filename("ggml-base.Q8.0.bin"), Quantization::Q8_0);
    }

    #[test]
    fn gguf_q5_q6_collapse_to_q4k() {
        assert_eq!(
            from_gguf_filename("ggml-base.bin.q5_K_M"),
            Quantization::Q4K
        );
        assert_eq!(from_gguf_filename("ggml-base.bin.q6_K"), Quantization::Q4K);
    }

    #[test]
    fn gguf_unquantized_is_f16() {
        assert_eq!(from_gguf_filename("ggml-base.bin"), Quantization::F16);
        assert_eq!(from_gguf_filename("model.gguf"), Quantization::F16);
    }
}
