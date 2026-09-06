//! Offline smoke bench: measure the cost of constructing a
//! [`voxora_traits::ModelCapabilities`] from defaults.
//!
//! Construction is intentionally trivial (a few bool fields + an
//! empty `Vec`); the bench exists so that:
//!
//! 1. `cargo bench --workspace --no-run` exercises the
//!    `voxora-engine` crate's bench target on every CI run,
//!    catching future compile drift in criterion or its
//!    workspace-level config.
//! 2. A regression in the trait's `Default` impl (e.g. a future
//!    `Vec::with_capacity(N)` accidental upcast) is loud.
//!
//! Runs in <1 ms even on CI; the wall-clock cost is dominated by
//! the harness's own measurement overhead.

use criterion::{criterion_group, criterion_main, Criterion};
use voxora_engine::ModelCapabilities;

fn bench_default(c: &mut Criterion) {
    c.bench_function("model_capabilities::default", |b| {
        b.iter(ModelCapabilities::default);
    });
}

fn bench_new_multilingual(c: &mut Criterion) {
    c.bench_function("model_capabilities::new_multilingual", |b| {
        b.iter(|| {
            ModelCapabilities::new(
                true,
                true,
                false,
                vec!["en".into(), "es".into(), "fr".into()],
            )
        });
    });
}

fn bench_unknown(c: &mut Criterion) {
    c.bench_function("model_capabilities::UNKNOWN", |b| {
        b.iter(|| ModelCapabilities::UNKNOWN.clone());
    });
}

criterion_group!(
    benches,
    bench_default,
    bench_new_multilingual,
    bench_unknown,
);
criterion_main!(benches);
