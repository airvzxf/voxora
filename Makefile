.DEFAULT_GOAL := help

.PHONY: help validate fmt fmt-check guard-artifacts lint test build build-release build-cli build-musl doc package clean
HAS_RUST := $(shell find . -name '*.rs' -not -path './target/*' 2>/dev/null | head -1)

help:
	@echo "voxora — Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  validate       Run the full pre-commit gauntlet (fmt-check, lint, test, build)"
	@echo "                 plus the strict doc pair (default + --no-default-features)"
	@echo "                 and the per-crate `cargo package` guard (catches a workspace"
	@echo "                 dependency requirement that exists in spirit but is not yet"
	@echo "                 on crates.io; `cargo package --workspace` would NOT catch it"
	@echo "                 because workspace packaging resolves inter-crate deps via path)"
	@echo "  fmt            Format all code with cargo fmt"
	@echo "  fmt-check      Check formatting without modifying files"
	@echo "  lint           Run clippy with warnings as errors"
	@echo "  test           Run all tests across all targets"
	@echo "  build          Build all targets (debug)"
	@echo "  build-release  Build all targets (release)"
	@echo "  build-cli      Build the voxora-cli binary (release)"
	@echo "  build-musl     Build voxora-cli as a fully static musl binary (x86_64)"
	@echo "                 Requires: rustup target add x86_64-unknown-linux-musl"
	@echo "  doc            Build documentation (strict, default + --no-default-features)"
	@echo "  package        Run cargo package -p <each publishable crate> --allow-dirty --no-verify"
	@echo "                 (catches workspace dep requirements that aren't on crates.io)"
	@echo "  clean          Remove build artifacts (target/)"

validate: fmt-check guard-artifacts lint test build doc package

fmt:
	@if [ -n "$(HAS_RUST)" ]; then cargo fmt --all; else echo "(no Rust sources — skipping fmt)"; fi

fmt-check:
	@if [ -n "$(HAS_RUST)" ]; then cargo fmt --all --check; else echo "(no Rust sources — skipping fmt-check)"; fi

guard-artifacts:
	@if git ls-files | grep -E '(^|/)CACHEDIR\.TAG$$|(^|/)\.(rustc_info|rustdoc_fingerprint)\.json$$|(^|/)(target|\.cargo-target|\.worktrees)/'; then \
		echo "::error::build artifacts are tracked; see the paths above" >&2; \
		exit 1; \
	fi

lint:
	@if [ -n "$(HAS_RUST)" ]; then cargo clippy --workspace --all-targets -- -D warnings; else echo "(no Rust sources — skipping lint)"; fi

test:
	@if [ -n "$(HAS_RUST)" ]; then cargo test --workspace --all-targets; else echo "(no Rust sources — skipping test)"; fi

build:
	@if [ -n "$(HAS_RUST)" ]; then cargo build --workspace --all-targets --locked; else echo "(no Rust sources — skipping build)"; fi

build-release:
	@if [ -n "$(HAS_RUST)" ]; then cargo build --release --workspace; else echo "(no Rust sources — skipping build-release)"; fi

build-cli:
	@if [ -n "$(HAS_RUST)" ]; then cargo build --release -p voxora-cli; else echo "(no Rust sources — skipping build-cli)"; fi

build-musl:
	@if [ -n "$(HAS_RUST)" ]; then \
		if rustup target list --installed 2>/dev/null | grep -q 'x86_64-unknown-linux-musl'; then \
			cargo build --release -p voxora-cli --target x86_64-unknown-linux-musl; \
		else \
			echo "musl target not installed. Run: rustup target add x86_64-unknown-linux-musl"; \
			exit 1; \
		fi; \
	else \
		echo "(no Rust sources — skipping build-musl)"; \
	fi

doc:
	@if [ -n "$(HAS_RUST)" ]; then \
		export RUSTDOCFLAGS="-D warnings"; \
		for features in '' '--no-default-features' \
		               '--no-default-features --features voxora-bridge/whisper' \
		               '--no-default-features --features voxora-bridge/qwen3asr'; do \
			echo ">> doc leg: '$${features:-<default>}'"; \
			cargo doc --no-deps --workspace $$features; \
		done; \
	else echo "(no Rust sources — skipping doc)"; fi

package:
	@if [ -n "$(HAS_RUST)" ]; then \
		for crate in voxora-traits voxora-config voxora-hf voxora-engine voxora-backend voxora-whisper voxora-qwen3asr voxora-registry voxora-local voxora-vad voxora-bridge; do \
			cargo package -p "$${crate}" --allow-dirty --no-verify; \
		done; \
	else echo "(no Rust sources — skipping package)"; fi

clean:
	cargo clean
