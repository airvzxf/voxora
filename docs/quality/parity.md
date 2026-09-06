# Cross-engine parity (EPIC #133, closes #59)

`voxora-bridge` carries a single `#[ignore]`-d integration test
that exercises both engines against the **same audio** and
asserts they agree within a Word Error Rate (WER) threshold.

## The contract

```text
for shared_audio in [sample1.wav, ...]:
    whisper_text  = WhisperEngine::transcribe(shared_audio)
    qwen3_text    = QwenAsrEngine::transcribe(shared_audio)
    wer(normalize(whisper_text), normalize(qwen3_text)) < 0.3
```

Where `normalize` lower-cases, strips ASCII punctuation, maps
smart-quote variants to ASCII apostrophes, and collapses
whitespace. Without normalisation, the WER comparison would
treat `Hello, world!` and `hello world` as different — the test
applies the canonical four-pass transform so the score
reflects semantic agreement.

## Why this lives in `voxora-bridge`

`voxora-testkit` is **offline-only by manifest**
(`voxora-testkit/Cargo.toml:18` `publish = false`,
`test = false`, `bench = false`, no network, no engine deps).
Pulling `voxora-whisper` + `voxora-qwen3asr` in as dev-deps
would flip testkit into a hybrid crate and break every consumer
that uses it as a hermetic dependency.

`voxora-bridge` is the umbrella crate that already re-exports
both engines behind Cargo features (closes #49 + EPIC #117),
so the parity test becomes a **published contract**: a
downstream consumer that enables `voxora-bridge/parity` gets the
cross-engine guarantee for free.

## Why it is `#[ignore]`-d

The test requires real model weights:

| Engine | Checkpoint | Size |
|---|---|---|
| Whisper | `ggml-tiny.bin` (75 MB) | pulled from `huggingface.co/ggerganov/whisper.cpp` via `voxora-testkit::resolve_real_fixture` |
| Qwen3-ASR | `Qwen/Qwen3-ASR-0.6B` | **~1.7 GB** (not ~600 MB as the original issue estimated — the un-sharded safetensors release is ~1.7 GB) |

Both downloads are too heavy for PR CI. The `#[ignore]` gate
keeps the offline lanes green; the test is invoked manually:

```text
cargo test -p voxora-bridge --test cross_engine_parity \
    --features parity -- --ignored --nocapture
```

## Threshold rationale

`WER < 0.3` is the acceptance threshold from the issue body.
It is intentionally loose: Whisper's greedy decoder and
Qwen3-ASR's beam-search-lite will diverge on filler words,
punctuation, and the trailing silence in `sample1.wav`. The
test is a regression tripwire on **gross engine disagreement**,
not a strict-text-equality gate. A failure indicates that one
engine has shifted its transcript behaviour in a way the other
does not — a real signal worth investigating, but not a
guarantee that one engine is "right" and the other "wrong".

## Fixtures

- `sample1.wav` — ~3 s of English speech from
  [`alan890104/qwen3-asr-rs`](https://github.com/alan890104/qwen3-asr-rs/blob/main/tests/fixtures/audio/sample1.wav),
  mono 16 kHz, transcribed text "The quick brown fox jumps over
  the lazy dog." Both engines must identify the canonical
  pangram (or a recognisable paraphrase) for the WER to stay
  below `0.3`.
- `ggml-tiny.bin` — Whisper's smallest English model. The
  testkit cache key is `voxora/fixtures/ggml-tiny.bin`; the
  Whisper parity test reuses it so the cache stays hot across
  the workspace.

The Qwen3-ASR model is resolved through the real
[`voxora_hf::HuggingFaceSource`] so the sha256 sidecar
verification (closes the integrity half of EPIC #109) stays in
the production download path. `voxora-testkit` deliberately
does **not** host this URL — re-implementing HF download
inside the testkit would fork the download path silently.

## CI integration

The test does **not** run on PR CI; the offline CI lane stays
green because the test is `#[ignore]`-d. A future nightly
workflow (mirrors `quality-nightly.yml` for fuzzing, closes
#54) would invoke the test against a hermetic runner with the
two model checkpoints pre-cached; until that lands the manual
invocation above is the canonical way to exercise the
contract.

## Normalisation

`voxora_testkit::wer` splits on whitespace and compares tokens
verbatim. Without normalisation, semantically identical
transcripts produce a non-zero WER. The [`normalize`] helper in
`tests/cross_engine_parity.rs` applies, in order:

1. ASCII lower-case.
2. Map `\u{2018}` / `\u{2019}` / `\u{201C}` / `\u{201D}` to
   ASCII `'` / `"` so smart-quote variants of `don't` match
   the ASCII-spelled `don't`.
3. Keep ASCII alphanumerics, whitespace, and `'`; drop
   everything else (replaced with a single space, then
   collapsed in the next pass).
4. Collapse runs of whitespace into a single space and trim.

The normalisation helper has its own unit tests
(`tests/cross_engine_parity.rs::tests`); they run in the
default test lane because they are pure-string and need no
model weights.
