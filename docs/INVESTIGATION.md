# voxora — Investigation Recap

> The state of every repo involved, the gap we are filling, the
> options we considered, and the decisions we made before writing a
> single line of code.
>
> Last updated: 2026-07-09.

---

## 1. Why this project exists

Telora is a local Speech-to-Text assistant for Linux, written in Rust.
It was originally bound to `whisper-rs` because Whisper was one of the
few ASR models with a first-class native Rust crate. The maintainer's
long-standing goal was to make Telora **model-agnostic**: the user
should be able to pull any model from Hugging Face (Whisper, Qwen3-ASR,
Parakeet, Voxtral, Granite-Speech, …) and have Telora transcribe with
it, with no code changes.

When the maintainer asked an AI assistant to design this abstraction,
several practical blockers emerged. None of them are documented in
upstream Telora (TODO.md and CONTRIBUTING.md only describe hardware
backends, all of which are still whisper-rs variants), so the entire
investigation was reconstructed from commits, PR threads, and the
contents of each repo. That reconstruction is what this document
records, so the next contributor (or the maintainer after a long
break) does not have to repeat the discovery work.

## 2. State of the three repos involved

### `airvzxf/telora` (Rust STT client for Linux)

| | |
|---|---|
| Default branch | `main` |
| Latest commits | `c8e9c4a` (CI pinning), `10cbf17` (actions bump), `a1ea8e8` (GUI text cleanup) |
| Workspace crates | `telora-ctl`, `telora-daemon`, `telora-gui`, `telora-models` |
| Backend today | `whisper-rs` v0.13.2 with `cuda` feature flag |
| License | AGPL-3 |

**Key finding**: the refactor already exists. Commit `328724a`
*"refactor: introduce Transcriber trait to decouple speech-to-text
backend"* introduced a `Transcriber` trait in
`telora-daemon/src/transcriber.rs:6`:

```rust
pub trait Transcriber: Send {
    fn transcribe(&mut self, audio_data: &[f32], language: Option<&str>) -> Result<String>;
}
```

This is the exact decoupling point. `WhisperTranscriber` is currently
the only implementation, but the trait is already designed so that a
`Qwen3AsrTranscriber`, `ParakeetTranscriber`, or any other
`Transcriber` can be plugged in by changing the wiring in
`telora-daemon/src/main.rs`.

`TODO.md` already plans a multi-backend *launcher* architecture (CPU /
CUDA / OpenVINO / ROCm), but every variant described is still a
whisper-rs build with different acceleration backends. The plan does
not yet contemplate swapping the model family.

### `airvzxf/candle` (fork of `huggingface/candle`)

| | |
|---|---|
| Upstream | `huggingface/candle` |
| Origin | `airvzxf/candle.git` |
| Open PR | **#3509** — *Add Qwen3-ASR: multilingual speech recognition model to candle-transformers* |

The PR adds ~2 650 LOC under
`candle-transformers/src/models/qwen3_asr/` and a complete example
under `candle-examples/examples/qwen3-asr/`. Components:

| Module | Purpose |
|---|---|
| `audio_encoder.rs` | Feature extraction + conv downsample + encoder transformer |
| `model.rs` | `TextModel` decoder + full multimodal Model with audio merging |
| `rope.rs` | Multimodal Rotary Position Embedding (mRoPE), concat + interleaved |
| `kv_cache.rs` | Dynamic KV cache for autoregressive streaming generation |
| `mod.rs` | Configuration deserialization from HF `config.json` |

The PR has been **open since 2026-05-05** (~2 months at time of
writing) with no review from `ivarflakstad` or `EricLBuehler`. The
thread contains:

- `lucasjinreal` (2026-05-17): *"if no speed compare with pytorch, they
  won't merge any ASR pr."*
- `airvzxf` (2026-07-07): two comment posts with full benchmarks
  (CPU on i7-7820HK, GPU on rented RTX 3090).
- No follow-up from the maintainers.

