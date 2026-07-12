# Real HF recordings (fixtures)

This directory contains real HTTP responses captured from
`huggingface.co`, used by the wiremock-based integration tests in
`voxora-hf/tests/`. Recording them once means the tests catch API
drift — if Hugging Face changes the shape of `config.json` or the
siblings list, the next `cargo test` run flags it immediately.

## Layout

```
fixtures/
├── qwen3-asr-0.6b/
│   ├── _metadata.json          ← response of /api/models/Qwen/Qwen3-ASR-0.6B/revision/main
│   ├── config.json
│   ├── preprocessor_config.json
│   ├── tokenizer_config.json
│   ├── vocab.json              ← ~2.7 MB
│   └── merges.txt              ← ~1.7 MB
├── qwen3-asr-1.7b/
│   ├── _metadata.json
│   ├── config.json
│   ├── preprocessor_config.json
│   ├── tokenizer_config.json
│   ├── vocab.json
│   ├── merges.txt
│   └── model.safetensors.index.json   ← sharded model
└── whisper-tiny/
    ├── _metadata.json
    ├── config.json
    ├── preprocessor_config.json
    ├── tokenizer_config.json
    └── tokenizer.json           ← ~2.5 MB
```

Total: ~12 MB. The actual safetensors weights are **not** captured;
the wiremock tests substitute 1 KB of synthetic bytes because the
real weights are 100 MB – 4 GB. JSON and text files are kept verbatim.

## Refreshing the recordings

The script `refresh.sh` (in this directory) re-captures every file
from `huggingface.co`. Run it when:

- The HF API response shape changes.
- You add support for a new model family and want to capture its
  metadata / config / tokenizer / preprocessor as fixtures.
- A test is failing because a fixture drifted from HF.

```bash
cd voxora-hf/tests/fixtures
./refresh.sh
```

The script requires Python 3 and `curl`-equivalent access to
`huggingface.co`. It rewrites only the metadata and config files
already in the directory; to add a new model, extend the `models`
dict at the top of `refresh.sh`.

## Why not Git LFS?

The total is ~12 MB. Above ~50 MB we'd consider LFS; below, plain
git is fine and the .git directory stays small. The actual model
weights (which would push us into LFS territory) are deliberately
**not** committed — see the substitution strategy above.
