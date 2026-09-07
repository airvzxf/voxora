//! Integration tests for `voxora-registry::RegistryHfExt`'s
//! `with_builtin_descriptors_and_chained_source` helper (closes #120).
//!
//! These tests pin four behaviours of the new Local-id accept arm:
//!
//! 1. When a Local-rooted file is present, the chain's
//!    [`voxora_local::LocalSource`] wins and the resolved
//!    `ModelDir.kind` is `Local`.
//! 2. When the Local-rooted file is absent, the chain falls through
//!    to the Hugging Face arm and `kind` is `HuggingFace`.
//! 3. A 3-segment Whisper HF id still resolves through the chain
//!    unchanged (regression guard for the existing
//!    `with_builtin_descriptors()` flow).
//! 4. The production helper builds a registry with all three
//!    descriptors (Whisper HF + Qwen HF + Local fallback) registered
//!    in the documented order.
//!
//! ## Why a hand-rolled `FakeHFSource` instead of `wiremock`
//!
//! `wiremock` is the listed dev-dependency, but it cannot easily
//! stub the chained helper's HTTP flow: the helper builds its own
//! `HuggingFaceSource::new()` internally — a new client pointing at
//! `https://huggingface.co` regardless of any test harness URL.
//! Pointing the helper at a mock would require either a builder
//! overload that exposes the URL or a `chained_registry_with_source`
//! overload that takes pre-built sources; both are wider scope than
//! this issue. The test instead wires the registry's source field
//! directly via `Registry::with_source(ChainedSource::new(...))` and
//! substitutes a `FakeHFSource` for the chain's fallback slot.
//! This still exercises the new accept arm (the Local fallback
//! descriptor registered by the helper) and the chain's
//! fallthrough semantics — the only thing the `FakeHFSource` skips
//! is the bytes-on-the-wire round-trip, which `voxora-hf`'s own
//! test suite already covers.

#![cfg(all(feature = "hf", feature = "local"))]

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use voxora_engine::EngineFamily;
use voxora_local::{ChainedSource, LocalSource};
use voxora_registry::{
    ModelId, Registry, RegistryError, RegistryHfExt, builtin_local_descriptor,
    builtin_qwen3asr_descriptor, builtin_whisper_descriptor,
};
use voxora_traits::{
    AsrError, ModelCapabilities, ModelDir, ModelSource, ModelSourceKind, Quantization,
    ResolveOptions,
};

/// `ModelSource` that returns `ModelNotFound` for every id. Used
/// to drive the chain's HF arm without a network round-trip.
#[derive(Clone)]
struct MissingHFSource;

#[async_trait]
impl ModelSource for MissingHFSource {
    fn name(&self) -> &'static str {
        "missing-hf"
    }

    async fn resolve(&self, model_id: &str, _opts: &ResolveOptions) -> Result<ModelDir, AsrError> {
        Err(AsrError::ModelNotFound(format!(
            "test stub: no HF backend wired for {model_id:?}"
        )))
    }

    async fn capabilities_for(&self, _model_id: &str) -> Result<ModelCapabilities, AsrError> {
        Ok(ModelCapabilities::UNKNOWN)
    }
}

/// `ModelSource` that resolves every id to a deterministic
/// `ModelDir`. Used as the chain's fallback when the test needs the
/// HF arm to win.
#[derive(Clone)]
struct FakeHFSource {
    cache_root: PathBuf,
}

impl FakeHFSource {
    fn new(cache_root: PathBuf) -> Self {
        Self { cache_root }
    }
}

#[async_trait]
impl ModelSource for FakeHFSource {
    fn name(&self) -> &'static str {
        "fake-hf"
    }

    async fn resolve(&self, model_id: &str, _opts: &ResolveOptions) -> Result<ModelDir, AsrError> {
        let dir = self.cache_root.join(model_id.replace('/', "_"));
        Ok(ModelDir::with_entry(
            dir.clone(),
            dir.join("model.bin"),
            ModelSourceKind::HuggingFace,
            Quantization::F16,
        ))
    }

    async fn capabilities_for(&self, _model_id: &str) -> Result<ModelCapabilities, AsrError> {
        Ok(ModelCapabilities::UNKNOWN)
    }
}

