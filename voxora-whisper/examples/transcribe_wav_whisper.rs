//! End-to-end demo: load a Whisper GGML model, read a WAV file with
//! `hound`, downmix to mono `f32` at 16 kHz, and print the
//! transcription to stdout.
//!
//! Run with:
//!
//! ```text
//! cargo run -p voxora-whisper --features hf --example transcribe_wav_whisper -- \
//!     models/ggml-tiny.bin samples/jfk.wav
//! ```
//!
//! Requires the `hf` feature for `from_hf` (not used here directly,
//! but the example lives behind the same feature to keep the
//! minimal-cpu build slim — drop the `required-features` line if you
//! want the example available without `hf`).

#[cfg(feature = "hf")]
use voxora_core::{AsrEngine, TranscribeOptions};
#[cfg(feature = "hf")]
use voxora_whisper::WhisperEngine;

#[cfg(feature = "hf")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let model_path = args
        .next()
        .ok_or("usage: transcribe_wav_whisper <model.bin> <audio.wav>")?;
    let audio_path = args
        .next()
        .ok_or("usage: transcribe_wav_whisper <model.bin> <audio.wav>")?;

    let engine = WhisperEngine::load(std::path::Path::new(&model_path))?;

    let mut reader = hound::WavReader::open(&audio_path)?;
    let spec = reader.spec();
    let samples = reader
        .samples::<i16>()
        .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
        .collect::<Result<Vec<_>, _>>()?;

    // Downmix stereo -> mono if needed. We do not resample; the model
    // is expected to receive audio at its native rate (typically 16 kHz
    // for whisper.cpp's smallest models).
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
        model_path,
        spec.sample_rate,
        spec.channels,
        mono.len(),
        mono.len() as f64 / spec.sample_rate as f64,
    );

    let opts = TranscribeOptions::new(Some("en".into()), false, true);
    let result = engine.transcribe(&mono, &opts)?;

    println!("{}", result.text);
    if !result.segments.is_empty() {
        eprintln!("---");
        for seg in &result.segments {
            let start = seg.start_sample as f64 / 16_000.0;
            let end = seg.end_sample as f64 / 16_000.0;
            eprintln!("[{start:7.2}s - {end:7.2}s] {}", seg.text);
        }
    }

    Ok(())
}

#[cfg(not(feature = "hf"))]
fn main() {
    eprintln!(
        "this example requires the `hf` feature: cargo run -p voxora-whisper --features hf --example transcribe_wav_whisper -- <model> <wav>"
    );
    std::process::exit(2);
}
