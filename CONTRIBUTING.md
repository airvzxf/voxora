# Contributing to voxora

Thanks for your interest in voxora. This project is in pre-alpha
(scaffolding only). The investigation recap
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
tracked in the root `Cargo.toml` (`rust-version = "1.78"`).

```bash
git clone https://github.com/airvzxf/voxora.git
cd voxora
cargo --version    # must be >= 1.78
```

The first phase to land is `voxora-core` (the trait). Until then
`cargo build` will produce an empty workspace.

## Coding standards

The project follows standard Rust conventions:

- `cargo fmt --all` before committing.
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
3. Implement `voxora_core::AsrEngine` for the engine's wrapper type.
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