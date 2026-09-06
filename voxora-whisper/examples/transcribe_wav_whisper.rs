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
use voxora_traits::{AsrEngine, TranscribeOptions};
#[cfg(feature = "hf")]
use voxora_whisper::WhisperEngine;

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
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let model_path = args
        .next()
        .ok_or("usage: transcribe_wav_whisper <model.bin> <audio.wav>")?;
    let audio_path = args
        .next()
        .ok_or("usage: transcribe_wav_whisper <model.bin> <audio.wav>")?;

    let engine = WhisperEngine::load(std::path::Path::new(&model_path))?;

    let (mono, sample_rate) = decode_wav_to_mono_f32(&audio_path)?;

    eprintln!(
        "loaded {} ({} Hz, mono), {} samples ({:.2} s)",
        model_path,
        sample_rate,
        mono.len(),
        mono.len() as f64 / sample_rate as f64,
    );

    let opts = TranscribeOptions::new(Some("en".into()), false, true);
    let result = engine.transcribe(&mono, &opts)?;

    println!("{}", result.text);
    if !result.segments.is_empty() {
        eprintln!("---");
        for seg in &result.segments {
            // Use the WAV's declared sample rate, not a hard-coded
            // 16 kHz. Engines internally resample to their native
            // rate; `start_sample` / `end_sample` are at the WAV's
            // rate, so the divide should match the source rate.
            let start = seg.start_sample as f64 / sample_rate as f64;
            let end = seg.end_sample as f64 / sample_rate as f64;
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
