# Contributing to voxora

Thanks for your interest in voxora. This project is in pre-alpha
(core + hf + whisper + qwen3asr + cli). The investigation recap
([`docs/INVESTIGATION.md`](docs/INVESTIGATION.md)) explains why the
project exists, what it will become, and how it relates to
[candle](https://github.com/huggingface/candle),
[qwen3-asr-rs](https://github.com/airvzxf/qwen3-asr-rs), and
[Telora](https://github.com/airvzxf/telora).

## Code of Conduct

By participating, you agree to abide by our
[Code of Conduct](CODE_OF_CONDUCT.md).

## Development setup

voxora is a Cargo workspace. The minimum supported Rust version is
tracked in the root `Cargo.toml` (`rust-version = "1.88"`).

```bash
git clone https://github.com/airvzxf/voxora.git
cd voxora
cargo --version    # must be >= 1.88 (the workspace MSRV)
```

The first phase to land was `voxora-core` (the trait, since
moved to `voxora-traits`). The workspace today has 11 crates and
ships `voxora-cli`, `voxora-bridge`, and several engines.

## Coding standards

The project follows standard Rust conventions:

- `cargo fmt --all` before committing (see `rustfmt.toml` and
  [`AGENTS.md` → Coding conventions](AGENTS.md#coding-conventions)
  for the pinned policy).
- `cargo clippy --all-targets -- -D warnings` must pass.
- `cargo test --all` must pass.
- Public APIs use `#[non_exhaustive]` on structs so we can add fields
  without breaking SemVer during pre-1.0.
- `unsafe` is forbidden at the workspace level (enforced by
  `#![forbid(unsafe_code)]` in each crate's `lib.rs`). All unsafe
  we need lives in our dependencies (candle, tokenizers).

## Commit signing

Commits must be GPG-signed. The `airvzxf/voxora` repo follows the
same GPG policy as the rest of the maintainer's projects
(`commit.gpgsign = true`).

Verify before pushing:

```bash
git log --pretty="%H %G? %s" origin/main..HEAD
```

Every line must start with `G` (good signature).

## Branch and PR conventions

- Branch off `main`.
- Use Conventional Commits for the subject line
  (`feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`).
- Reference any related issue in the PR body.
- One logical change per PR.

## Adding a new engine adapter

When a new model family becomes available (Parakeet, Voxtral,
Granite-Speech, …), the workflow is:

1. Add a new crate `voxora-<engine>/` under the workspace.
2. Add it to the root `Cargo.toml` `[workspace] members` list.
3. Implement `voxora_traits::AsrEngine` for the engine's wrapper type.
4. Re-export the engine crate's public API if needed; do not
   re-export private types.
5. Add a smoke test that loads a small fixture and asserts on the
   resulting `TranscriptionResult::text`.
6. Update `docs/ROADMAP.md` to mark the relevant phase done.

## License

By contributing, you agree that your contributions are licensed
under the Apache License, Version 2.0. The maintainer prefers a CLA
not be required for the pre-alpha phase; this will be revisited if
the project gains outside contributors.

## Release process

voxora uses **per-crate SemVer tags** of the shape `voxora-<name>-vX.Y.Z`
(e.g. `voxora-core-v0.3.0`) and a single umbrella `vX.Y.Z` tag on the
same trunk commit. The operator tags each crate separately
(`release.yml` enforces the `voxora-<name>-vX.Y.Z` shape), but the
**version numbers themselves are coordinated**: when a release ships
as `voxora X.Y.0`, every workspace crate that participates in that
release ships at `X.Y.0`. See
[`AGENTS.md` → Version coordination](AGENTS.md#version-coordination)
for the full invariant and the additive-change exception.

### Tagging a release

`git tag -s` produces an **annotated + signed** tag. voxora's git
config sets `gpg.format = ssh` (per
[`AGENTS.md` → Trusted Signers](AGENTS.md#trusted-signers)), so
`-s` here means **SSH-signed with the maintainer's ED25519 key
(`airvzxf@github`)** — not a PGP/GPG signature. `release.yml`'s
`verify-tag-signature` job checks the SSH allow-list at
`.github/trusted-signers`, not the GPG keyring, so a tag signed
with any other key (PGP or SSH) aborts the release.

```bash
# 1. Sync main
git fetch origin main
git checkout main
git reset --hard origin/main

# 2. Verify Cargo.toml matches the planned tag
git log --oneline -1   # expect: <sha> chore(release): vX.Y.Z — ...
grep -E '^version\s*=\s*"X\.Y\.Z"' voxora-<name>/Cargo.toml
# expect: version = "X.Y.Z" (same X.Y.Z across participating crates).
# Note: crates that ship with `version.workspace = true` already
# match the workspace-level X.Y.Z; verify by checking
# `[workspace.package] version` in the root Cargo.toml.

# 3. Tag with SSH signing (annotated + signed via -s + -m).
#    Tag the merge commit on `origin/main`, NOT a branch tip.
git tag -s -m "release: voxora-<name> vX.Y.Z" \
    voxora-<name>-vX.Y.Z "$(git rev-parse origin/main)"
git push origin voxora-<name>-vX.Y.Z

# 4. Verify the tag's commit equals origin/main
[ "$(git rev-parse voxora-<name>-vX.Y.Z^{commit})" = "$(git rev-parse origin/main)" ] || { echo "ORPHAN TAG — abort"; exit 1; }

# 5. Verify the signature matches a trusted signer (must print
#    'Good "git" signature for airvzxf@github with ED25519 key
#    SHA256:POu2Sr8ILb1IM05Vh1cGU3xivjx05QjWoWYhdLc6YHA').
git verify-tag voxora-<name>-vX.Y.Z
```

Lightweight tags (`git tag NAME SHA` without `-a -s`) carry no
signature object and will fail `verify-tag-signature` in the
release workflow. They are forbidden.

### Workflow then runs

`release.yml` triggers on the tag push, validates the tag shape,
verifies the tag is reachable from `origin/main` and signed by a
key in `.github/trusted-signers`, builds the workspace with
`cargo build --workspace --release --locked`, generates an SBOM,
and creates a GitHub Release page.

### Publishing to crates.io

The GitHub Release page is **audit-only** on its own — pushing a
tag does NOT publish to crates.io. The operator invokes the
`publish-cratesio` job manually after visually confirming the
release page:

```bash
# 6. Publish (after the GitHub Release is live)
gh workflow run release.yml -f tag=voxora-<name>-vX.Y.Z
gh run watch    # wait for the `publish-cratesio` job to turn green
```

The publish job uses [Trusted Publishing (OIDC)][tp] so no
long-lived `CARGO_REGISTRY_TOKEN` is needed — the workflow
exchanges a short-lived OIDC token for a crates.io API token
(multi-use within ~30 minutes, server-side expiry) at runtime.

[See the crates.io order table](#cratesio-trusted-publishing-setup)
below for the dependency order and the one-time setup checklist.

### crates.io Trusted Publishing setup

This is a **one-time** setup that the maintainer runs from the
crates.io web UI. Until every publishable crate is registered,
the `publish-cratesio` job will return HTTP 400 from crates.io's
`/api/v1/trusted_publishing/tokens` endpoint with an
`errors.detail` body explaining which claim rejected, and the
job fails fast.

The workspace has **9 publishable** crates (`voxora-cli` and
`voxora-testkit` are `publish = false`; no crates.io entry
needed). Register each one with the values:

| crates.io field | Value |
|---|---|
| Repository owner | `airvzxf` |
| Repository name | `voxora` |
| Workflow filename | `release.yml` |
| Environment name | *(leave blank)* |

**Crate order** (must be registered all at once, but the publish
job enforces the same order at runtime — registering out of
order is harmless):

```text
voxora-traits     (no deps)
voxora-config     (no voxora deps; env-var cascade)
voxora-engine     (depends on voxora-traits)
voxora-hf         (depends on voxora-traits, voxora-config)
voxora-backend    (depends on voxora-engine)
voxora-whisper    (depends on voxora-traits)
voxora-qwen3asr   (depends on voxora-traits)
voxora-registry   (depends on voxora-traits, voxora-engine, voxora-hf)
voxora-bridge     (depends on voxora-traits, voxora-hf,
                   voxora-engine, voxora-whisper, voxora-qwen3asr)
voxora-cli        (publish=false — skip)
voxora-testkit    (publish=false — skip)
```

Per crate, the steps in the crates.io UI are:

1. Sign in to <https://crates.io/> as the maintainer.
2. Click the crate name → **Settings** → **Trusted Publishing**.
3. Click **Add GitHub publisher**.
4. Fill the four fields from the table above. The
   "Workflow filename" must be exactly `release.yml` (the path
   under `.github/workflows/`). Leave **Environment name**
   blank — that matches the workflow's default environment
   (none). Do NOT enter `*`; that literal string won't match
   any environment.
5. Save. The crates.io UI shows a green check next to the
   registered workflow file.

Repeat for each of the 9 crates. After the last one, run
`gh workflow run release.yml -f tag=voxora-traits-v0.4.0` as a
smoke test — the OIDC exchange should return a token and
`cargo publish -p voxora-traits --locked` should land the upload
on crates.io without you touching an API token.

[tp]: https://crates.io/docs/trusted-publishing

#### Fallback: manual `cargo publish`

If Trusted Publishing is not yet configured (or fails), the
fallback is the legacy flow:

```bash
git checkout voxora-<name>-vX.Y.Z
export CARGO_REGISTRIES_CRATES_IO_TOKEN=crates-io:<your-token>
cargo publish -p voxora-<name> --locked
```

Same dependency order applies; both flows publish one crate at
a time.