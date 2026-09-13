# Changelog

All notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.1] — 2026-09-13

Coordinated patch release covering the `0.6.0 → 0.6.1` cycle.
Per `AGENTS.md` § "Version coordination", all 13 participating
crates ship at 0.6.1. No public API change on this crate; the
bump is purely a workspace-pin update (closes the eight
issues #110, #111, #112, #113, #114, #142, #164, #58 whose
source changes landed in the other crates).

## [0.6.0] — 2026-09-12

Initial release. Part of [EPIC #153](https://github.com/airvzxf/voxora/issues/153)
(closes [#154](https://github.com/airvzxf/voxora/issues/154)).
The 6 participating crates (`voxora-engine`, `voxora-bridge`,
`voxora-cli`, `voxora-config`, `voxora-registry`, and the new
`voxora-minimax`) ship at 0.6.0 per `AGENTS.md` § "Version
coordination".

### Added
- **`MiniMaxEngine`**: the first hosted-API engine adapter in the
  voxora workspace. Implements `voxora_traits::AsrEngine` against
  the [MiniMax `/v1/speech_to_text` API](https://platform.minimax.io/docs/api-reference/speech-to-text.md).
  Bearer-token auth via `MiniMaxConfig::api_key` (or
  `VOXORA_MINIMAX_API_KEY` / `MINIMAX_API_KEY` cascade).
  `multipart/form-data` upload with in-memory WAV wrapper;
  `verbose_json` response parsed into `TranscriptionResult` +
  per-segment `TranscriptionSegment`s.
- **`MiniMaxConfig`**: per-crate config struct (`api_key`,
  `endpoint`, `model`, `timeout_secs`) with builder methods.
  `api_key` is wrapped in `SecretString` (zero-on-drop,
  `Debug`-redacted).
- **`MiniMaxAdapter`** behind the `engine-adapter` feature: the
  optional `voxora_engine::EngineAdapter` wrapper. Mirrors the
  opt-in pattern from `voxora-whisper` / `voxora-qwen3asr`.
- **`validate_lang_bcp47` / `known_languages_bcp47`**: the
  closed 20-tag BCP-47 whitelist, mirroring
  `voxora-qwen3asr`'s `language.rs` pattern.
- **In-memory WAV header writer** (`f32` LE PCM): zero-dependency
  encoder. Mono 16 kHz is the assumed rate; stereo uploads are
  supported but documented as wasteful given the 50 MB / 500 s
  server caps.
- **`transcribe_wav_minimax` example** + **`benches/transcribe_wav.rs`**
  criterion stub: the example posts a WAV to the live API
  (`MINIMAX_API_KEY` env var); the bench is a `#[ignore]`-gated
  compile-only placeholder that the workspace-wide
  `cargo bench --workspace --no-run` lane compiles.
- **Live `#[ignore]`-gated test** in `tests/capabilities.rs`:
  `MINIMAX_API_KEY=<key> cargo test -p voxora-minimax -- --ignored live_transcription_works`
  posts 1 s of 440 Hz sine to the real endpoint and asserts the
  round-trip succeeds.

### Out of scope (tracked separately, deferred from EPIC #153)
- **Streaming `StreamingAsrEngine` impl**. MiniMax SSE streaming is
  well-defined upstream (`{index, delta, finish}` per the OpenAPI
  schema) but adding it requires `async-trait`. Deferred to a
  future release.
- **Diarization-specific output**. The MiniMax `segments[].speaker`
  field is folded into the segment text (`[S1] hello`) for now;
  surfacing speaker id requires a new `voxora_traits::TranscriptionSegment`
  field, which is out of scope for this release.