The benchmark results the maintainer posted (full data in
[`benchmarks/RESULTS.md`](https://github.com/airvzxf/qwen3-asr-rs/blob/bench/compare-rust-vs-torch/benchmarks/RESULTS.md)
of the fork):

| Metric | Rust candle CPU | torch CPU | Rust candle GPU (RTX 3090) | torch GPU |
|---|---:|---:|---:|---:|
| RTF (mean across 5 audios) | 1.089 | 0.435 | **0.0313** | 0.1074 |
| Cold start → first result | 8 s | 57 s | **3.5 s** | 17.6 s |
| RSS peak | 3.8 GB | 6.4 GB | — | — |
| VRAM peak | — | — | 4.2 GB | 4.2 GB |
| Static binary size | 11 MB | 1.5 GB venv | 11 MB | 1.5 GB venv |
| Ground-truth match | 5/5 | 4/5 | 5/5 | 4/5 |

Headline: **CPU 2.5× slower than torch, GPU 3.4× faster than torch,
cold-start 7× faster, single 11 MB statically-linked binary with zero
runtime dependencies.** The Rust implementation is also numerically
exact on the official Qwen3-ASR test audio.

**Why this matters for voxora**: PR #3509 may eventually merge, it may
not. Either way, the Qwen3-ASR model code in that PR is the canonical
candle implementation, and voxora will consume it (either as a
vendored copy initially, or as a `candle-transformers` dependency
once merged). The PR is **not** a blocker for voxora and **not** a
dependency of voxora — voxora works against the maintained
`qwen3-asr-rs` crate today.

### `airvzxf/qwen3-asr-rs` (fork of `alan890104/qwen3-asr-rs`)

| | |
|---|---|
| Upstream | `alan890104/qwen3-asr-rs` |
| Origin | `airvzxf/qwen3-asr-rs.git` |
| crates.io | `qwen3-asr` v0.2.2 |
| Default branch (fork) | `bench/compare-rust-vs-torch` |

The upstream crate (`alan890104/qwen3-asr-rs`) provides a working
`AsrInference` engine:

- Supports Qwen3-ASR 0.6B and 1.7B.
- Pure Rust over candle (Metal, CUDA, CPU).
- `AsrInference::load(model_dir, device)` and
  `AsrInference::from_pretrained(model_id, cache_dir, device)` APIs.
- Streaming with cross-session `initial_text` context.
- BF16/F16→F32 conversion patch for CPU and Pascal-class GPUs
  (`src/inference.rs:524`, gated by the
  `QWEN3_ASR_CUDA_NATIVE_BF16` env var).
- Hugging Face model download helper (`src/hub.rs`).

The fork added the benchmark suite that PR #3509 references. The
relevant code paths (`encoder.rs`, `decoder.rs`, `mel.rs`,
`streaming.rs`, `hub.rs`, `inference.rs`) are the building blocks
voxora will wrap with its own trait.

## 3. The existing landscape: `transcribe-rs`

The Rust ASR ecosystem already has a strong community project:

- [cjpais/transcribe-rs](https://github.com/cjpais/transcribe-rs)
  — 233⭐ at time of writing.
- One `SpeechModel` trait, one `TranscribeOptions` struct, one
  `TranscriptionResult` struct.
- 9 engines implemented: Parakeet, Canary, Cohere, Moonshine,
  SenseVoice, GigaAM, Whisper (GGML), Whisperfile, OpenAI.
- Hardware acceleration via ORT (`ort-cuda`, `ort-rocm`, `ort-directml`,
  `ort-coreml`, `ort-webgpu`) and via whisper.cpp (`whisper-metal`,
  `whisper-vulkan`, `whisper-cuda`).

**Critically: every engine except whisper.cpp goes through ONNX
Runtime.** The candle-native architecture of Qwen3-ASR
(encoder + LLM with mRoPE) does not have an ONNX export that anyone
maintains, and exporting it would lose the candle-specific benefits
(fused kernels, no Python).

`transcribe-rs` is what we considered duplicating. We chose not to.
See section 5.

## 4. The gap

| Need | Fulfilled by | Gaps |
|---|---|---|
| Local Whisper inference | `whisper-rs` (used by Telora) | Whisper only |
| Qwen3-ASR inference | `qwen3-asr-rs` (your fork) | Qwen3 only, its own API |
| Other HF models (Parakeet, Voxtral, Granite-Speech) | nothing Rust-native | — |
| Trait that unifies all engines | `transcribe-rs` (SpeechModel) | ORT-only substrate, not candle-native |
| HF model auto-resolution | partial in `qwen3-asr-rs::hub` | tied to that one engine |
| Cross-engine runtime dispatch (CUDA/Metal/CPU) | per-engine today | not unified |

**voxora** fills every cell of the third column:

- A single `AsrEngine` trait modeled on `transcribe-rs::SpeechModel`
  but with a Send+Sync contract that suits daemon-style consumers like
  Telora.
- A `voxora-hf` crate for HF model resolution and quantization
  selection, generalized from `qwen3-asr-rs::hub`.
- A `ModelSource` trait in `voxora-core` so each engine adapter
  receives a `ModelDir` (a path on disk) and never has to know whether
  the weights came from Hugging Face, a local copy, or a future
  vendor. See section 5 below for the rationale.
- Per-engine adapter crates (`voxora-whisper`, `voxora-qwen3asr`,
  future ones) that each implement the trait.
- A hardware dispatcher that picks CUDA → Metal → CPU at runtime.

## 5. Design principle: separate model acquisition from inference

The earliest draft of voxora put everything in one crate: the trait,
the engine adapters, and the Hugging Face downloader. After
discussion, we split acquisition from inference into two distinct
crates with a trait boundary between them.

### Why two responsibilities, not one

"Acquire a model from a remote source" and "run inference on a
local model" are two operations with different failure modes and
different users:

| | Acquisition (`voxora-hf`) | Inference (`voxora-core` + adapters) |
|---|---|---|
| Failure mode | network, DNS, HTTP 4xx/5xx, disk full, partial download | OOM, shape mismatch, NaN, bad audio |
| Determinism | no — same input, different bytes on different days | yes — same input → same output |
| User persona | one-time setup (`voxora download`) | every transcription |
| Threat model | untrusted remote, auth tokens, rate limits | trusted local file |
| Test cost | integration tests, fixtures, network mocks | unit tests, deterministic |

If both live in the same crate, the inference library inherits HF's
network dependency, its auth-token handling, its rate-limit logic,
and its `async_trait` requirements — for the 90% of consumers who
already have the model on disk. We split them so the inference
path stays **offline-pure**.

### How the split looks

```text
              ┌─────────────────────┐
              │ voxora-hf           │   ← network I/O, auth tokens,
              │ (HuggingFaceSource) │     HF Hub REST, cache, SHA256
              └──────────┬──────────┘
                         │ implements
                         ▼
              ┌─────────────────────┐
              │ voxora-core         │   ← offline-pure, no reqwest,
              │ ModelSource trait   │     no tokio required for use
              └──────────┬──────────┘
                         │ used by
        ┌────────────────┼─────────────────┐
        ▼                ▼                 ▼
  voxora-whisper   voxora-qwen3asr   voxora-parakeet (future)
        │                │                 │
        └────────────────┼─────────────────┘
                         ▼
              Application (Telora, voxora-cli, …)
```

The `ModelSource` trait is defined in `voxora-core` and implemented
in `voxora-hf`. A consumer does:

```rust
let source  = voxora_hf::HuggingFaceSource::new();   // optional
let opts    = voxora_core::ResolveOptions::default();
let model   = source.resolve("Qwen/Qwen3-ASR-0.6B", &opts).await?;
let engine  = voxora_qwen3asr::Qwen3AsrEngine::load(&model.path)?;
let result  = engine.transcribe(&samples, &opts)?;
```

Notice: the engine is constructed from `&model.path`, not from the
HF id. The engine does not know where the model came from. A future
`voxora_local::LocalSource` (path-based) or `voxora_modelscope::ModelScopeSource`
plugs in without changing a single engine adapter.

### Why a sibling crate, not a feature flag

We considered `voxora = { features = ["hf"] }` instead of a separate
crate. The crate split wins because:

1. **Compilation cost**. `voxora-hf` will pull in `reqwest`,
   `tokio`, `serde_json`, possibly `hf-hub`. If a downstream only
   wants inference, they should not pay that compile time.
2. **Versioning**. HF resolution is a moving target (new file
   formats, new quantization schemes). Iterating on it without
   bumping `voxora-core` is valuable.
3. **Tests stay fast**. `voxora-core` tests run with no network
   and no fixtures. The HF integration tests live in `voxora-hf`
   where they belong.

### Telora impact

Today `telora/telora-models/src/main.rs` ships its own HF downloader
(`telora-models list | download`). After voxora phase 6 that file
becomes a thin wrapper around `voxora_hf::HuggingFaceSource` — same
CLI surface, same UX, single source of truth for download logic.

The Telora maintainer's `telora.toml` does not change:

```toml
model_kind = "whisper" | "qwen3-asr"
model_id   = "Qwen/Qwen3-ASR-0.6B"   # or "ggerganov/whisper.cpp/ggml-base.bin"
```

What changes under the hood: the daemon's `BridgeTranscriber` calls
`source.resolve(model_id, &opts).await?` before constructing the
engine.

## 6. Strategic decision: build standalone, not as a transcribe-rs sibling

Three options were considered:

### A) Complementary to `transcribe-rs`

Build voxora as the "candle sibling" of `transcribe-rs`. Same trait
goal, different substrate (candle vs ORT). The two crates coexist;
eventually voxora could upstream a `candle-asr` backend into
`transcribe-rs`.

**Pros**: clean positioning; matches what the maintainer originally
hinted at in the PR #3509 comment
(*"Once this lands in candle, it enables a candle backend for
transcribe-rs"*).

**Cons**: conceptual duplication of two similar traits; if
`transcribe-rs` evolves (it already did — version 0.3.0 was a
breaking release), voxora gets dragged along or gets stuck.

### B) Standalone, reusing proven code ← **chosen**

Build voxora as a new repo with its own trait, owned end-to-end by
`airvzxf/voxora`. Reuse the **code** from the existing projects (the
Qwen3-ASR model implementation from PR #3509, the inference engine
from `qwen3-asr-rs`, the HF resolver from `qwen3-asr-rs::hub`, the
whisper-rs bindings already in Telora) without depending on them as
**upstream packages**.

**Pros**:
- 100% control of the roadmap and the release cadence.
- No external maintainer can block, slow down, or break the project.
- The brand stays consistent with Telora (same naming convention:
  `vox + ora = voxora`, `tele + ora = Telora`).
- Adoption goal is Telora first, broader community second.

**Cons**: no network effect from the `transcribe-rs` community (which
is fine — Telora is the primary consumer anyway).

### C) Replace / fork `transcribe-rs`

Propose to `cjpais` a merge, or fork `transcribe-rs` and add a candle
backend.

**Pros**: avoids ecosystem fragmentation.

**Cons**: months of social work; `transcribe-rs`'s code is tightly
coupled to ORT (its 9 engines all use `ort`); the maintainer
(`cjpais`) and `airvzxf` have no prior interactions visible in either
repo's issues.

**Why we chose B**: the maintainer explicitly asked for a project
*"creado de cero, sí, tomando en cuenta lo que se ha descubierto, pero
más bien creando todo de cero para no depender de upstream"* while
still reusing the **code** that has been validated by the PR #3509
benchmarks. That maps cleanly onto option B.

## 7. The trait we will implement

voxora exposes **two** traits in `voxora-core`:

1. `ModelSource` — answers *"where does the model live on disk?"*
2. `AsrEngine` — answers *"how do I transcribe audio with it?"*

The split mirrors the design principle in section 5: acquisition and
inference are separate concerns.

### `AsrEngine` — the inference trait

Modeled on `transcribe-rs::SpeechModel` for ergonomic symmetry, with
the `Send + Sync` supertrait that daemon-style consumers require:

```rust
pub trait AsrEngine: Send + Sync {
    fn capabilities(&self) -> ModelCapabilities;
    fn transcribe(
        &self,
        samples: &[f32],          // 16 kHz, mono, f32 ∈ [-1, 1]
        opts: &TranscribeOptions,
    ) -> Result<TranscriptionResult, AsrError>;
}

pub struct TranscribeOptions {
    pub language: Option<String>,
    pub translate: bool,
    pub leading_silence_ms: Option<u32>,
    pub trailing_silence_ms: Option<u32>,
}

pub struct TranscriptionResult {
    pub text: String,
    pub segments: Vec<TranscriptionSegment>,
    pub language: Option<String>,
}

pub struct ModelCapabilities {
    pub name: &'static str,        // e.g. "qwen3-asr-0.6b"
    pub engine_id: &'static str,   // e.g. "qwen3_asr"
    pub sample_rate: u32,          // 16000
    pub languages: &'static [&'static str],
    pub supports_timestamps: bool,
    pub supports_translation: bool,
    pub supports_streaming: bool,
}
```

The contract deliberately matches `transcribe-rs` field-for-field where
possible. If voxora ever needs to interoperate with a
`transcribe-rs` engine (e.g. by wrapping a Parakeet model via ORT),
the conversion will be mechanical.

### `ModelSource` — the acquisition trait

`ModelSource` is the new trait. It abstracts over Hugging Face,
local directories, and any future vendor.

```rust
#[async_trait::async_trait]
pub trait ModelSource: Send + Sync {
    /// Stable name of the source (e.g. "huggingface", "local").
    fn name(&self) -> &'static str;

    /// Resolve a model identifier into a local `ModelDir`.
    /// Implementations are responsible for downloading, caching,
    /// verifying integrity, and applying quantization if requested.
    async fn resolve(
        &self,
        model_id: &str,
        opts: &ResolveOptions,
    ) -> Result<ModelDir, AsrError>;

    /// Read the model's declared capabilities without downloading
    /// the weights (for `voxora info <model-id>`).
    async fn capabilities_for(
        &self,
        model_id: &str,
    ) -> Result<ModelCapabilities, AsrError>;

    /// Optional: list models available in this source.
    /// Default returns `AsrError::Unsupported`.
    async fn list_available(&self) -> Result<Vec<ModelDescriptor>, AsrError> {
        Err(AsrError::Unsupported("list_available"))
    }
}

#[derive(Debug, Clone)]
pub struct ModelDir {
    pub path: PathBuf,                 // absolute path to weights + config
    pub source: ModelSourceKind,       // provenance
    pub quantization: Quantization,    // what was actually loaded
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelSourceKind {
    HuggingFace { repo: String, revision: Option<String> },
    Local { path: PathBuf },
    // Future: ModelScope, Civitai, OCI artifact, …
}

#[derive(Debug, Clone, Default)]
pub struct ResolveOptions {
    pub cache_dir: Option<PathBuf>,            // default: $XDG_CACHE_HOME/voxora
    pub quantization: QuantizationPreference,  // default: Auto
    pub force_redownload: bool,                // default: false
    pub token: Option<String>,                 // override HF_TOKEN
}

#[derive(Debug, Clone, Default)]
pub enum QuantizationPreference {
    #[default]
    Auto,    // BF16 if CUDA/Metal, F32 if CPU, native weights otherwise
    Bf16,
    F16,
    F32,
    Q4_K,
    Q8_0,
}
```

### How the two traits compose

A consumer ties them together with two lines:

```rust
let source = voxora_hf::HuggingFaceSource::new();   // any ModelSource impl
let dir    = source.resolve("Qwen/Qwen3-ASR-0.6B", &ResolveOptions::default()).await?;
let engine = voxora_qwen3asr::Qwen3AsrEngine::load(&dir.path)?;
let result = engine.transcribe(&samples, &TranscribeOptions::default())?;
```

The engine never sees the HF id. It loads from a path on disk. A
test fixture that uses `voxora_local::LocalSource::new("/srv/models")`
passes the same `&dir.path` to the same engine — same code, no
special case.

## 8. Naming decision

**voxora** was selected from ~60 candidates across six categories:

| Category | Examples |
|---|---|
| Telora family | `telora-bridge`, `telora-core`, `telora-hub`, `telara` |
| Candle-native | `candle-asr`, `candle-stt` |
| Latin / Greek roots | `auralis`, `phonos`, `aurison`, `melora`, `phonora` |
| Toolkit convention | `asrkit`, `sttsuite`, `asrtoolkit` |
| Bridge / Hub literal | `asr-bridge`, `asr-hub`, `asr-fabric` |
| Tech-evocative single words | `echora`, `spectrum-asr`, `lyra-asr` |

Pre-flight availability check was performed against both
`airvzxf/<name>` on GitHub and `https://crates.io/api/v1/crates/<name>`
for every candidate.

`voxora` won on these criteria:

1. **Same construction as Telora** (`vox + ora` parallels `tele + ora`).
2. **Phonetic**: ends in the open vowel **ah**, the sound produced
   when opening the mouth to speak — metacognitive fit for a voice
   project.
3. **No hyphens, no `-rs`, no `-asr` suffix**: works as a Rust
   `use voxora::...` import, and the name describes the brand rather
   than the function.
4. **Free** on both GitHub (`airvzxf/voxora`) and crates.io
   (`voxora`) at the time of selection.

## 9. License decision

**Apache License, Version 2.0** (single license, not dual).

| License | Adopted? | Reason |
|---|---|---|
| MIT | considered | no patent grant; weaker than Apache for ML |
| **Apache-2.0** | **chosen** | patent grant matters in ML; matches `candle` (dual) and Hugging Face Transformers |
| Dual MIT/Apache | rejected | the user prefers single; patent grant makes it strictly more protective than MIT |
| AGPL-3 | rejected for the library | kills adoption; AGPL remains on Telora itself, where the copyleft guarantee matters |

Note: AGPL-3 on Telora (the daemon) is **compatible** with Apache-2.0
on voxora. AGPL-3 section 5 explicitly allows AGPL works to depend on
non-copyleft libraries without propagating copyleft to those libraries.
Telora will continue to be AGPL-3; voxora will be Apache-2.0.

## 10. Phased plan

See [`ROADMAP.md`](ROADMAP.md) for the detailed phase breakdown.
Short version:

| Phase | Goal |
|---|---|
| 0 | Repo scaffolding, docs ← this commit |
| 1 | `voxora-core`: `AsrEngine` + `ModelSource` traits, types, errors |
| 2 | `voxora-hf`: `HuggingFaceSource: ModelSource` |
| 3 | `voxora-whisper`: engine adapter over `whisper-rs` |
| 4 | `voxora-qwen3asr`: engine adapter over `qwen3-asr-rs` |
| 5 | `voxora-cli`: `voxora list / download / run` |
| 6 | Telora integration: `telora-models` refactored to wrap `voxora-hf` |

## 11. What we are explicitly NOT doing

- **Not** depending on `transcribe-rs` as an upstream package. The
  trait surface is intentionally compatible, but the implementation
  is independent.
- **Not** depending on `huggingface/candle` at the substrate level
  beyond what `qwen3-asr-rs` already needs. We will vendor or
  re-implement model code as needed; we will not block on PR #3509
  being merged.
- **Not** bundling Hugging Face downloads into `voxora-core`. The
  inference library stays offline-pure; HF resolution lives in
  `voxora-hf`. See section 5 for the rationale.
- **Not** building a model-training or fine-tuning pipeline. voxora
  is an inference layer.
- **Not** providing a cloud / remote backend in phase 0–5. Remote
  engines (OpenAI API, Cohere cloud, etc.) are a possible phase 7+.

---

*Document compiled from the maintainer's working session on
2026-07-09. Sources: `airvzxf/telora` git log, `huggingface/candle`
PR #3509 thread, `airvzxf/qwen3-asr-rs` working tree, and the
`cjpais/transcribe-rs` public README. Updated 2026-07-09 to record
the ModelSource / voxora-hf split.*