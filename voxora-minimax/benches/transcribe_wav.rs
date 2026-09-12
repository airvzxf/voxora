//! Placeholder bench for the MiniMax hosted ASR engine.
//!
//! ## Status
//!
//! A real RTF measurement requires a `MINIMAX_API_KEY` env var
//! and a network round-trip; the CI lane is compile-only via
//! `cargo bench --workspace --no-run`, exactly the way
//! `voxora-whisper` and `voxora-qwen3asr` benches are scoped
//! (closes EPIC #133 #56 + follow-up).
//!
//! The harness prints a notice and returns immediately when
//! `VOXORA_SKIP_STUB_BENCHES` is unset, and is a true no-op when
//! it is set.
//!
//! Run with:
//!
//! ```text
//! cargo bench -p voxora-minimax --bench transcribe_wav
//! ```

use criterion::{criterion_group, criterion_main, Criterion};

/// Stub: would measure `transcription_time / audio_duration`
/// against the live MiniMax API. Today it prints a notice and
/// returns immediately; the harness still records a single sample
/// so the `Criterion::bench_function` surface stays exercised.
fn bench_rtf_30s_silence(c: &mut Criterion) {
    c.bench_function("minimax::rtf_30s_silence_stub", |b| {
        b.iter(|| {
            if std::env::var_os("VOXORA_SKIP_STUB_BENCHES").is_some() {
                return;
            }
            eprintln!(
                "voxora-minimax RTF bench is a stub until the live MiniMax API call \
                 is wired into voxora-testkit (closes EPIC #133)."
            );
        });
    });
}

criterion_group!(minimax_benches, bench_rtf_30s_silence,);
criterion_main!(minimax_benches);
