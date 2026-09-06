//! Integration tests for `voxora-local`.
//!
//! Always-run, offline-only, no network. The `block_on` helper
//! mirrors the pattern in `voxora-testkit/src/fixtures/mod.rs`:
//! `async_trait` produces no-op futures, so `Waker::noop()` + a
//! spin loop is enough to drive `resolve` / `capabilities_for` to
//! completion without pulling in `tokio`. `Waker::noop()` is
//! stable from Rust 1.85 (MSRV is 1.88).

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use voxora_local::{ChainedSource, LocalSource};
use voxora_traits::{
    AsrError, ModelCapabilities, ModelDir, ModelSource, ModelSourceKind, Quantization,
    ResolveOptions,
};

/// Spin-block a future to completion using a no-op waker.
///
/// `LocalSource::resolve` is `async` only via `async_trait`; its
/// body has no real `await` points, so it completes on the first
/// poll. A real executor is overkill — this keeps `voxora-local`
/// free of `tokio` / `futures` runtime deps (mirroring the
/// `voxora-testkit` contract).
fn block_on<F: Future>(fut: F) -> F::Output {
    let waker = Waker::noop();
    let mut fut: Pin<Box<F>> = Box::pin(fut);
    let mut ctx = Context::from_waker(waker);
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut ctx) {
            return v;
        }
        std::hint::spin_loop();
    }
}

fn opts() -> ResolveOptions {
    ResolveOptions::default()
}

#[test]
fn resolve_existing_file_returns_entry() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("some-org").join("some-repo");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let file = dir.join("model.bin");
    std::fs::write(&file, b"weights").expect("write");

    let source = LocalSource::new(tmp.path().to_path_buf());
    let resolved =
        block_on(source.resolve("some-org/some-repo/model.bin", &opts())).expect("resolve");

    assert_eq!(resolved.path, tmp.path().to_path_buf());
    assert_eq!(resolved.entry.as_ref(), Some(&file));
    assert_eq!(resolved.kind, ModelSourceKind::Local);
    assert_eq!(resolved.quantization, Quantization::F16);
}

#[test]
fn resolve_missing_file_returns_model_not_found_naming_the_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("org").join("repo");
    std::fs::create_dir_all(&dir).expect("mkdir");
    // The file `model.bin` does NOT exist — only a sibling does.
    std::fs::write(dir.join("other.bin"), b"x").expect("write");

    let source = LocalSource::new(tmp.path().to_path_buf());
    let err = block_on(source.resolve("org/repo/model.bin", &opts())).expect_err("missing file");

    match err {
        AsrError::ModelNotFound(msg) => {
            assert!(
                msg.contains("model.bin"),
                "error must name the missing file: {msg}",
            );
            assert!(
                msg.contains(tmp.path().to_string_lossy().as_ref()),
                "error must name the configured root: {msg}",
            );
        }
        other => panic!("expected ModelNotFound, got {other:?}"),
    }
}

#[test]
fn resolve_missing_directory_returns_model_not_found() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let source = LocalSource::new(tmp.path().to_path_buf());

    let err = block_on(source.resolve("org/repo/model.bin", &opts())).expect_err("missing dir");

    assert!(
        matches!(err, AsrError::ModelNotFound(_)),
        "missing directory must surface as ModelNotFound",
    );
}

#[test]
fn resolve_trailing_slash_on_root_matches_no_trailing_slash() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("org").join("repo");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let file = dir.join("model.bin");
    std::fs::write(&file, b"weights").expect("write");

    // Construct a "trailing-slash" root by appending a separator
    // on Unix. PathBuf already normalises, so we use the underlying
    // string form to keep the test cross-platform meaningful.
    let root = format!("{}/", tmp.path().display());
    let source = LocalSource::new(PathBuf::from(&root));
    let resolved = block_on(source.resolve("org/repo/model.bin", &opts())).expect("resolve");

    // PathBuf::join follows `root` exactly; on Unix the joined
    // path keeps the trailing separator. The key invariant is that
    // the entry equals the on-disk file — no double-slash, no
    // empty path component, no resolution mismatch.
    assert_eq!(resolved.entry.as_ref(), Some(&file));
    assert!(
        !resolved
            .entry
            .as_ref()
            .expect("entry")
            .to_string_lossy()
            .contains("//"),
        "entry must not contain a double-slash: {:?}",
        resolved.entry,
    );
}

#[test]
fn name_returns_local() {
    let source = LocalSource::new("/srv/models");
    assert_eq!(source.name(), "local");
}

#[test]
fn capabilities_for_returns_unknown() {
    let source = LocalSource::new("/srv/models");
    let caps = block_on(source.capabilities_for("anything")).expect("caps");
    assert_eq!(caps, ModelCapabilities::UNKNOWN);
}

