# voxora-cli test fixtures

This directory holds fixtures for the voxora-cli integration tests.

## Layout

```
fixtures/
├── qwen3-asr-0.6b/                 # Captured from huggingface.co
│   ├── _metadata.json
│   ├── config.json
│   ├── preprocessor_config.json
│   ├── tokenizer_config.json
│   ├── vocab.json
│   └── merges.txt
├── qwen3-asr-1.7b/                 # Captured from huggingface.co
│   └── …
└── whisper-tiny/                   # Captured from huggingface.co
    └── …
```

These are the **same** fixtures that live under
`voxora-hf/tests/fixtures/` and are replayed by wiremock to simulate
the Hugging Face Hub without hitting the network. Re-using them across
crates keeps the integration tests in lockstep with the wiremock tests
shipped by `voxora-hf`.

The fixtures were originally captured from real `huggingface.co`
responses. See `voxora-hf/tests/fixtures/refresh.sh` to re-record.

## End-to-end `#[ignore]` fixtures

The end-to-end integration tests (`e2e_qwen3_asr.rs`,
`e2e_whisper_tiny.rs`) download two larger fixtures on first run:

| File | Source | Used by |
|---|---|---|
| `samples/jfk.wav` | `github.com/ggerganov/whisper.cpp` | `e2e_whisper_tiny.rs` (parity check) |
| `samples/sample1.wav` | `github.com/alan890104/qwen3-asr-rs` | `e2e_qwen3_asr.rs` (parity check) |

They cache under `tests/fixtures/samples/` once downloaded.
