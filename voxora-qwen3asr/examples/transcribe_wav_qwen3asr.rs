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
use voxora_core::{AsrEngine, TranscribeOptions};
#[cfg(feature = "hf")]
use voxora_hf::HuggingFaceSource;
#[cfg(feature = "hf")]
use voxora_qwen3asr::QwenAsrEngine;

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
    let engine =
        QwenAsrEngine::from_hf(&source, &model_id, &voxora_core::ResolveOptions::default()).await?;

    let mut reader = hound::WavReader::open(&audio_path)?;
    let spec = reader.spec();
    let samples = reader
        .samples::<i16>()
        .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
        .collect::<Result<Vec<_>, _>>()?;

    // Downmix stereo -> mono if needed. We do not resample; the model
    // is expected to receive audio at its native rate (typically
    // 16 kHz for Qwen3-ASR's shortest path).
    let mono: Vec<f32> = match spec.channels {
        1 => samples,
        n => {
            let ch = n as usize;
            samples
                .chunks(ch)
                .map(|frame| frame.iter().sum::<f32>() / ch as f32)
                .collect()
        }
    };

    eprintln!(
        "loaded {} ({} Hz, {} ch), {} mono samples ({:.2} s)",
        model_id,
        spec.sample_rate,
        spec.channels,
        mono.len(),
        mono.len() as f64 / spec.sample_rate as f64,
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
