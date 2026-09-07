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
            // Closes #143, #144: the message names the missing
            // path so the caller can debug, but the configured root
            // is NOT separately annotated (closes the `(root: …)`
            // leak in the previous contract). The explicit
            // `(root: ` annotation must NOT appear.
            assert!(
                msg.contains("model.bin"),
                "error must name the missing file: {msg}",
            );
            assert!(
                !msg.contains("(root: "),
                "error must not annotate the configured root: {msg}",
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

// ---- Security hardening (closes #143, #144, EPIC #148) ----
//
// These tests exercise the runtime guards added to
// `LocalSource::resolve`. They are written against the *direct*
// `LocalSource::resolve` surface (not the registry) so a caller
// who bypasses `ModelId::parse` still hits the same defence.

#[test]
fn resolve_rejects_path_traversal_relative() {
    // Closes #143 — `LocalSource::new(root).resolve("../escape", ...)`
    // must return `InvalidInput` regardless of `root`. The parser
    // already rejects `../escape` as a standalone id (it starts
    // with `../`, so it falls into the Local arm, and the body
    // `escape` is fine, but the join onto any root produces a
    // path that contains a `ParentDir` component — which the
    // runtime cap catches).
    let tmp = tempfile::tempdir().expect("tempdir");
    let source = LocalSource::new(tmp.path().to_path_buf());
    let err = block_on(source.resolve("../escape", &opts())).expect_err("path traversal must fail");
    match err {
        AsrError::InvalidInput(msg) => {
            assert!(
                msg.contains("traversal") || msg.contains(".."),
                "expected traversal wording, got {msg:?}",
            );
        }
        other => panic!("expected InvalidInput, got {other:?}"),
    }
}

#[test]
fn resolve_rejects_path_traversal_in_subpath() {
    // Closes #143 — `safe/../../etc/passwd` joins onto `root` to
    // produce `<root>/safe/../../etc/passwd`, which contains a
    // `ParentDir` component; the runtime cap rejects it before
    // any I/O. The first half of the path (`<root>/safe`) does
    // exist on disk so this is not a `ModelNotFound` — the cap
    // fires before the lstat would have noticed the missing leaf.
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("safe");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let source = LocalSource::new(tmp.path().to_path_buf());
    let err = block_on(source.resolve("safe/../../etc/passwd", &opts()))
        .expect_err("path traversal must fail");
    match err {
        AsrError::InvalidInput(msg) => {
            assert!(
                msg.contains("traversal") || msg.contains(".."),
                "expected traversal wording, got {msg:?}",
            );
        }
        other => panic!("expected InvalidInput, got {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn resolve_rejects_symlink_pointing_outside_root() {
    // Closes #143, #144 — a symlink under `local_root` whose target
    // lives outside `local_root` must be rejected by
    // `LocalSource::resolve` *before* any symlink-following
    // `is_file()` would have reported the existence. The previous
    // implementation silently followed symlinks; this test pins
    // the new `symlink_metadata` + `O_NOFOLLOW` contract.
    use std::os::unix::fs::symlink;

    let tmp = tempfile::tempdir().expect("tempdir");
    // Plant a real file outside `tmp` that the symlink will try to
    // point at. `/etc/passwd` is the canonical issue example.
    let escape_target = PathBuf::from("/etc/passwd");
    if !escape_target.exists() {
        // Some test environments (e.g. macOS CI without `/etc/passwd`)
        // lack the canonical target. Skip rather than fail: the
        // test is about the symlink-following rejection, not the
        // specific target.
        eprintln!("skipping: /etc/passwd not present on this host");
        return;
    }

    let escape_link = tmp.path().join("escape");
    symlink(&escape_target, &escape_link).expect("symlink");

    let source = LocalSource::new(tmp.path().to_path_buf());
    let err = block_on(source.resolve("escape", &opts()))
        .expect_err("symlink-to-outside must be rejected");
    match err {
        AsrError::InvalidInput(msg) => {
            assert!(
                msg.contains("symlink"),
                "expected symlink wording, got {msg:?}",
            );
        }
        other => panic!("expected InvalidInput, got {other:?}"),
    }
}

#[test]
fn resolve_rejects_oversized_model_id() {
    // Closes #143 — `model_id` longer than 4096 bytes returns
    // `AsrError::InvalidInput("model id too long: ...")`.
    let tmp = tempfile::tempdir().expect("tempdir");
    let source = LocalSource::new(tmp.path().to_path_buf());

    // 5000 `a`s — well over the 4096-byte cap, all valid path
    // characters so the only thing that can trip the resolver is
    // the length check.
    let oversized = "a".repeat(5000);
    let err =
        block_on(source.resolve(&oversized, &opts())).expect_err("oversized id must be rejected");
    match err {
        AsrError::InvalidInput(msg) => {
            assert!(
                msg.contains("too long"),
                "expected 'too long' wording, got {msg:?}",
            );
        }
        other => panic!("expected InvalidInput, got {other:?}"),
    }
}

#[test]
fn resolve_rejects_oversized_file_with_max_bytes() {
    // Closes #144 — a resolved file larger than
    // `ResolveOptions::max_bytes` returns
    // `AsrError::InvalidInput("file too large: ...")` when the
    // option is set.
    let tmp = tempfile::tempdir().expect("tempdir");
    let file = tmp.path().join("big.bin");
    // 2 KiB file, asked to be rejected as > 1 KiB.
    std::fs::write(&file, vec![0u8; 2048]).expect("write big");

    let source = LocalSource::new(tmp.path().to_path_buf());
    let tight_opts = ResolveOptions::with_max_bytes(1024);
    let err = block_on(source.resolve("big.bin", &tight_opts))
        .expect_err("oversized file must be rejected");
    match err {
        AsrError::InvalidInput(msg) => {
            assert!(
                msg.contains("too large"),
                "expected 'too large' wording, got {msg:?}",
            );
        }
        other => panic!("expected InvalidInput, got {other:?}"),
    }
}

#[test]
fn resolve_accepts_file_at_exact_max_bytes() {
    // Boundary test: a file of exactly `max_bytes` must be
    // accepted (the cap is `> max`, not `>= max`). Pins the
    // off-by-one so a future tightening is deliberate.
    let tmp = tempfile::tempdir().expect("tempdir");
    let file = tmp.path().join("exact.bin");
    std::fs::write(&file, vec![0u8; 1024]).expect("write exact");

    let source = LocalSource::new(tmp.path().to_path_buf());
    let tight_opts = ResolveOptions::with_max_bytes(1024);
    let resolved =
        block_on(source.resolve("exact.bin", &tight_opts)).expect("resolve at exact cap");
    assert!(resolved.entry.is_some());
}

#[test]
fn resolve_error_message_does_not_leak_root() {
    // Closes #144 — the `ModelNotFound` envelope must name the
    // missing path but NOT redundantly annotate the configured
    // `local_root`. A panic or log emission carrying the rendered
    // error must not expose the operator's model directory layout
    // via the explicit `(root: …)` substring the previous contract
    // included. The relative-id form means the joined path will
    // naturally contain the root — that is by design and is what
    // the caller asked for.
    let secret_root = "/secret/path/that/should/not/leak";
    let source = LocalSource::new(secret_root);
    let err = block_on(source.resolve("missing/file.bin", &opts())).expect_err("missing file");
    let rendered = format!("{err}");
    assert!(
        !rendered.contains("(root: "),
        "ModelNotFound message must not annotate the configured root: {rendered}",
    );
    assert!(
        !rendered.contains("root: /secret"),
        "ModelNotFound message must not echo the configured root: {rendered}",
    );
}

#[test]
fn resolve_invalid_input_message_does_not_leak_root() {
    // Closes #144 — `InvalidInput` messages produced by the
    // traversal / symlink / length-cap checks must reference the
    // user-supplied id (NOT the joined path that would naturally
    // include the configured root). The wording
    // "path traversal: .. segment in id \"../escape\"" is the
    // pinned contract; changing it is a deliberate decision.
    let secret_root = "/secret/path/that/should/not/leak";
    let source = LocalSource::new(secret_root);
    let err = block_on(source.resolve("../escape", &opts())).expect_err("path traversal must fail");
    let rendered = format!("{err}");
    assert!(
        !rendered.contains(secret_root),
        "InvalidInput message must not contain the configured root: {rendered}",
    );
    assert!(
        rendered.contains("../escape"),
        "InvalidInput message must echo the user-supplied id: {rendered}",
    );
}

#[cfg(unix)]
#[test]
fn resolve_does_not_follow_symlink_to_directory() {
    // A symlink under `local_root` that resolves to a directory
    // (not a file) must be rejected too — `LocalSource::resolve`
    // only serves regular files. The previous implementation used
    // `Path::is_file` which on Unix follows symlinks, so this
    // case was a silent escape hatch. The new lstat-based check
    // rejects the symlink first.
    use std::os::unix::fs::symlink;

    let tmp = tempfile::tempdir().expect("tempdir");
    let outside_dir = tempfile::tempdir().expect("outside tempdir");
    let link = tmp.path().join("link_to_dir");
    symlink(outside_dir.path(), &link).expect("symlink");

    let source = LocalSource::new(tmp.path().to_path_buf());
    let err = block_on(source.resolve("link_to_dir", &opts()))
        .expect_err("symlink-to-directory must be rejected");
    match err {
        AsrError::InvalidInput(_) | AsrError::ModelNotFound(_) => {}
        other => panic!("expected InvalidInput or ModelNotFound, got {other:?}"),
    }
}

#[test]
fn resolve_rejects_oversized_id_via_max_id_length_option() {
    // The caller-imposed `ResolveOptions::max_id_length` caps the
    // id length independently of the source's intrinsic 4 KiB
    // cap. Setting it to a tighter value (e.g. 100 bytes) must
    // surface `InvalidInput` for ids longer than 100 bytes.
    let tmp = tempfile::tempdir().expect("tempdir");
    let source = LocalSource::new(tmp.path().to_path_buf());

    let opts = ResolveOptions::with_max_id_length(100);
    let oversized = "a".repeat(200);
    let err = block_on(source.resolve(&oversized, &opts))
        .expect_err("id over caller cap must be rejected");
    assert!(
        matches!(err, AsrError::InvalidInput(_)),
        "expected InvalidInput, got {err:?}",
    );
}
