//! End-to-end demo using the `voxora-bridge` umbrella crate.
//!
//! This is the canonical "Telora-style" flow:
//!
//! 1. Build a [`HuggingFaceSource`] with a cache directory.
//! 2. Resolve a model id (e.g. `ggerganov/whisper.cpp/ggml-tiny.bin`)
//!    from Hugging Face and load a `WhisperEngine` in one call.
//! 3. Read a WAV file with `hound`, downmix to mono `f32` at 16 kHz,
//!    and call [`AsrEngine::transcribe`].
//! 4. Print the transcription to stdout.
//!
//! Run with:
//!
//! ```text
//! cargo run --example bridge_demo -- \
//!     ggerganov/whisper.cpp/ggml-tiny.bin samples/jfk.wav
//! ```
//!
//! The first run downloads ~75 MB of model weights into the cache; the
//! second run resolves from disk in milliseconds.

#[cfg(feature = "whisper")]
use voxora_bridge::{AsrEngine, HuggingFaceSource, TranscribeOptions, WhisperEngine};

/// Bit-depth-aware WAV decoder. Mirrors `voxora-cli/src/audio.rs`
/// but inlined here so the example stays self-contained — examples
/// in the workspace do not lift audio.rs across crates.
#[cfg(feature = "whisper")]
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

#[cfg(feature = "whisper")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let model_id = args
        .next()
        .ok_or("usage: bridge_demo <hf-model-id> <audio.wav>")?;
    let audio_path = args
        .next()
        .ok_or("usage: bridge_demo <hf-model-id> <audio.wav>")?;

    let source = HuggingFaceSource::new()?;
    let engine = WhisperEngine::from_hf(&source, &model_id, &Default::default()).await?;

    let (mono, sample_rate) = decode_wav_to_mono_f32(&audio_path)?;

    eprintln!(
        "loaded {} ({} Hz, mono), {} samples ({:.2} s)",
        model_id,
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

#[cfg(not(feature = "whisper"))]
fn main() {
    eprintln!(
        "this example requires the `whisper` feature: cargo run --features whisper --example bridge_demo -- <hf-model-id> <wav>"
    );
    std::process::exit(2);
}
