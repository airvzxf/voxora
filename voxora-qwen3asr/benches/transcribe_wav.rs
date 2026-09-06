//! Placeholder bench for the Real-Time Factor (RTF) measurement of
//! `voxora-qwen3asr::QwenAsrEngine::transcribe` against a 30-second
//! silence fixture.
//!
//! ## Status (EPIC #133, PR #56)
//!
//! A real RTF measurement requires the `Qwen/Qwen3-ASR-0.6B`
//! checkpoint (~1.7 GB). The download helpers in
//! `voxora-testkit::fixtures::real::resolve_real_fixture` are
//! being filled in by PR #59 (closes #133); until that lands,
//! the real RTF bench is `#[ignore]`-d and the harness only
//! verifies that the surrounding plumbing compiles cleanly.
//!
//! Run with:
//!
//! ```text
//! cargo bench -p voxora-qwen3asr --bench transcribe_wav -- --ignored
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
    c.bench_function("qwen3asr::rtf_30s_silence_stub", |b| {
        b.iter(|| {
            eprintln!(
                "voxora-qwen3asr RTF bench is a stub until PR #59 lands the \
                 ureq-based download in voxora-testkit (closes #133). The \
                 Qwen3-ASR-0.6B checkpoint is ~1.7 GB and needs hermetic \
                 download plumbing before RTF can be measured."
            );
        });
    });
}

criterion_group!(qwen3asr_benches, bench_rtf_30s_silence,);
criterion_main!(qwen3asr_benches);
