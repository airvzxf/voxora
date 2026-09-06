# Fuzz testing (EPIC #133, closes #54)

The voxora workspace carries a `cargo-fuzz` lane for the two
parser surfaces that have caused silent regressions in the
past: `voxora-registry::id::ModelId::parse` and
`voxora-engine::family::EngineFamily::from_config`.

The fuzz infrastructure lives under `fuzz/` and is a **separate
Cargo workspace** with its own nightly toolchain pin so the
stable MSRV build of the main workspace (1.98.1) is unaffected.

## Layout

```text
fuzz/
├── Cargo.toml                   # standalone [workspace], libfuzzer-sys = "0.4"
├── rust-toolchain.toml          # channel = "nightly" (override scope)
├── fuzz_targets/
│   ├── registry_id.rs           # ModelId::parse round-trip property
│   └── engine_family.rs         # EngineFamily::from_config alias match
└── corpus/
    ├── registry_id/             # seed inputs from id.rs unit tests
    └── engine_family/           # seed inputs from family.rs unit tests
```

The seed corpus is committed under `fuzz/corpus/<target>/`;
runtime corpus additions and crash outputs are gitignored under
`/fuzz/target`, `/fuzz/corpus`, and `/fuzz/artifacts` so the
fuzzer's local writes never dirty the tree.

## Targets

### `registry_id`

Property under test:

```text
for all s: &str:
    if let Ok(id) = ModelId::parse(s):
        let canonical = id.canonical();
        assert_eq!(ModelId::parse(&canonical)?, id)
```

The canonical round-trip: every string the parser accepts must
re-parse to the same `ModelId` after going through
`canonical()`. A violation here would mean future code that
joins `ModelId::path` onto a base or compares two `ModelId`s by
their canonical form silently disagrees.

### `engine_family`

Property under test:

```text
for all s: &str:
    matches!(
        s.to_ascii_lowercase().as_str(),
        "whisper" | "qwen3-asr" | "qwen3asr" | "qwen3_asr"
    ) == EngineFamily::from_config(s).is_some()
```

The four canonical literals (and only those) must return
`Some(_)`. `from_config` is a single `match` today; the fuzzer
keeps it that way — if a future refactor accidentally drops a
case or accepts an unrelated string, the next nightly run
catches it.

## Running

```bash
# Compile (also done by `cargo fuzz run`).
cd fuzz
cargo +nightly fuzz build

# Run for 60 s — the canonical nightly duration. libFuzzer
# exits cleanly when the budget elapses; a crash surfaces as
# a non-zero exit and a `crash-<hash>` file under
# `fuzz/artifacts/<target>/`.
cargo +nightly fuzz run registry_id   -- -max_total_time=60
cargo +nightly fuzz run engine_family -- -max_total_time=60
```

The seed corpus under `fuzz/corpus/<target>/` is read on the
first run; subsequent runs grow the corpus in-place when the
fuzzer finds new coverage-increasing inputs. Cleaning the
corpus back to the seed set is `rm -rf fuzz/corpus/<target>/*
&& git checkout fuzz/corpus/<target>/`.

## CI integration

`.github/workflows/quality-nightly.yml` runs both targets for
60 s on `schedule: cron: '0 4 * * *'` plus on-demand
`workflow_dispatch`. PR CI does **not** run fuzzing — the 60 s
budget would still add a minute of CI time per PR, and the
nightly lane catches new regressions within 24 hours. The
workflow installs nightly + `cargo-fuzz` via `dtolnay/rust-
toolchain` and `cargo install` respectively.

## Toolchain

`fuzz/rust-toolchain.toml` pins `channel = "nightly"`. This
file is **scoped** to the `fuzz/` sub-directory: rustup only
reads it when `cargo` is invoked from inside `fuzz/`, so the
main voxora workspace (which pins stable 1.98.1) keeps building
on stable. The nightly channel is required because
`cargo-fuzz` uses libFuzzer + the `-Zsanitizer` family of
flags, both of which are nightly-only.

## Out of scope

- Fuzz targets for engine adapters (`voxora-whisper`,
  `voxora-qwen3asr`). Their surface is stateful — model
  loading, GPU dispatch, audio decoding — and model-based
  fuzzing is a different problem class (deferred to a future
  EPIC).
- Coverage-guided corpus pruning. libFuzzer's default `-corpus`
  heuristics are sufficient at 60 s; revisit when the budget
  goes up.
