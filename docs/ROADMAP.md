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

## Phase 1 — `voxora-core` (the trait)

**Deliverable**: a publishable library crate that defines the public
API. No engine adapters yet.

Tasks:

- [ ] Crate `voxora-core/`, add to workspace `members`.
- [ ] Define `AsrEngine` trait with `Send + Sync` supertrait.
- [ ] Define `TranscribeOptions`, `TranscriptionResult`,
      `TranscriptionSegment`, `ModelCapabilities`, `AsrError`.
- [ ] Add `#[non_exhaustive]` on the public types so we can evolve
      them without breaking SemVer.
- [ ] Add `capabilities()` as a default-implemented method that
      returns a sensible "unknown" default, so implementors can
      override only what they know.
- [ ] Add `voxora-core/Cargo.toml` with `serde` (optional, behind a
      feature flag), `thiserror` for the error enum, no
      `unsafe_code`.
- [ ] Unit tests for option defaults and error mapping.
- [ ] `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
      `cargo test -p voxora-core` all pass on Linux x86_64.

**Trait sketch** (full version in [`INVESTIGATION.md`](INVESTIGATION.md#6-the-trait-we-will-implement)):

```rust
pub trait AsrEngine: Send + Sync {
    fn capabilities(&self) -> ModelCapabilities;
    fn transcribe(
        &self,
        samples: &[f32],
        opts: &TranscribeOptions,
    ) -> Result<TranscriptionResult, AsrError>;
}
```

## Phase 2 — `voxora-hf` (Hugging Face model resolution)

**Deliverable**: a library crate that turns a HF `model_id` (e.g.
`Qwen/Qwen3-ASR-0.6B`) into a local model directory with cached
weights, tokenizer, and config. Selects quantization based on the
target device.

Tasks:

- [ ] Crate `voxora-hf/`.
- [ ] Wrap `huggingface_hub` Rust client (or implement minimal
      `snapshot_download` against the HF Hub REST API directly, to
      keep the dependency footprint small — we already do this in
      `qwen3-asr-rs/src/hub.rs`, port and generalize).
- [ ] Cache directory layout:
      `$XDG_CACHE_HOME/voxora/models/<org>/<name>/<revision>/`
      with a `.complete` marker file (same pattern as
      `qwen3-asr-rs::hub::ensure_model_cached`).
- [ ] Detect required files: `config.json`, `tokenizer.json`,
      `*.safetensors` (single or sharded via
      `model.safetensors.index.json`), preprocessor_config.json.
- [ ] Quantization selector: takes a `VoxoraDevice` (CUDA, Metal, CPU)
      and a `QuantizationPreference` enum (`Auto`, `Bf16`, `F32`,
      `Q4`, `Q8`) and returns a concrete path on disk.
- [ ] Integrity check: SHA256 against `*.sha256` files if present,
      otherwise skip with a warning.
- [ ] Tests: a `#[cfg(test)]` mock that points at a local fixture
      directory.

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
  model_id   = "ggerganov/whisper.cpp/ggml-base.bin"   # or HF id
  ```
- [ ] `telora-models` learns the new vocabulary (`voxora download`).
- [ ] Document in `telora/README.md` how to add a new model without
      code changes.

## Phase 7+ (future, not yet planned)

- [ ] `voxora-parakeet` (NVIDIA Parakeet via candle).
- [ ] `voxora-voxtral` (Mistral Voxtral, when candle support lands).
- [ ] `voxora-granite-speech` (IBM Granite-Speech, same).
- [ ] `voxora-tts` (text-to-speech, reverse direction).
- [ ] `voxora-vad` (voice activity detection, shared).
- [ ] `voxora-diarization` (speaker diarization).

These are sketched only; their design will be revised when each
underlying model lands in `candle-transformers`.

---

*Last updated: 2026-07-09.*