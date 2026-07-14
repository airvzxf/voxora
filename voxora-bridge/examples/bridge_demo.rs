//! End-to-end demo using the `voxora-bridge` umbrella crate.
//!
//! This is the canonical "Telora-style" flow:
//!
//! 1. Build a [`HuggingFaceSource`] with a cache directory.
//! 2. Resolve a model id (e.g. `ggerganov/whisper.cpp/ggml-tiny.bin`)
//!    from Hugging Face and load a [`WhisperEngine`] in one call.
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

    let mut reader = hound::WavReader::open(&audio_path)?;
    let spec = reader.spec();
    let samples = reader
        .samples::<i16>()
        .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
        .collect::<Result<Vec<_>, _>>()?;

    // Downmix to mono if needed; we do not resample, the model
    // expects 16 kHz mono.
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

#[cfg(not(feature = "whisper"))]
fn main() {
    eprintln!(
        "this example requires the `whisper` feature: cargo run --features whisper --example bridge_demo -- <hf-model-id> <wav>"
    );
    std::process::exit(2);
}
