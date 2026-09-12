//! End-to-end demo: read a WAV file with `hound`, downmix to
//! mono `f32`, and post it to the MiniMax `/v1/speech_to_text`
//! API.
//!
//! Run with:
//!
//! ```text
//! MINIMAX_API_KEY=<key> cargo run -p voxora-minimax --example transcribe_wav_minimax --release -- tests/fixtures/audio/sample1.wav
//! ```
//!
//! Requires `MINIMAX_API_KEY` (or `VOXORA_MINIMAX_API_KEY`)
//! to be set in the environment. The example exits with a clear
//! message if the env var is missing.

use voxora_minimax::{MiniMaxConfig, MiniMaxEngine};
use voxora_traits::{AsrEngine, TranscribeOptions};

/// Bit-depth-aware WAV decoder. Mirrors `voxora-cli/src/audio.rs`
/// but inlined here so the example stays self-contained.
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let audio_path = args
        .next()
        .ok_or("usage: transcribe_wav_minimax <audio.wav>")?;

    let key = std::env::var("MINIMAX_API_KEY")
        .or_else(|_| std::env::var("VOXORA_MINIMAX_API_KEY"))
        .map_err(|_| "MINIMAX_API_KEY (or VOXORA_MINIMAX_API_KEY) not set")?;
    if key.trim().is_empty() {
        return Err("MINIMAX_API_KEY is empty".into());
    }

    let config = MiniMaxConfig::new(key)?;
    let engine = MiniMaxEngine::new(config)?;

    let (mono, sample_rate) = decode_wav_to_mono_f32(&audio_path)?;

    eprintln!(
        "loaded {} ({} Hz, mono), {} samples ({:.2} s)",
        audio_path,
        sample_rate,
        mono.len(),
        mono.len() as f64 / sample_rate as f64,
    );

    let opts = TranscribeOptions::default();
    let result = engine.transcribe(&mono, &opts)?;

    println!("language: {}", result.language.as_deref().unwrap_or("?"));
    println!("text    : {}", result.text);
    Ok(())
}
