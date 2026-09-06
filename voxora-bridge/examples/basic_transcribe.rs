//! Minimal "library API" smoke for `voxora-bridge`: load a
//! [`WhisperEngine`] from a Hugging Face model id and call
//! [`AsrEngine::transcribe`] on a synthetic sample buffer.
//!
//! Where `bridge_demo` is the *binary* flow (CLI args + WAV decode),
//! this example is the *library* flow — the smallest possible
//! program a downstream crate can copy to start a transcribe
//! pipeline. It does not touch `hound`; the audio buffer is a baked
//! silent slice at the engine's native 16 kHz rate.
//!
//! Run with:
//!
//! ```text
//! cargo run --example basic_transcribe -p voxora-bridge --features voxora-bridge/whisper -- \
//!     ggerganov/whisper.cpp/ggml-tiny.bin
//! ```
//!
//! The first run downloads ~75 MB of model weights into the cache;
//! the second run resolves from disk in milliseconds.

#[cfg(feature = "whisper")]
use voxora_bridge::{AsrEngine, HuggingFaceSource, TranscribeOptions, WhisperEngine};

#[cfg(feature = "whisper")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model_id = std::env::args()
        .nth(1)
        .ok_or("usage: basic_transcribe <hf-model-id>")?;

    let source = HuggingFaceSource::new()?;
    let engine = WhisperEngine::from_hf(&source, &model_id, &Default::default()).await?;

    // 1 second of silence at 16 kHz. The library API does not care
    // where the samples come from — `hound`, a ringbuffer, an audio
    // capture thread — only that they arrive as `&[f32]` at a rate
    // the engine recognises. Whisper resamples internally to its
    // native rate regardless.
    let samples = vec![0.0_f32; 16_000];

    let opts = TranscribeOptions::new(Some("en".into()), false, false);
    let result = engine.transcribe(&samples, &opts)?;

    println!("text: {:?}", result.text);
    println!("segments: {}", result.segments.len());
    if let Some(model_type) = engine.model_type() {
        println!("model_type: {model_type}");
    }

    Ok(())
}

#[cfg(not(feature = "whisper"))]
fn main() {
    eprintln!(
        "this example requires the `whisper` feature: cargo run --example basic_transcribe -p voxora-bridge --features voxora-bridge/whisper -- <hf-model-id>"
    );
    std::process::exit(2);
}
