# voxora — agent instructions

## Project

`voxora` is a workspace of 11 crates that implement a model-agnostic
ASR bridge (whisper.cpp + candle/qwen3-asr). The workspace is
published to crates.io per-crate; telora is the canonical consumer.
The canonical home for the public API surface is `voxora-traits`;
`voxora-bridge` is the umbrella crate that re-exports it together
with the engines.

## Stack

- Rust stable 1.86+, edition 2024 (MSRV pinned by the workspace
  `Cargo.toml` `rust-version` field; bump it together with the
  `rust-toolchain.toml` channel).
- `rust-toolchain.toml` pins the channel to 1.98.1; bump it
  together with the MSRV in the workspace `Cargo.toml`.
- Workspace: 11 crates (9 publishable; `voxora-cli` and
  `voxora-testkit` are `publish = false`). Per-crate `publish` flags.
- ASR-specific: no generic LLM/vision/multimodal traits.

## Coding conventions

- All code, comments, and documentation in English.
- `Result<T, E>` everywhere.
- `#![forbid(unsafe_code)]` at each `lib.rs`.
- `#![warn(missing_docs)]` on crates with public APIs.
- `#[non_exhaustive]` on public structs/enums that may grow.
- `rustfmt.toml` pins two settings, both load-bearing:
  - `reorder_imports = false` — rustfmt 1.9+ started alphabetising
    cross-crate `use` statements, which is what broke PR #66's
    fmt-check (a single-name `use voxora_engine::EngineFamily;`
    sitting above a brace-group `use voxora_traits::{…};` got
    reordered; see #77). Writing `use` in the order that reads
    naturally is the desired style.
  - `style_edition = "2024"` — freezes rustfmt's rewrap rules
    against binary-version drift, so a future rustfmt cannot
    silently rewrap long `use voxora_traits::{…}` blocks in
    unrelated code (see #86).
  Do not reorder imports in follow-up commits.

## Commit policy

- GPG-signed commits are mandatory (`commit.gpgsign = true`).
- Conventional commits: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `ci`, `build`, `perf`.
- One logical change per commit.
- No `git commit --amend`. No `git push --force`. No `--no-gpg-sign`.

## Version coordination

Voxora follows a **coordinated bump** policy for every
breaking release. When a release ships as `voxora X.Y.0`:

- Every workspace crate that **participates** in the release (i.e.
  was modified, or whose public surface is documented as part of
  the release) ships at `X.Y.0`.
- Crates that did not change AND are not in the release notes
  stay at their current version.
- Add a one-line entry to each affected crate's CHANGELOG
  documenting the bump rationale, even if the change is purely a
  version-pin update.

This produces two benefits:

1. **No version-narrative confusion.** A user reading the
   `voxora X.Y.0` GitHub Release page sees consistent versions
   across all participating crates.
2. **No "research burden" on consumers.** A consumer who writes
   `voxora-traits = "X.Y.0", voxora-engine = "X.Y.0", ...` in
   their `Cargo.toml` gets the expected matching set.

The exception case is documented: if a change is purely
additive and the maintainer elects not to bump, the rationale
must be recorded in a CHANGELOG entry.

## Validation tiers

The dev loop splits validation into tiers:

| Tier | Cost | Where | What |
|---|---|---|---|
| T0 | <30 s | pre-commit | `cargo fmt --all --check` + tracked-artifact guard (`git ls-files \| grep -E '(^|/)CACHEDIR\.TAG$\|(^|/)\.(rustc_info\|rustdoc_fingerprint)\.json$\|(^|/)(target\|\.cargo-target\|\.worktrees)/'`; fails if any tracked path matches a build-output directory name or cargo's content markers — closes #99). |
| T1 | <90 s | pre-commit | `cargo clippy --workspace --all-targets -- -D warnings` |
| T2 | 1–5 min | pre-push | `cargo test --workspace --all-targets` + `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace` (and again with `--no-default-features`) + `cargo package -p <each publishable crate> --allow-dirty --no-verify` |
| T3 | CI | CI | `cargo build --workspace --locked` + `cargo deny check` |

### Cargo.lock invariant

Voxora commits `Cargo.lock` at the workspace root (`./Cargo.lock`)
because every release runs `cargo build --workspace --locked`
and publishes each voxora-* crate with `cargo publish`. The
lockfile is the audit trail that pins transitive dependencies
(candle-core, qwen3-asr, whisper-rs, …) to the versions each
release was actually built and tested against.

Rules:

- **Always commit `Cargo.lock`.** A `git status` that lists it
  as modified is a pre-commit failure, not a "skip me" line —
  every workspace dependency change must land with its lockfile
  update in the same commit.
- **Never edit `Cargo.lock` by hand.** Use `cargo add`,
  `cargo update`, or `cargo upgrade`; let cargo rewrite the file.
- **Regenerate explicitly.** Run `cargo update --workspace` (or
  just `cargo build --workspace` without `--locked`) before
  staging the manifest change, so the regenerated lockfile is
  intentional and reviewable.
- **Tag the commit that includes the lockfile change.** If
  `Cargo.lock` shifts between the tag commit and the trunk
  merge, the GitHub Release's SBOM will disagree with the
  binary the operator built and published — the same TOCTOU
  gap that `verify-tag-reachability` was added to close.
- **CI is locked.** Tier T3 uses
  `cargo build --workspace --locked`; a lockfile drift that
  would flip a transitive version fails the build before the
  SBOM is generated, not after.

## ⚠️ Red / failed workflows: do NOT merge, repair until green

`release.yml` gates on tag-reachability AND tag-signature AND build.
A red required job blocks the pipeline. Tag is irreversible — once
`release.yml` has published, a workflow bug found afterwards can
only be fixed with another release.

Concretely:

1. Work on a local branch. Implement, commit (GPG-signed), push.
2. Watch it: `gh run watch <run-id>` / `gh run view <run-id> --log-failed`.
3. **If any required job is red**: read the log, fix locally, commit, push, re-dispatch. Do NOT proceed.
4. Only when the branch CI is fully green: open the PR.
5. Only when the release branch is green: tag the **merge commit on the trunk**, NOT the branch tip.
6. Verify the tag commit equals `origin/main`: `[ "$(git rev-parse TAG^{commit})" = "$(git rev-parse origin/main)" ]`.

`verify-tag-reachability` enforces this mechanically.
`verify-tag-signature` enforces that the tag was signed by a key in
`.github/trusted-signers.asc`.

## Trusted Signers

- `.github/trusted-signers` — SSH backend. Each line is
  `<principal> <key-type> <key-body>`. Used by
  `release.yml::verify-tag-signature` via
  `git config gpg.ssh.allowedsignersfile`. Current entry:
  `airvzxf@github` (ED25519, fingerprint
  `SHA256:POu2Sr8ILb1IM05Vh1cGU3xivjx05QjWoWYhdLc6YHA`).
- `.github/trusted-signers.asc` — PGP backend. ASCII-armored
  public key block for the maintainer's legacy PGP key
  (long ID `414687A3CD7E65B9`). Imported into the runner only
  when the tag being verified is PGP-signed (no such tags exist
  in voxora today; the file is forward-compatibility for a
  future PGP migration).

## License

Apache-2.0. See `LICENSE`.
