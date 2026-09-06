//! Library-API smoke for `voxora-engine`: wrap a [`MockAdapter`]
//! behind [`AnyEngine`], dispatch on `family()`, and call the
//! borrowed [`voxora_traits::AsrEngine`] synchronously.
//!
//! Where `bridge_demo` and `basic_transcribe` exercise the
//! end-to-end HF → engine → WAV flow, this example is the smallest
//! possible program that proves the adapter contract works. It
//! never touches Hugging Face or loads a real model, so it is
//! hermetic and offline — useful as a sanity check after a trait
//! refactor lands.
//!
//! Run with:
//!
//! ```text
//! cargo run --example adapter_dispatch -p voxora-engine
//! ```

use voxora_engine::{AnyEngine, BackendKind, EngineFamily, MockAdapter};
use voxora_traits::{AsrEngine, TranscribeOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        (EngineFamily::Whisper, BackendKind::Cpu),
        (EngineFamily::Qwen3Asr, BackendKind::Cuda),
    ];

    for (family, backend) in cases {
        let adapter = MockAdapter::new(family).with_backend(backend);
        let any = AnyEngine::new(adapter);

        let info = any.info();
        let caps = info.capabilities.clone();
        let samples = [0.0_f32; 8];
        let opts = TranscribeOptions::default();
        let result = any.transcribe(&samples, &opts)?;

        println!("family         : {family}");
        println!("backend        : {:?}", any.backend().kind);
        println!(
            "model_label    : {}",
            info.model_label.as_deref().unwrap_or("<none>")
        );
        println!("multilingual   : {}", caps.multilingual);
        println!("streaming      : {}", any.as_streaming_engine().is_some());
        println!("transcript     : {:?}", result.text);
        println!();
    }

    Ok(())
}
