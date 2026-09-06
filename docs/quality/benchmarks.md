# Benchmarks (EPIC #133, closes #56)

The voxora workspace carries a small [criterion] benchmark suite.
It is **compile-only on every PR** (`cargo bench --workspace
--no-run`); the runtime benches stay manual because the
engine-level RTF measurements require model weights that are
too heavy for a per-PR lane.

## Layout

| Crate | Bench file | Status | What it measures |
|---|---|---|---|
| `voxora-engine` | `benches/model_capabilities.rs` | always runs | cost of `ModelCapabilities::default` / `new` / `UNKNOWN.clone()` |
| `voxora-registry` | `benches/resolve.rs` | `#[ignore]`-d | `Registry::resolve` against `InMemorySource` |
| `voxora-whisper` | `benches/transcribe_wav.rs` | `#[ignore]`-d stub | RTF (`transcription_time / audio_duration`) — blocked on PR #59 |
| `voxora-qwen3asr` | `benches/transcribe_wav.rs` | `#[ignore]`-d stub | RTF — blocked on PR #59 |

The two stub benches are placeholders so the `cargo bench
--workspace --no-run` job compiles every crate's bench target.
They print a notice at runtime and exit early — they do not
fake a number.

## Running

```bash
# Compile everything (mirrors the CI `bench` job).
cargo bench --workspace --no-run

# Run the offline smoke benches that don't need model weights.
cargo bench -p voxora-engine --bench model_capabilities

# Run the `#[ignore]`-d registry bench (needs tokio runtime; uses
# the in-memory mock source so no network).
cargo bench -p voxora-registry --bench resolve -- --ignored
```

The two engine-level RTF benches are intentionally not runnable
today. PR #59 (closes #133) lands the `ureq`-based fixture
download in `voxora-testkit::fixtures::real::resolve_real_fixture`,
which unblocks the real RTF benches as a follow-up. Until then
the stub files keep the bench target compilable but return
immediately.

## Reading the output

criterion prints a table with the following columns:

- **time**: median wall-clock time per iteration, in nanoseconds.
- **thrpt**: throughput (iterations per second); mostly informational.
- **change**: vs. the previous run on this machine. criterion
  detects regressions when the new median sits outside the
  previous run's interquartile noise envelope.

For the engine-level benches that ship later, **RTF** is
defined as:

```text
RTF = wall_clock_transcription_seconds / audio_duration_seconds
```

- `RTF < 1.0` — the engine transcribes **faster than realtime**.
- `RTF = 1.0` — the engine transcribes **exactly at realtime**.
- `RTF > 1.0` — the engine transcribes **slower than realtime**.

A regression that pushes RTF above 1.0 on a CPU-class runner
is the canonical signal that a new release needs optimisation
work before tagging it.

## CI integration

The `bench` job in `.github/workflows/ci.yml` runs
`cargo bench --workspace --no-run` on `RUSTUP_TOOLCHAIN=1.88`.
Compile-only; wall-clock is ~1 minute cold, ~10 seconds warm.
Failures block PR merge via the `protect-main` ruleset.

For nightly / on-demand RTF measurement, follow up with the
[nightly quality workflow][nightly] (the `quality-nightly.yml`
workflow added by PR #54 covers the fuzz lane; a future
`bench-nightly` lane would mirror it for the RTF benches once
the hermetic download lands).

## Toolchain

criterion 0.8 is MSRV-clean — the `bench` CI job installs rustc
1.88 to match the workspace `rust-version`. Local development
on stable works the same way; `RUSTUP_TOOLCHAIN=stable` is the
canonical override if the project's `rust-toolchain.toml`
disagrees with your setup.

[criterion]: https://github.com/bheisler/criterion.rs
[nightly]: ../../.github/workflows/quality-nightly.yml
