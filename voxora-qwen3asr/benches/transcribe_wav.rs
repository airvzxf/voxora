//! Real-Time Factor (RTF) of `QwenAsrEngine::transcribe`:
//! transcription time / audio duration (below 1.0 is faster than
//! real time). See `docs/BENCHMARKS.md`.
//!
//! Needs a model directory (with `tokenizer.json`, which
//! `QwenAsrEngine::from_hf` or `prepare_model_dir` writes) and a
//! speech clip, so both come from the environment and nothing is
//! measured without them:
//!
//! ```text
//! VOXORA_BENCH_QWEN_DIR=~/.cache/voxora/models/huggingface/Qwen/Qwen3-ASR-0.6B/main \
//! VOXORA_BENCH_WAV=speech-16k-mono.wav \
//!   cargo bench -p voxora-qwen3asr --bench transcribe_wav
//! ```

use std::path::PathBuf;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion};
use voxora_qwen3asr::QwenAsrEngine;
use voxora_traits::{AsrEngine, TranscribeOptions};

/// 16 kHz mono 16-bit PCM, as the engine expects.
fn read_wav(path: &PathBuf) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path).expect("open VOXORA_BENCH_WAV");
    let spec = reader.spec();
    assert!(
        spec.sample_rate == 16_000 && spec.channels == 1 && spec.bits_per_sample == 16,
        "VOXORA_BENCH_WAV must be 16 kHz mono 16-bit PCM, got {spec:?}"
    );
    reader
        .samples::<i16>()
        .map(|s| f32::from(s.expect("decode sample")) / f32::from(i16::MAX))
        .collect()
}

fn bench_rtf(c: &mut Criterion) {
    let (Some(dir), Some(wav)) = (
        std::env::var_os("VOXORA_BENCH_QWEN_DIR"),
        std::env::var_os("VOXORA_BENCH_WAV"),
    ) else {
        eprintln!("qwen3-asr RTF bench skipped: set VOXORA_BENCH_QWEN_DIR and VOXORA_BENCH_WAV");
        return;
    };
    let engine = QwenAsrEngine::load(&PathBuf::from(dir)).expect("load the Qwen3-ASR model");
    let audio = read_wav(&PathBuf::from(wav));
    #[allow(clippy::cast_precision_loss)]
    let seconds = audio.len() as f64 / 16_000.0;
    let opts = TranscribeOptions::default();

    let mut group = c.benchmark_group("qwen3asr");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(60));
    group.bench_function("transcribe_wav", |b| {
        b.iter(|| engine.transcribe(&audio, &opts).expect("transcribe"));
    });
    group.finish();

    // Criterion reports time per call; RTF needs it against the clip
    // length, from a median so one slow run does not skew it.
    let mut runs: Vec<f64> = (0..5)
        .map(|_| {
            let started = std::time::Instant::now();
            engine.transcribe(&audio, &opts).expect("transcribe");
            started.elapsed().as_secs_f64()
        })
        .collect();
    runs.sort_by(f64::total_cmp);
    let median = runs[runs.len() / 2];
    println!(
        "qwen3-asr RTF: {:.3} (median {median:.2} s of 5 runs for {seconds:.2} s of audio)",
        median / seconds
    );
}

criterion_group!(qwen3asr_benches, bench_rtf);
criterion_main!(qwen3asr_benches);
