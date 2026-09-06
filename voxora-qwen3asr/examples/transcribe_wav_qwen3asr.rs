//! End-to-end demo: resolve a Qwen3-ASR model from Hugging Face,
//! load it, read a WAV file with `hound`, downmix to mono `f32` at
//! 16 kHz, and print the transcription to stdout.
//!
//! Run with:
//!
//! ```text
//! cargo run -p voxora-qwen3asr --features hf --example transcribe_wav_qwen3asr --release -- \
//!     Qwen/Qwen3-ASR-0.6B tests/fixtures/audio/sample1.wav
//! ```
//!
//! The model is resolved via `voxora-hf` and cached under
//! `$XDG_CACHE_HOME/voxora/models/huggingface/` (subsequent runs are
//! instant).
//!
//! Requires the `hf` feature for `from_hf` (not used here directly,
//! but the example lives behind the same feature to keep the
//! minimal-cpu build slim — drop the `required-features` line if you
//! want the example available without `hf`).

#[cfg(feature = "hf")]
use voxora_hf::HuggingFaceSource;
#[cfg(feature = "hf")]
use voxora_qwen3asr::QwenAsrEngine;
#[cfg(feature = "hf")]
use voxora_traits::{AsrEngine, TranscribeOptions};

/// Bit-depth-aware WAV decoder. Mirrors `voxora-cli/src/audio.rs`
/// but inlined here so the example stays self-contained.
#[cfg(feature = "hf")]
fn decode_wav_to_mono_f32(path: &str) -> Result<(Vec<f32>, u32), Box<dyn std::error::Error>> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    let ch = spec.channels as usize;
    let mut mono = Vec::new();
    match (spec.sample_format, spec.bits_per_sample) {
        (hound::SampleFormat::Int, 16) => {
            let mut iter = reader.samples::<i16>();
            loop {
                let mut sum: i64 = 0;
                let mut got = 0;
                for _ in 0..ch {
                    if let Some(Ok(v)) = iter.next() {
                        sum += v as i64;
                        got += 1;
                    } else {
                        break;
                    }
                }
                if got == 0 {
                    break;
                }
                mono.push(((sum / got as i64) as f32) / 32_768.0);
            }
        }
        (hound::SampleFormat::Int, 24) => {
            let mut iter = reader.samples::<i32>();
            loop {
                let mut sum: i64 = 0;
                let mut got = 0;
                for _ in 0..ch {
                    if let Some(Ok(v)) = iter.next() {
                        sum += v as i64;
                        got += 1;
                    } else {
                        break;
                    }
                }
                if got == 0 {
                    break;
                }
                mono.push(((sum / got as i64) as f32) / 8_388_608.0);
            }
        }
        (hound::SampleFormat::Int, 32) => {
            let mut iter = reader.samples::<i32>();
            loop {
                let mut sum: i64 = 0;
                let mut got = 0;
                for _ in 0..ch {
                    if let Some(Ok(v)) = iter.next() {
                        sum += v as i64;
                        got += 1;
                    } else {
                        break;
                    }
                }
                if got == 0 {
                    break;
                }
                mono.push(((sum / got as i64) as f32) / 2_147_483_648.0);
            }
        }
        (hound::SampleFormat::Float, 32) => {
            let mut iter = reader.samples::<f32>();
            loop {
                let mut sum: f32 = 0.0;
                let mut got = 0;
                for _ in 0..ch {
                    if let Some(Ok(v)) = iter.next() {
                        sum += v;
                        got += 1;
                    } else {
                        break;
                    }
                }
                if got == 0 {
                    break;
                }
                mono.push(sum / got as f32);
            }
        }
        (fmt, bits) => {
            return Err(format!("unsupported WAV: format={fmt:?} bits={bits}").into());
        }
    }
    Ok((mono, spec.sample_rate))
}

#[cfg(feature = "hf")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let model_id = args
        .next()
        .ok_or("usage: transcribe_wav_qwen3asr <hf-model-id> <audio.wav>")?;
    let audio_path = args
        .next()
        .ok_or("usage: transcribe_wav_qwen3asr <hf-model-id> <audio.wav>")?;

    let source = HuggingFaceSource::new()?;
    let engine = QwenAsrEngine::from_hf(
        &source,
        &model_id,
        &voxora_traits::ResolveOptions::default(),
    )
    .await?;

    let (mono, sample_rate) = decode_wav_to_mono_f32(&audio_path)?;

    eprintln!(
        "loaded {} ({} Hz, mono), {} samples ({:.2} s)",
        model_id,
        sample_rate,
        mono.len(),
        mono.len() as f64 / sample_rate as f64,
    );

    let opts = TranscribeOptions::new(Some("english".into()), false, false);
    let result = engine.transcribe(&mono, &opts)?;

    println!("language: {}", result.language.as_deref().unwrap_or("?"));
    println!("text    : {}", result.text);
    Ok(())
}

#[cfg(not(feature = "hf"))]
fn main() {
    eprintln!(
        "this example requires the `hf` feature: cargo run -p voxora-qwen3asr --features hf --example transcribe_wav_qwen3asr -- <model> <wav>"
    );
    std::process::exit(2);
}
