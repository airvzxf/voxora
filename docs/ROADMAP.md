# voxora — Roadmap

> The phased plan for going from scaffolding to a Telora-compatible
> model-agnostic ASR layer.
>
> See [`INVESTIGATION.md`](INVESTIGATION.md) for the *why* behind
> these phases.

## Phase 0 — Scaffolding *(this commit)*

- [x] Create `airvzxf/voxora` on GitHub, public, Apache-2.0.
- [x] Write canonical `LICENSE` (Apache-2.0, full text from
      apache.org).
- [x] Write `README.md` with the "Why voxora?" section.
- [x] Write `Cargo.toml` workspace (no crates yet, `publish = false`).
- [x] Write `.gitignore`.
- [x] Write `docs/INVESTIGATION.md` (the recap).
- [x] Write `docs/ROADMAP.md` (this file).
- [x] Write `CONTRIBUTING.md`.
- [x] Write `CODE_OF_CONDUCT.md` (Contributor Covenant v2.1).
- [x] Sign commit with GPG, push to `main`.

**No code yet.** This phase exists so the URL exists and the
investigation is preserved.

## Phase 1 — `voxora-core` (the traits)

**Deliverable**: a publishable library crate that defines the public
API. Two traits are introduced in this phase: `AsrEngine` for
inference and `ModelSource` for acquisition. No engine adapters
and no network code yet.

Tasks:

- [x] Crate `voxora-core/`, add to workspace `members`.
- [x] Define `AsrEngine` trait with `Send + Sync` supertrait.
- [x] Define `TranscribeOptions`, `TranscriptionResult`,
      `TranscriptionSegment`, `ModelCapabilities`, `AsrError`.
- [x] Define `ModelSource` trait with `Send + Sync` supertrait
      and `async_trait` (acquisition is async because HF downloads
      are async).
- [x] Define `ModelDir`, `ModelSourceKind`, `ResolveOptions`,
      `QuantizationPreference`, `Quantization`.
- [x] Add `#[non_exhaustive]` on the public types so we can evolve
      them without breaking SemVer.
- [x] Add `capabilities()` and `list_available()` as default-
      implemented methods that return sensible defaults
      ("unknown" / `Unsupported`), so implementors override only
      what they know.
- [x] Add `voxora-core/Cargo.toml` with `async-trait`,
      `thiserror`, optional `serde` behind a feature flag.
      No `unsafe_code` at the workspace level.
- [x] Explicit zero network dependencies: no `reqwest`, no `tokio`,
      no `http`. `voxora-core` must build offline.
- [x] Unit tests for option defaults, error mapping, and trait
      object construction (`Arc<dyn AsrEngine + Send + Sync>`).
- [x] `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`,
      `cargo test -p voxora-core` all pass on Linux x86_64.

