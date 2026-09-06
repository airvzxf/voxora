//! End-to-end cost of [`Registry::resolve`] against the
//! bench-local [`InMemorySource`] mock (see `in_memory_source.rs`).
//!
//! Criterion 0.8 has no `#[ignore]` support; this bench runs
//! unconditionally. To silence it locally, set
//! `VOXORA_SKIP_STUB_BENCHES=1` or pass
//! `cargo bench … -- --bench-filter <other-bench-name>`. The
//! bench body short-circuits on that env var so the per-iteration
//! work is skipped.
//!
//! Run with:
//!
//! ```text
//! cargo bench -p voxora-registry --bench resolve
//! ```
//!
//! The harness is also exercised by `cargo bench --workspace
//! --no-run` on every PR (closes #56), so a future compile
//! regression in the bench target or its deps is caught at PR
//! time.

use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use voxora_engine::EngineFamily;
use voxora_registry::{builtin_whisper_descriptor, EngineDescriptor, ModelId, Registry};
use voxora_traits::{ModelCapabilities, ResolveOptions};

mod in_memory_source;
use in_memory_source::InMemorySource;

fn bench_resolve(c: &mut Criterion) {
    let source = Arc::new(InMemorySource::new());
    let descriptor: EngineDescriptor = builtin_whisper_descriptor();
    let _ = descriptor; // silence unused; the lookup below uses the same name
    let registry = Registry::new(source).register(EngineDescriptor::new(
        EngineFamily::Whisper,
        "whisper",
        |id| id.repo.starts_with("ggerganov/whisper.cpp"),
        ModelCapabilities::UNKNOWN,
    ));

    let id = ModelId::parse("ggerganov/whisper.cpp/ggml-tiny.bin").expect("valid id");

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let opts = ResolveOptions::default();

    c.bench_function("registry::resolve_whisper_tiny", |b| {
        b.iter(|| {
            if std::env::var_os("VOXORA_SKIP_STUB_BENCHES").is_some() {
                return;
            }
            let resolved = rt.block_on(async { registry.resolve(&id, &opts).await });
            assert!(resolved.is_ok(), "resolve must succeed");
        });
    });
}

criterion_group!(
    name = resolve_benches;
    config = Criterion::default().sample_size(20);
    targets = bench_resolve,
);
criterion_main!(resolve_benches);
