.DEFAULT_GOAL := help

.PHONY: help validate fmt fmt-check lint test build build-release build-cli build-musl doc clean
HAS_RUST := $(shell find . -name '*.rs' -not -path './target/*' 2>/dev/null | head -1)

help:
	@echo "voxora — Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  validate       Run the full pre-commit gauntlet (fmt-check, lint, test, build)"
	@echo "  fmt            Format all code with cargo fmt"
	@echo "  fmt-check      Check formatting without modifying files"
	@echo "  lint           Run clippy with warnings as errors"
	@echo "  test           Run all tests across all targets"
	@echo "  build          Build all targets (debug)"
	@echo "  build-release  Build all targets (release)"
	@echo "  build-cli      Build the voxora-cli binary (release)"
	@echo "  build-musl     Build voxora-cli as a fully static musl binary (x86_64)"
	@echo "                 Requires: rustup target add x86_64-unknown-linux-musl"
	@echo "  doc            Build documentation"
	@echo "  clean          Remove build artifacts (target/)"

validate: fmt-check lint test build

fmt:
	@if [ -n "$(HAS_RUST)" ]; then cargo fmt --all; else echo "(no Rust sources — skipping fmt)"; fi

fmt-check:
	@if [ -n "$(HAS_RUST)" ]; then cargo fmt --all --check; else echo "(no Rust sources — skipping fmt-check)"; fi

lint:
	@if [ -n "$(HAS_RUST)" ]; then cargo clippy --workspace --all-targets -- -D warnings; else echo "(no Rust sources — skipping lint)"; fi

test:
	@if [ -n "$(HAS_RUST)" ]; then cargo test --workspace --all-targets; else echo "(no Rust sources — skipping test)"; fi

build:
	@if [ -n "$(HAS_RUST)" ]; then cargo build --workspace --all-targets; else echo "(no Rust sources — skipping build)"; fi

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
	@if [ -n "$(HAS_RUST)" ]; then cargo doc --no-deps --workspace; else echo "(no Rust sources — skipping doc)"; fi

clean:
	cargo clean
