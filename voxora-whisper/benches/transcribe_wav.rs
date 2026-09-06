//! Placeholder bench for the Real-Time Factor (RTF) measurement of
//! `voxora-whisper::WhisperEngine::transcribe` against a 30-second
//! silence fixture.
//!
//! ## Status (EPIC #133, PR #56)
//!
//! A real RTF measurement requires a hermetic model-download
//! story: the canonical `ggml-tiny.bin` is ~75 MB and the engine
//! refuses to load without it. The download helpers in
//! `voxora-testkit::fixtures::real::resolve_real_fixture` are
//! being filled in by PR #59 (closes #133); until that lands,
//! the real RTF bench is `#[ignore]`-d and the harness only
//! verifies that the surrounding plumbing compiles cleanly.
//!
//! Run with:
//!
//! ```text
//! cargo bench -p voxora-whisper --bench transcribe_wav -- --ignored
//! ```
//!
//! The bench target is also exercised by `cargo bench --workspace
//! --no-run` on every PR so a future compile regression is loud.

use criterion::{criterion_group, criterion_main, Criterion};

/// Stub: would measure `transcription_time / audio_duration`.
/// Today it prints a notice and returns immediately; the harness
/// still records a single sample so the `Criterion::bench_function`
/// surface stays exercised.
fn bench_rtf_30s_silence(c: &mut Criterion) {
    c.bench_function("whisper::rtf_30s_silence_stub", |b| {
        b.iter(|| {
            eprintln!(
                "voxora-whisper RTF bench is a stub until PR #59 lands the \
                 ureq-based download in voxora-testkit (closes #133)."
            );
        });
    });
}

criterion_group!(whisper_benches, bench_rtf_30s_silence,);
criterion_main!(whisper_benches);
