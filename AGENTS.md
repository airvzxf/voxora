# Working rules (agents and humans)

1. **One task = one branch = one PR.** `main` is protected; the owner merges, never the agent.
2. **Reproduce first.** A failing test, or a command/log showing the bug. The PR carries before/after evidence.
3. **Nothing is fixed by text alone.** Editing a comment, CHANGELOG, README or issue does not fix behaviour.
4. **Do not trust comments, docs, CHANGELOGs, commit messages or issues.** Much of it was written without verification. Check against code and runtime.
5. **Comments**: only the non-obvious *why* (an invariant, an external constraint). No issue/PR numbers, no "closes #", no history, no restating the code.
6. **Tests**: each test catches a concrete, realistic failure. No tests that pass when they cannot run.
7. **No releases, tags or `cargo publish`** without an explicit request from the owner.
8. **Public API**: crates are on crates.io. Any breaking change needs the owner's approval and a minor bump.
9. **PR checklist**:
   - [ ] `cargo fmt --all -- --check`
   - [ ] `cargo clippy --workspace --all-targets -- -D warnings`
   - [ ] `cargo test --workspace` (plus `-- --ignored` for the touched engine when it needs a real model)
   - [ ] Before/after evidence
   - [ ] Every new claim in docs/comments/CHANGELOG is verified

Conventions: English for code and docs; `#![forbid(unsafe_code)]`; `rustfmt.toml` settings are intentional.