/// Build a registry with the HF built-ins + Local fallback descriptor
/// AND a chain whose primary is `LocalSource::new(local_root)` and
/// whose fallback is `fallback`. This is the same recipe
/// `Registry::with_builtin_descriptors_and_chained_source` uses,
/// except we substitute `fallback` for the helper's auto-built
/// `HuggingFaceSource` so the test can run without network access.
fn registry_with_chain(local_root: PathBuf, fallback: Arc<dyn ModelSource>) -> Registry {
    let local = Arc::new(LocalSource::new(local_root));
    let chain: Arc<dyn ModelSource> = Arc::new(ChainedSource::new(local, fallback));
    Registry::new(Arc::new(MissingHFSource))
        .register(builtin_whisper_descriptor())
        .register(builtin_qwen3asr_descriptor())
        .register(builtin_local_descriptor(EngineFamily::Whisper))
        .with_source(chain)
}

#[tokio::test]
async fn local_present_file_resolves_via_local_source() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let id_str = "Qwen/Qwen3-ASR-0.6B";
    let local_file = tmp.path().join(id_str);
    std::fs::create_dir_all(local_file.parent().expect("parent")).expect("mkdir");
    std::fs::write(&local_file, b"weights").expect("write local file");

    let hf_cache = tempfile::tempdir().expect("hf tempdir");
    let registry = registry_with_chain(
        tmp.path().to_path_buf(),
        Arc::new(FakeHFSource::new(hf_cache.path().to_path_buf())),
    );

    let id = ModelId::parse(id_str).expect("parse");
    let resolved = registry
        .resolve(&id, &ResolveOptions::default())
        .await
        .expect("resolve");

    // LocalSource won: kind is Local and entry is the on-disk file.
    assert_eq!(resolved.model_dir.kind, ModelSourceKind::Local);
    assert_eq!(
        resolved.model_dir.entry.as_ref(),
        Some(&local_file),
        "LocalSource entry must name the on-disk file"
    );
    // The id is HF-shaped so the Qwen3-ASR descriptor wins the
    // accept arm — NOT the Local fallback descriptor (which would
    // match SourceKind::Local only). The chain's primary
    // (LocalSource) is the one that produced the file.
    assert_eq!(resolved.descriptor.family, EngineFamily::Qwen3Asr);
}

#[tokio::test]
async fn local_absent_file_falls_through_to_hf() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let id_str = "Qwen/Qwen3-ASR-0.6B";
    // Deliberately do NOT create the file — LocalSource must miss.
    assert!(!tmp.path().join(id_str).exists());

    let hf_cache = tempfile::tempdir().expect("hf tempdir");
    let fake_hf = Arc::new(FakeHFSource::new(hf_cache.path().to_path_buf()));
    let registry = registry_with_chain(
        tmp.path().to_path_buf(),
        fake_hf.clone() as Arc<dyn ModelSource>,
    );

    let id = ModelId::parse(id_str).expect("parse");
    let resolved = registry
        .resolve(&id, &ResolveOptions::default())
        .await
        .expect("resolve via chain fallthrough");

    // The chain fell through to the FakeHF arm: kind is
    // HuggingFace, entry is the fake's synthesised path under
    // hf_cache. (We avoid asserting the entry path verbatim — the
    // FakeHFSource's path-join is implementation detail; the
    // important contract is `kind == HuggingFace`.)
    assert_eq!(resolved.model_dir.kind, ModelSourceKind::HuggingFace);
    assert!(
        resolved.model_dir.entry.is_some(),
        "FakeHFSource must populate `entry` so the descriptor-arm fallthrough is meaningful"
    );
    assert_eq!(resolved.descriptor.family, EngineFamily::Qwen3Asr);
}

#[tokio::test]
async fn hf_only_id_still_resolves_unchanged() {
    let tmp = tempfile::tempdir().expect("local tempdir");
    let hf_cache = tempfile::tempdir().expect("hf tempdir");

    // 3-segment Whisper HF id: the Whisper descriptor wins the
    // accept arm. LocalSource misses (no file under local_root with
    // this shape), so the chain falls through to the FakeHF arm.
    let id_str = "ggerganov/whisper.cpp/ggml-tiny.bin";
    let fake_hf = Arc::new(FakeHFSource::new(hf_cache.path().to_path_buf()));
    let registry = registry_with_chain(
        tmp.path().to_path_buf(),
        fake_hf.clone() as Arc<dyn ModelSource>,
    );

    let id = ModelId::parse(id_str).expect("parse");
    let resolved = registry
        .resolve(&id, &ResolveOptions::default())
        .await
        .expect("resolve via HF arm");

    assert_eq!(resolved.model_dir.kind, ModelSourceKind::HuggingFace);
    assert_eq!(resolved.descriptor.family, EngineFamily::Whisper);
    // Regression guard: a 3-segment HF id must end up on the HF arm
    // even with the Local fallback descriptor registered.
    assert!(
        resolved
            .model_dir
            .entry
            .as_ref()
            .is_some_and(|e| e.starts_with(hf_cache.path())),
        "3-segment HF id must resolve through the FakeHF arm under hf_cache, got {:?}",
        resolved.model_dir.entry,
    );
}

