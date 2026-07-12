# voxora-whisper test fixtures

This directory holds audio and model fixtures used by the
`#[ignore]`-gated integration tests.

The tests download on demand:

| File | Source | Size | Used by |
|---|---|---|---|
| `jfk.wav` | `github.com/ggerganov/whisper.cpp` | ~330 KB | `tests/parity.rs::jfk_parity_substring_match` |
| `ggml-tiny.bin` | `huggingface.co/ggerganov/whisper.cpp` | ~75 MB | `tests/parity.rs` and `tests/concurrency.rs` |
| `ggml-tiny.en.bin` | `huggingface.co/ggerganov/whisper.cpp` | ~75 MB | `tests/parity.rs::english_only_model_reports_no_multilingual` |

Run the gated tests with:

```text
cargo test -p voxora-whisper -- --ignored
```

To pre-populate fixtures manually, place the files in this directory
and the tests will skip the download step.

The `ggml-tiny.bin` model is also cached under
`$XDG_CACHE_HOME/voxora/whisper-fixtures/` so it is shared across
runs without re-downloading.