**Trait sketch** (full version in [`INVESTIGATION.md`](INVESTIGATION.md#7-the-trait-we-will-implement)):

```rust
pub trait AsrEngine: Send + Sync {
    fn capabilities(&self) -> ModelCapabilities;
    fn transcribe(
        &self,
        samples: &[f32],
        opts: &TranscribeOptions,
    ) -> Result<TranscriptionResult, AsrError>;
}

#[async_trait::async_trait]
pub trait ModelSource: Send + Sync {
    fn name(&self) -> &'static str;

    async fn resolve(
        &self,
        model_id: &str,
        opts: &ResolveOptions,
    ) -> Result<ModelDir, AsrError>;

    async fn capabilities_for(
        &self,
        model_id: &str,
    ) -> Result<ModelCapabilities, AsrError>;

    async fn list_available(&self) -> Result<Vec<ModelDescriptor>, AsrError> {
        Err(AsrError::Unsupported("list_available"))
    }
}
```

## Phase 2 — `voxora-hf` (Hugging Face model resolution)

**Deliverable**: a library crate that implements `ModelSource` for
Hugging Face. Turns a HF `model_id` (e.g. `Qwen/Qwen3-ASR-0.6B`)
into a `ModelDir` on disk with cached weights, tokenizer, and
config. Selects quantization based on the target device.

Tasks:

- [x] Crate `voxora-hf/`.
- [x] `HuggingFaceSource: ModelSource` — the concrete implementation.
- [x] Implement against the HF Hub REST API directly (no
      `huggingface_hub` dependency), generalising the helper from
      `qwen3-asr-rs/src/hub.rs`.
- [x] `voxora-hf/Cargo.toml` depends on `voxora-core`,
      `tokio`, `reqwest` (rustls), `serde`, `serde_json`, `sha2`,
      `thiserror`, `async-trait`, `futures-util`, `dirs`.
- [x] Cache directory layout:
      `$XDG_CACHE_HOME/voxora/models/<source>/<org>/<name>/<revision>/`
      with a `.complete` marker file (same pattern as
      `qwen3-asr-rs::hub::ensure_model_cached`).
- [x] Detect required files: `config.json`, `tokenizer.json` (or
      `vocab.json` + `merges.txt` + `tokenizer_config.json`),
      `*.safetensors` (single or sharded via
      `model.safetensors.index.json`), `preprocessor_config.json`.
- [x] Quantization detection: `torch_dtype` in `config.json` for
      safetensors models (Bf16 / F16 / F32), GGUF filename suffix
      for whisper.cpp (`q4_K` → Q4K, `q8_0` → Q8_0). Qwen3-ASR is
      auto-detected as BF16 from its architecture string.
- [x] Integrity check: SHA256 against `*.sha256` sidecars if
      present, otherwise skip silently.
- [x] Auth tokens: read `HF_TOKEN` then `HUGGING_FACE_HUB_TOKEN`
      from the environment; `ResolveOptions::token` overrides per
      call.
- [x] Tests: 33 unit tests (offline) + 14 wiremock integration tests
      with **real recordings** captured from `huggingface.co`
      (`tests/fixtures/{qwen3-asr-0.6b,qwen3-asr-1.7b,whisper-tiny}/`),
      plus a `#[ignore]`-gated smoke test for live API drift
      detection.
- [x] `cargo fmt --all --check`, `cargo clippy --workspace
      --all-targets -- -D warnings`, `cargo test --workspace
      --all-targets`, `cargo doc --no-deps --workspace` all green.
- [x] Added `AsrError::Network { url, message, source }` to
      `voxora-core` (keeps `voxora-core` offline-pure; only carries
      `String` + boxed error, no `reqwest` / `tokio` types).
- [x] Constructors for the `#[non_exhaustive]` types
      (`ModelCapabilities::new`, `ModelDescriptor::new` /
      `with_details`, `ModelDir::new`, `ModelSourceKind::tag`) so
      downstream crates can build them without struct expressions.
- [ ] docs.rs metadata; publish to crates.io as `voxora-hf` after
      phase 3 is stable.

## Phase 3 — `voxora-whisper` (engine adapter over `whisper-rs`)

**Deliverable**: an `AsrEngine` implementation backed by `whisper-rs`,
the same library Telora uses today.

Tasks:

- [x] Crate `voxora-whisper/`.
- [x] Feature flags: `metal`, `cuda`, `vulkan`, `cpu` (mirroring
      `whisper-rs` feature flags). `cpu` is the default; `hf` is an
      additional optional flag that pulls in `voxora-hf` for
      `WhisperEngine::from_hf`.
- [x] `WhisperEngine::load(model_path: &Path) -> Result<Self>`.
- [x] Map `ModelCapabilities` from the GGUF / GGML file's metadata
      (multilingual flag via `WhisperContext::is_multilingual`,
      language count via `whisper_rs::get_lang_max_id`).
- [x] Map `TranscribeOptions` to `whisper_rs::FullParams`:
      `language` (validated against whisper's table, rejected as
      `InvalidInput` on miss), `translate` (rejected on
      English-only models), `timestamps` (drives
      `set_no_timestamps`). Quieter runtime: progress / realtime /
      special-token printing disabled.
- [x] Convert `&[f32]` samples → `WhisperState::full`. A fresh
      state is minted per `transcribe()` call so concurrent calls
      on the same engine work without contention.
- [x] Build `TranscriptionResult` from `full_n_segments` and the
      per-segment `WhisperSegment` API (`start_timestamp` /
      `end_timestamp` in centiseconds → samples at 16 kHz,
      `to_str_lossy` for the text). Detected language captured via
      `WhisperState::full_lang_id_from_state`.
- [x] Validate against the canonical `jfk.wav` from `whisper.cpp`
      (parity test gated by `#[ignore]`, downloads the audio and
      `ggml-tiny.bin` on first run, asserts the
      "ask not what your country can do for you" substring is in
      the transcript).
- [x] When `voxora-hf` is available, accept a HF model id as input
      too (downloads ggml-*.bin from `ggerganov/whisper.cpp` via
      `WhisperEngine::from_hf`, gated by the `hf` Cargo feature).

## Phase 4 — `voxora-qwen3asr` (engine adapter over `qwen3-asr-rs`)

**Deliverable**: an `AsrEngine` implementation backed by
`qwen3-asr-rs`. First non-Whisper engine.

Tasks:

- [x] Crate `voxora-qwen3asr/`.
- [x] Re-export `qwen3_asr::AsrInference` and wrap it.
- [x] Map language code: `TranscribeOptions::language` →
      `qwen3_asr::TranscribeOptions::language`. Qwen3-ASR expects
      full English names (`"english"`, `"chinese"`), not ISO 639-1
      codes, so the adapter keeps a closed 20-name list and a
      `validate_lang` helper. ISO 639-1 codes are rejected as
      `InvalidInput` so users get a clear error.
- [x] Honor `QWEN3_ASR_CUDA_NATIVE_BF16` env var (passthrough — no
      adapter-side code; upstream `qwen3-asr` reads it directly on
      `load`).
- [x] Map the `TranscribeResult` output:
      `language` (forced → echo back caller request instead of
      upstream's literal `"forced"` sentinel; auto-detect →
      upstream's language name), `text` (already stripped of the
      `language <lang><asr_text>` prefix upstream, but we `.trim()`
      once for safety).
- [x] Feature flags: `cpu` (default), `metal`, `cuda`, `hf` —
      mirrors upstream `qwen3-asr`. The `hf` flag pulls in
      `voxora-hf` and exposes `QwenAsrEngine::from_hf`.
- [x] `Send + Sync` engine wrapper: upstream `AsrInference` is `Send`
      but not `Sync`, so the adapter wraps it in `Arc<Mutex<…>>` to
      satisfy `Arc<dyn AsrEngine>`.
- [x] Capabilities advertised: multilingual, 20-language list,
      no word-timestamps, no streaming.
- [x] Error mapping: `qwen3_asr::AsrError::{ModelLoad, AudioDecode,
      Inference}` all collapse to `voxora_core::AsrError::Inference`
      with the inner `anyhow` chain preserved in the message.
- [x] `candle_core::Device` re-exported as `voxora_qwen3asr::Device`
      for callers that want explicit device control via
      `load_with_device`.
- [x] Tests: 28 unit tests + 2 doctests (offline) + 2 `#[ignore]`
      integration tests (parity + concurrency, require the model and
      audio fixture).
- [x] `cargo fmt --all --check`, `cargo clippy --workspace
      --all-targets -- -D warnings`, `cargo test --workspace
      --all-targets`, `cargo doc --no-deps --workspace` all green.
- [ ] Add streaming variant later (not in phase 4; requires a
      `StreamingAsrEngine` trait extension).

## Phase 5 — `voxora-cli` (list / download / run)

**Deliverable**: a tiny CLI binary that demonstrates the full stack.

Tasks:

- [x] Crate `voxora-cli/` (binary, `voxora` command).
- [x] Subcommands:
  - [x] `voxora list` — list locally cached models (powered by
        `voxora-hf::cache::list_cached`).
  - [x] `voxora info <hf-model-id>` — fetch model card and
        capabilities from HF Hub without downloading.
  - [x] `voxora download <hf-model-id>` — resolve + download + cache.
  - [x] `voxora run <hf-model-id> <audio.wav>` — load, transcribe,
        print (engine auto-detect from `config.json`, overridable with
        `--engine whisper|qwen3-asr`).
  - [x] `voxora serve` — placeholder; HTTP wrapper deferred to a
        later phase.
- [x] Hardware flag mirroring both engines: `cpu` (default), `metal`,
      `cuda`. Hidden `--base-url` flag for tests pointing at wiremock.
- [x] Use `voxora-hf` for resolution, `voxora-whisper` and
      `voxora-qwen3asr` for engines, auto-selecting based on model
      metadata (capabilities heuristic).
- [x] Distribution: a single binary built from the workspace.
      Verified against the existing Makefile targets (`build-cli`,
      `build-musl` for `x86_64-unknown-linux-musl`; aarch64 musl is
      done by passing the target name to the same rule).
- [x] Tests: 41 unit tests + 11 CLI integration tests
      (`cli_help`, `cli_list`, `cli_info`, `cli_download_dry`
      backed by wiremock) + 4 `#[ignore]`-gated end-to-end tests
      (`e2e_qwen3_asr`, `e2e_whisper_tiny`, `e2e_engine_override`)
      that exercise real HF downloads + real engines.

### Built artifacts

`make build-cli` produces
`target/release/voxora` (and the corresponding musl variant under
`target/x86_64-unknown-linux-musl/release/voxora` for fully static
Linux distributions). The musl build is verified by
`make build-musl`; aarch64 uses the same rule with
`--target aarch64-unknown-linux-musl` after `rustup target add`.

## Phase 6 — Telora integration *(DONE, voxora 0.1.0 on crates.io)*

**Deliverable**: Telora depends on `voxora` instead of `whisper-rs`
directly. The user can switch models by editing `config.toml`.

Tasks:

- [x] Add `voxora = { version = "0.1", features = ["whisper", "qwen3asr"] }`
      to `telora-daemon/Cargo.toml`. *(now `voxora-bridge = "0.1"` in
      the workspace.dependencies table; the umbrella crate re-exports
      voxora-core + voxora-hf + both engines behind feature flags.)*
- [x] Replace `WhisperTranscriber` with `BridgeTranscriber` that
      holds `Arc<dyn AsrEngine>`. *(`telora-daemon/src/transcriber.rs`,
      ISO 639-1 → engine vocabulary mapping for the 20 Qwen3 languages,
      CLI flags `--model-id` / `--model-kind` / `--voxora-cache`.)*
- [x] Add to `telora.toml`:
  ```toml
  model_kind = "whisper" | "qwen3-asr"
  model_id   = "Qwen/Qwen3-ASR-0.6B"   # or "ggerganov/whisper.cpp/ggml-base.bin"
  ```
      *(legacy `model_path` field still honoured: when `model_id` is
      empty and `model_path` is set, the daemon treats `model_path`
      as the model id, so older configs keep working.)*
- [x] **Refactor `telora/telora-models/src/main.rs` to delegate to
      `voxora-hf`**: keep the existing CLI surface
      (`telora-models list | download`), but its implementation calls
      `voxora_hf::HuggingFaceSource::resolve()` under the hood.
      Single source of truth for download logic; UX unchanged for the
      user. No new vocabulary to learn.
- [x] Document in `telora/README.md` how to add a new model without
      code changes (point at `voxora-hf` and `ModelSource` instead
      of duplicating HF download code per engine).
- [x] Confirm AGPL-3 (Telora) + Apache-2.0 (voxora) license
      compatibility holds in `Cargo.toml` (it does — see
      [`INVESTIGATION.md`](docs/INVESTIGATION.md#9-license-decision)).
- [x] **Publish voxora to crates.io**: all five crates
      (`voxora-core`, `voxora-hf`, `voxora-whisper`,
      `voxora-qwen3asr`, `voxora-bridge`) shipped as `0.1.0`.
      Telora consumes them via `voxora-bridge = "0.1"` from the
      registry; the sibling path-dep layout is no longer required.

### What got fixed during phase 6 smoke testing

The integration surfaced four pre-existing bugs that the validation
gauntlet alone missed (every end-to-end test is `#[ignore]`-gated
on real model + audio downloads):

1. **voxora-cli cache root**: `hf_cache_dir()` only consulted
   `--cache` and `VOXORA_CACHE_DIR`. Added a `dirs::cache_dir()`
   fallback so the CLI honours `XDG_CACHE_HOME` / `$HOME/.cache/`
   by default. (Commit `134366a`.)
2. **voxora-hf single-file downloads**: `HuggingFaceSource::resolve`
   only knew about whole-repo HF layouts. Added a `org/repo/file`
   three-segment id path so `ggerganov/whisper.cpp/ggml-base.bin`
   resolves to a direct download of the ggml file, restoring
   parity with the legacy telora-models convention. (Commit `73343c2`.)
3. **voxora-hf capability synthesis**: `capabilities_for` always
   fetched `config.json` (404 for single-file ids). Synthesises
   `ModelCapabilities` from the filename instead (`ggml-*.bin` →
   multilingual Whisper, `.en.` → English-only, etc.).
   (Commit `e1f7f63`.)
4. **voxora-qwen3asr tokenizer synthesis**: `qwen3_asr::AsrInference::load`
   requires `tokenizer.json` but Qwen3-ASR's official HF release
   only ships `vocab.json` + `merges.txt` + `tokenizer_config.json`.
   `QwenAsrEngine::from_hf` now builds `tokenizer.json` from the
   trio before loading. (Commit `c4acb28`.)
5. **voxora-cli PCM audio scaling**: `decode_wav` divided every
   PCM sample by `i32::MAX`, making 16-bit audio 65536× too quiet
   (engines then saw what looked like silence). Now honours the
   declared bit depth and uses `2^(bits-1)` as the divisor for
   16/24/32-bit WAVs. (Commit `fe66183`.)

### Validation gauntlet at end of phase 6

```text
cargo fmt --all --check                                              ✓
cargo clippy --workspace --all-targets -- -D warnings               ✓
cargo test --workspace --all-targets                                205 pass / 0 fail / 11 #[ignore]
cargo doc --no-deps --workspace                                     ✓
cargo build --workspace --all-targets                              ✓

End-to-end on the VPS:
  voxora run Qwen/Qwen3-ASR-0.6B /tmp/jfk.wav --engine qwen3-asr --language english
    → "And so, my fellow Americans, ask not what your country can do for you;
       ask what you can do for your country."
  voxora run ggerganov/whisper.cpp/ggml-tiny.bin /tmp/jfk.wav --language en
    → "And so my fellow Americans ask not what your country can do for you
       ask what you can do for your country."

Published to crates.io (5 crates, v0.1.0):
  https://crates.io/crates/voxora-core
  https://crates.io/crates/voxora-hf
  https://crates.io/crates/voxora-whisper
  https://crates.io/crates/voxora-qwen3asr
  https://crates.io/crates/voxora-bridge
```

## Phase 7 — More engines *(planned, not yet started)*

The same `voxora-bridge` umbrella that hides voxora-core + voxora-hf
behind two Cargo features today scales the same way to any new
engine. Each engine adapter follows the recipe from phases 3/4:

1. **Declare a new workspace member** (e.g. `voxora-parakeet`)
   that implements `voxora_core::AsrEngine`.
2. **Re-export it from `voxora-bridge`** behind a new Cargo feature
   (e.g. `parakeet = ["dep:voxora-parakeet"]`) and add a new
   variant to `voxora_bridge::ModelKind`.
3. **Add tests** in the same pattern as `voxora-whisper` /
   `voxora-qwen3asr`: offline unit tests for the trait glue,
   `#[ignore]`-gated integration tests that hit real HF.

The current candidates, in priority order:

- [ ] **voxora-parakeet** — NVIDIA Parakeet via candle. The most
      likely first new engine because NVIDIA publishes reference
      candle implementations (neMo-style) and there is HF
      community momentum. Same model_kind = "parakeet" /
      "parakeet-tdt" pattern as today. Blocking: depends on a
      candle-friendly Parakeet implementation landing in
      `huggingface/candle` (not blocked on us — pure upstream
      coordination).

- [ ] **voxora-voxtral** — Mistral Voxtral, via candle. Active
      upstream work; once candle support lands the adapter is a
      matter of glue.

- [ ] **voxora-granite-speech** — IBM Granite-Speech, via candle.
      Same story as Voxtral: blocked on candle support.

- [ ] **voxora-local** — a `ModelSource` impl that reads from a
      local directory. Useful for offline users who vendor the
      weights, for hermetic test environments, and for nightly CI
      without HF credentials. Trivially small implementation on top
      of the existing `voxora_core::ModelSource` trait.

- [ ] **voxora-tts** — text-to-speech, the reverse direction. Lives
      outside the `voxora-bridge` umbrella because the engine trait
      signature is different (`text → audio` not `audio → text`).
      Independent of phase 7 engine work.

- [ ] **voxora-vad** — voice activity detection, shared utility
      across all engines. Useful for trimming silence before ASR
      and for live-streaming UIs. Probably wants a streaming-aware
      trait extension (`StreamingAsrEngine`) which is a real
      breaking change, so this lands as phase 8.

- [ ] **voxora-diarization** — speaker diarization ("who spoke
      when"). Compositional on top of ASR engines (we already get
      word timestamps from whisper; qwen3-ASR has no segments so
      diarization is whisper-only initially).

### Phase 7 design notes

Two recurring patterns we should bake in up front, not retrofit:

1. **`StreamingAsrEngine` trait extension**. The current
   `AsrEngine::transcribe(&self, samples: &[f32], …) -> Result<…>`
   is whole-audio-in / whole-text-out. Streaming engines
   (Whisper's full encoder/decoder split, Voxtral's chunk-based
   decoder, Parakeet TDT) want `transcribe_chunk` /
   `finalise_chunk` calls. Adding the trait extension is a breaking
   change for engine authors but a non-breaking change for
   downstream callers (existing `transcribe` becomes the
   single-chunk special case). Plan: design alongside the first
   streaming engine, not before.

2. **Hardware dispatcher** (CUDA → Metal → CPU at runtime, not
   compile time). Right now voxora-qwen3asr takes a device at
   load time. For consumers that want "pick whatever's available",
   we need a `best_device()` helper that resolves at process
   start. qwen3-asr already has one upstream; we just need to
   surface it.

These two are scoped under phase 7 because they cut across every
engine, so they should ship with the next engine rather than
landing as standalone PRs.

### Non-engine roadmap items (could ship in parallel)

- [ ] **CI on GitHub Actions** — voxora currently has no CI. Every
      phase 6 PR was validated locally + on this VPS only. A
      matrix workflow (clippy + fmt + test + build, Linux x86_64
      + aarch64, MSRV 1.85) would have caught the
      `transcribe_wav` filename collision warning and the
      `categories = ["science::linguistics"]` retiree at PR time
      instead of after merge. Should land before phase 7's first
      new engine.

- [ ] **docs.rs** — crates.io metadata already includes
      `documentation = "https://docs.rs/voxora-bridge"` via the
      docs.rs auto-link, but the README is the workspace README,
      not a per-crate one. Each crate would benefit from its own
      crate-level README so docs.rs shows the right one. Trivial
      follow-up.

- [ ] **Telora-models deprecation** — the legacy `telora-models`
      binary is now a thin wrapper around `voxora-cli`. Retire it
      in favour of `voxora-cli list/download`. (Tracked as a TODO
      item in `telora/TODO.md`.)

---

*Last updated: 2026-07-14 — Phase 6 closed (voxora 0.1.0 published
to crates.io, telora consuming via `voxora-bridge = "0.1"`); Phase 7
scoped (more engines + cross-cutting trait extension + hardware
dispatcher + non-engine roadmap items). Previous milestones: phase 5
`voxora-cli` (2026-07-12), phase 4 `voxora-qwen3asr` (2026-07-12),
phase 3 `voxora-whisper` (2026-07-12), phase 2 `voxora-hf`
(2026-07-12), phase 1 `voxora-core` (2026-07-11).*