#[tokio::test]
async fn chained_helper_builds_registry_with_all_three_descriptors() {
    // Smoke test for the convenience helper itself: builds a
    // registry with the helper and verifies the descriptor list
    // shape. We do NOT call `resolve` on it — the helper wires a
    // real `HuggingFaceSource` (production URL), which would hit
    // the wire on the first miss. The descriptor shape is the
    // contract this test pins.
    let tmp = tempfile::tempdir().expect("local tempdir");
    let registry = Registry::new(Arc::new(MissingHFSource))
        .with_builtin_descriptors_and_chained_source(tmp.path().to_path_buf())
        .expect("chained helper");

    assert_eq!(
        registry.descriptors().len(),
        3,
        "chained helper must register Whisper HF + Qwen HF + Local fallback"
    );

    // The Local fallback descriptor must be last (so the existing
    // first-match-wins accept order is preserved for HF ids).
    let last = registry.descriptors().last().expect("last descriptor");
    let local_id = ModelId::parse("/srv/models/anything.bin").expect("parse local");
    assert!(
        (last.accepts)(&local_id),
        "Local fallback descriptor must match any SourceKind::Local id"
    );

    // Sanity: the first descriptor still matches a Whisper HF id.
    let hf_id = ModelId::parse("ggerganov/whisper.cpp/ggml-tiny.bin").expect("parse Whisper HF");
    assert!(
        (registry.descriptors()[0].accepts)(&hf_id),
        "first descriptor must still be the Whisper HF descriptor"
    );
}

// ---- Security hardening (closes #143, EPIC #148) ----
//
// These tests pin the end-to-end behaviour of the parser +
// chained-helper source stack against hostile Local ids. They
// use the same `registry_with_chain` recipe as the positive tests
// above so the wire-up matches production and we do NOT need a
// real `HuggingFaceSource::new()` (which would hit the network).

#[tokio::test]
async fn chained_helper_rejects_hostile_local_id() {
    // #143, EPIC #148 — `ModelId::parse("/etc/passwd")` is parsed
    // as Local. The Local fallback descriptor accepts the id. The
    // chained helper routes it through `LocalSource::resolve`,
    // which now enforces a containment guard (absolute ids must
    // start with the configured `local_root`). `/etc/passwd` is
    // outside `tmp.path()`, so the resolve returns
    // `AsrError::InvalidInput` — surfaced through the registry as
    // `RegistryError::Parse`.
    let tmp = tempfile::tempdir().expect("local tempdir");
    let hf_cache = tempfile::tempdir().expect("hf tempdir");

    let fake_hf = Arc::new(FakeHFSource::new(hf_cache.path().to_path_buf()));
    let registry = registry_with_chain(
        tmp.path().to_path_buf(),
        fake_hf.clone() as Arc<dyn ModelSource>,
    );

    let id = ModelId::parse("/etc/passwd").expect("absolute Local id parses");
    let err = registry
        .resolve(&id, &ResolveOptions::default())
        .await
        .expect_err("absolute id outside local_root must be rejected");
    match err {
        RegistryError::Parse(msg) => {
            // The registry wraps the source's `AsrError::InvalidInput`
            // as `Parse("source resolve: invalid input: …")`. The
            // containment-guard wording is the pinned contract.
            assert!(
                msg.contains("configured root") || msg.contains("outside"),
                "expected containment-guard wording, got {msg:?}",
            );
        }
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn parser_rejects_traversal_local_id_before_chain() {
    // #143, EPIC #148 — the parser rejects a `..` subpath
    // upstream, before any source sees it. This is the defence-
    // in-depth contract: `registry.resolve(&hostile_id, …)` would
    // also fail (because the id is malformed), but a direct
    // `LocalSource::resolve` would too — and a direct caller of
    // `ModelId::parse` would fail FIRST. Pin the order: the parse
    // must fail.
    for hostile in [
        "/safe/../etc/passwd",
        "/srv/models/../../etc/passwd",
        "./foo/../bar",
    ] {
        let err = ModelId::parse(hostile).expect_err(&format!("must reject: {hostile:?}"));
        match err {
            RegistryError::Parse(msg) => {
                assert!(
                    msg.contains("traversal") || msg.contains(".."),
                    "expected traversal wording for {hostile:?}, got {msg:?}",
                );
            }
            other => panic!("expected Parse, got {other:?} for {hostile:?}"),
        }
    }
}
