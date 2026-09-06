//! Placeholder bench for the Real-Time Factor (RTF) measurement of
//! `voxora-qwen3asr::QwenAsrEngine::transcribe` against a 30-second
//! silence fixture.
//!
//! ## Status (EPIC #133, PR #56 + follow-up #137)
//!
//! A real RTF measurement requires the `Qwen/Qwen3-ASR-0.6B`
//! checkpoint (~1.7 GB). The download helpers in
//! `voxora-testkit::fixtures::real::resolve_real_fixture` landed
//! in PR #135 (closes #59), but the model-weight download story
//! still requires the operator to pre-populate the cache or run
//! a nightly workflow, so the real RTF bench stays a stub. The
//! harness only verifies that the surrounding plumbing compiles
//! cleanly.
//!
//! Criterion 0.8 has no `#[ignore]` support; this bench runs
//! unconditionally. To silence it locally, set
//! `VOXORA_SKIP_STUB_BENCHES=1` or pass
//! `cargo bench … -- --bench-filter <other-bench-name>`.
//!
//! Run with:
//!
//! ```text
//! cargo bench -p voxora-qwen3asr --bench transcribe_wav
//! ```
//!
//! The bench target is also exercised by `cargo bench --workspace
//! --no-run` on every PR so a future compile regression is loud.

use criterion::{criterion_group, criterion_main, Criterion};

/// Stub: would measure `transcription_time / audio_duration`.
/// Today it prints a notice and returns immediately when
/// `VOXORA_SKIP_STUB_BENCHES` is unset, and is a true no-op when
/// it is set; the harness still records a single sample so the
/// `Criterion::bench_function` surface stays exercised.
fn bench_rtf_30s_silence(c: &mut Criterion) {
    c.bench_function("qwen3asr::rtf_30s_silence_stub", |b| {
        b.iter(|| {
            if std::env::var_os("VOXORA_SKIP_STUB_BENCHES").is_some() {
                return;
            }
            eprintln!(
                "voxora-qwen3asr RTF bench is a stub until the model-weight \
                 download story is wired into voxora-testkit (closes #133). \
                 The Qwen3-ASR-0.6B checkpoint is ~1.7 GB and needs hermetic \
                 download plumbing before RTF can be measured."
            );
        });
    });
}

criterion_group!(qwen3asr_benches, bench_rtf_30s_silence,);
criterion_main!(qwen3asr_benches);