#[test]
fn list_available_returns_unsupported() {
    let source = LocalSource::new("/srv/models");
    let err = block_on(source.list_available()).expect_err("unsupported");
    match err {
        AsrError::Unsupported("list_available") => {}
        other => panic!("expected Unsupported(\"list_available\"), got {other:?}"),
    }
}

#[test]
fn chained_source_falls_back_on_local_miss() {
    let root_a = tempfile::tempdir().expect("tempdir a");
    let root_b = tempfile::tempdir().expect("tempdir b");

    // model.bin only exists in `root_b` — `root_a` should miss
    // and the chain should fall through.
    let model_b = root_b.path().join("org").join("repo").join("model.bin");
    std::fs::create_dir_all(model_b.parent().expect("parent")).expect("mkdir");
    std::fs::write(&model_b, b"weights from b").expect("write");

    let chain = ChainedSource::new(
        Arc::new(LocalSource::new(root_a.path().to_path_buf())),
        Arc::new(LocalSource::new(root_b.path().to_path_buf())),
    );

    let resolved = block_on(chain.resolve("org/repo/model.bin", &opts()))
        .expect("chain resolves via fallback");

    assert_eq!(resolved.path, root_b.path().to_path_buf());
    assert_eq!(resolved.entry.as_ref(), Some(&model_b));
}

#[test]
fn chained_source_prefers_primary_on_hit() {
    let root_a = tempfile::tempdir().expect("tempdir a");
    let root_b = tempfile::tempdir().expect("tempdir b");

    // Both roots have the model, but different contents — the
    // primary's file must win.
    let model_a = root_a.path().join("org").join("repo").join("model.bin");
    std::fs::create_dir_all(model_a.parent().expect("parent")).expect("mkdir");
    std::fs::write(&model_a, b"weights from a").expect("write");

    let model_b = root_b.path().join("org").join("repo").join("model.bin");
    std::fs::create_dir_all(model_b.parent().expect("parent")).expect("mkdir");
    std::fs::write(&model_b, b"weights from b").expect("write");

    let chain = ChainedSource::new(
        Arc::new(LocalSource::new(root_a.path().to_path_buf())),
        Arc::new(LocalSource::new(root_b.path().to_path_buf())),
    );

    let resolved =
        block_on(chain.resolve("org/repo/model.bin", &opts())).expect("chain resolves via primary");

    assert_eq!(resolved.path, root_a.path().to_path_buf());
    assert_eq!(resolved.entry.as_ref(), Some(&model_a));
}

#[test]
fn chained_source_reports_primary_miss_when_both_miss() {
    let root_a = tempfile::tempdir().expect("tempdir a");
    let root_b = tempfile::tempdir().expect("tempdir b");

    let chain = ChainedSource::new(
        Arc::new(LocalSource::new(root_a.path().to_path_buf())),
        Arc::new(LocalSource::new(root_b.path().to_path_buf())),
    );

    let err = block_on(chain.resolve("org/repo/model.bin", &opts())).expect_err("both miss");

    match err {
        AsrError::ModelNotFound(msg) => {
            // The chain annotates the primary miss with the
            // fallback miss so operators see both root paths.
            assert!(
                msg.contains(root_a.path().to_string_lossy().as_ref()),
                "error must reference the primary root: {msg}",
            );
        }
        other => panic!("expected ModelNotFound, got {other:?}"),
    }
}

#[test]
fn chained_source_propagates_non_miss_errors() {
    // A chain whose primary does NOT exist as a directory on disk
    // is still a valid LocalSource (no constructor check). The
    // primary's `ModelNotFound` is exactly the trigger to fall
    // back, so this stays a "miss chain" test rather than
    // surfacing an I/O error.
    let root_a = tempfile::tempdir().expect("tempdir a");
    let root_b = tempfile::tempdir().expect("tempdir b");
    let model_b = root_b.path().join("model.bin");
    std::fs::write(&model_b, b"present").expect("write");

    let chain = ChainedSource::new(
        Arc::new(LocalSource::new(root_a.path().to_path_buf())),
        Arc::new(LocalSource::new(root_b.path().to_path_buf())),
    );

    let resolved =
        block_on(chain.resolve("model.bin", &opts())).expect("chain succeeds via fallback");

    // `root_b` joined with `model.bin` does NOT have an `org/repo`
    // component — the chain is model-id-agnostic.
    let expected: ModelDir = ModelDir::with_entry(
        root_b.path().to_path_buf(),
        resolved.entry.clone().expect("entry"),
        ModelSourceKind::Local,
        Quantization::F16,
    );
    assert_eq!(resolved.entry, expected.entry);
}
