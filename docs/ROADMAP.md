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

- [ ] Crate `voxora-whisper/`.
- [ ] Feature flags: `metal`, `cuda`, `vulkan`, `cpu` (mirroring
      `whisper-rs` feature flags).
- [ ] `WhisperEngine::load(model_path: &Path) -> Result<Self>`.
- [ ] Map `ModelCapabilities` from the GGUF / GGML file's metadata
      (language count, English-only vs multilingual).
- [ ] Map `TranscribeOptions` to `whisper_rs::FullParams`:
      `language`, `translate`, `print_timestamps`.
- [ ] Convert `&[f32]` samples → `whisper_rs::WhisperState::full`.
- [ ] Build `TranscriptionResult` from `full_n_segments` /
      `full_get_segment_text` / `full_get_segment_t0` /
      `full_get_segment_t1`.
- [ ] Validate against Telora's existing behaviour on the same audio
      file (parity test, regression guard).
- [ ] When `voxora-hf` is available, accept a HF model id as input
      too (downloads ggml-*.bin from `ggerganov/whisper.cpp`).

## Phase 4 — `voxora-qwen3asr` (engine adapter over `qwen3-asr-rs`)

**Deliverable**: an `AsrEngine` implementation backed by
`qwen3-asr-rs`. First non-Whisper engine.

Tasks:

- [ ] Crate `voxora-qwen3asr/`.
- [ ] Re-export `qwen3_asr::AsrInference` and wrap it.
- [ ] Map language code: `TranscribeOptions::language` →
      `qwen3_asr::TranscribeOptions::language`. Qwen3-ASR expects
      full English names (`"english"`, `"chinese"`), not ISO 639-1
      codes, so the adapter needs a small lookup table.
- [ ] Honor `QWEN3_ASR_CUDA_NATIVE_BF16` env var (passthrough).
- [ ] Map the `TranscribeResult` output:
      `language` (forced or detected), `text` (strip
      `language <lang><asr_text>` prefix).
- [ ] Add streaming variant later (not in phase 4; requires a
      `StreamingAsrEngine` trait extension).

## Phase 5 — `voxora-cli` (list / download / run)

**Deliverable**: a tiny CLI binary that demonstrates the full stack.

Tasks:

- [ ] Crate `voxora-cli/` (binary).
- [ ] Subcommands:
  - `voxora list` — list locally cached models.
  - `voxora info <hf-model-id>` — fetch model card and capabilities
    from HF Hub without downloading.
  - `voxora download <hf-model-id>` — resolve + download + cache.
  - `voxora run <hf-model-id> <audio.wav>` — load, transcribe, print.
  - `voxora serve` (later) — HTTP wrapper, optional.
- [ ] Hardware auto-detect on startup, log the chosen backend.
- [ ] Use `voxora-hf` for resolution, `voxora-whisper` and
      `voxora-qwen3asr` for engines, auto-selecting based on model
      metadata.
- [ ] Distribution: single static binary (musl) for Linux x86_64
      and aarch64. Verified against the `qwen3-asr-rs`
      musl build pipeline.

## Phase 6 — Telora integration

**Deliverable**: Telora depends on `voxora` instead of `whisper-rs`
directly. The user can switch models by editing `config.toml`.

Tasks:

- [ ] Add `voxora = { version = "0.1", features = ["whisper", "qwen3asr"] }`
      to `telora-daemon/Cargo.toml`.
- [ ] Replace `WhisperTranscriber` with `BridgeTranscriber` that
      holds `Arc<dyn AsrEngine>`.
- [ ] Add to `telora.toml`:
  ```toml
  model_kind = "whisper" | "qwen3-asr"
  model_id   = "Qwen/Qwen3-ASR-0.6B"   # or "ggerganov/whisper.cpp/ggml-base.bin"
  ```
- [ ] **Refactor `telora/telora-models/src/main.rs` to delegate to
      `voxora-hf`**: keep the existing CLI surface
      (`telora-models list | download`), but its implementation calls
      `voxora_hf::HuggingFaceSource::resolve()` under the hood.
      Single source of truth for download logic; UX unchanged for the
      user. No new vocabulary to learn.
- [ ] Document in `telora/README.md` how to add a new model without
      code changes (point at `voxora-hf` and `ModelSource` instead
      of duplicating HF download code per engine).
- [ ] Confirm AGPL-3 (Telora) + Apache-2.0 (voxora) license
      compatibility holds in `Cargo.toml` (it does — see
      [`INVESTIGATION.md`](docs/INVESTIGATION.md#9-license-decision)).

## Phase 7+ (future, not yet planned)

- [ ] `voxora-parakeet` (NVIDIA Parakeet via candle).
- [ ] `voxora-voxtral` (Mistral Voxtral, when candle support lands).
- [ ] `voxora-granite-speech` (IBM Granite-Speech, same).
- [ ] `voxora-local` (a `ModelSource` impl that reads from a local
      directory — useful for testing and for users who vendor the
      weights).
- [ ] `voxora-tts` (text-to-speech, reverse direction).
- [ ] `voxora-vad` (voice activity detection, shared).
- [ ] `voxora-diarization` (speaker diarization).

These are sketched only; their design will be revised when each
underlying model lands in `candle-transformers` (or, for the
non-engine items, when the underlying tech stabilizes).

---

*Last updated: 2026-07-12. Updated again on 2026-07-09 to add the
`ModelSource` trait and the `telora-models` → `voxora-hf`
delegation. Updated on 2026-07-12 to mark Phase 2 complete
(`voxora-hf` shipped with 47 tests + 2 ignored live smoke tests).*