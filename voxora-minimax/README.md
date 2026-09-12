# voxora-minimax

`voxora-minimax` is the **first hosted-API engine adapter** in the
[voxora](https://github.com/airvzxf/voxora) workspace. It implements
[`voxora_traits::AsrEngine`](https://docs.rs/voxora-traits) against the
[MiniMax `/v1/speech_to_text` API](https://platform.minimax.io/docs/api-reference/speech-to-text.md).

Unlike `voxora-whisper` and `voxora-qwen3asr` (which load a model
file from disk and run inference locally), MiniMax runs the
inference server-side. The engine wraps the bearer-token-authenticated
`multipart/form-data` upload, sends a single POST per transcription,
and parses the `verbose_json` response (which carries the full text
plus per-segment timestamps and speaker labels).

## Loading the engine

```rust,no_run
use voxora_minimax::{MiniMaxConfig, MiniMaxEngine};
use voxora_traits::{AsrEngine, TranscribeOptions};

# fn run() -> Result<(), voxora_traits::AsrError> {
let config = MiniMaxConfig::new("sk-...")?;
let engine = MiniMaxEngine::new(config)?;

let samples: Vec<f32> = vec![0.0; 16_000]; // 1 s of silence @ 16 kHz
let result = engine.transcribe(&samples, &TranscribeOptions::default())?;
println!("{}", result.text);
# Ok(()) }
```

## Authentication

MiniMax uses bearer-token auth. The token is resolved via
[`voxora_config::VoxoraConfig::minimax_api_key`](https://docs.rs/voxora-config):

1. Explicit value on `MiniMaxConfig::new(api_key)` — preferred for
   library callers; the key never enters process env.
2. `VOXORA_MINIMAX_API_KEY` env var.
3. `MINIMAX_API_KEY` env var (canonical alias).

The token is wrapped in a `SecretString` (zero-on-drop, `Debug`-redacted).

## Sample-rate contract

**Soft contract.** The engine accepts any sample rate / channel
layout from the caller and wraps the input `&[f32]` in an in-memory
WAV (`f32` LE PCM, mono at the engine's assumed 16 kHz). MiniMax
docs recommend mono 16 kHz; high-sample-rate stereo uploads eat the
50 MB / 500 s server caps without improving transcription quality.
The engine does not enforce the rate — the caller is responsible for
downsampling / channel mixing.

## Server caps

- **500 s** audio duration (server returns 400 `bad_request_error` past this)
- **50 MB** body size (server returns 413 `invalid_request_error` past this)
- **20 supported BCP-47 language tags** (server returns 400 for anything outside)
- **One model** today: `asr-1.0`

## Feature flags

| Flag | Default | Enables |
|---|---|---|
| (none) | yes | the base `MiniMaxConfig` + `MiniMaxEngine` surface |
| `engine-adapter` | no | `MiniMaxAdapter` (`voxora_engine::EngineAdapter` impl) |

No GPU backend features (compute is server-side). No `hf` feature
(MiniMax has no on-disk model).

## CLI

```bash
MINIMAX_API_KEY=<key> cargo run -p voxora-cli --release --features minimax -- minimax audio.wav
```

(Requires `voxora-cli` to be built with the `minimax` feature, which
re-exports `voxora-minimax` via `voxora-bridge`.)

## Live integration test

```bash
MINIMAX_API_KEY=<key> cargo test -p voxora-minimax -- --ignored live_transcription_works
```

The test posts 1 s of 440 Hz sine at 16 kHz mono to the real
MiniMax endpoint and asserts the round-trip succeeds. MiniMax does
not guarantee a specific text output for non-speech audio; the
contract is just that the response is parseable.

## Reference

- [MiniMax API docs](https://platform.minimax.io/docs/api-reference/speech-to-text.md)
- [voxora-traits](https://docs.rs/voxora-traits) — the trait surface this engine implements.
- [voxora-bridge](https://docs.rs/voxora-bridge) — the umbrella crate that re-exports this engine behind the `minimax` feature flag.

## License

Apache-2.0.
