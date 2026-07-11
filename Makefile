.DEFAULT_GOAL := help

.PHONY: help validate fmt fmt-check lint test build build-release doc clean
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
	@echo "  doc            Build documentation"
	@echo "  clean          Remove build artifacts (target/)"

validate: fmt-check lint test build

fmt:
	@if [ -n "$(HAS_RUST)" ]; then cargo fmt --all; else echo "(no Rust sources — skipping fmt)"; fi

fmt-check:
	@if [ -n "$(HAS_RUST)" ]; then cargo fmt --all --check; else echo "(no Rust sources — skipping fmt-check)"; fi

lint:
	@if [ -n "$(HAS_RUST)" ]; then cargo clippy --all-targets -- -D warnings; else echo "(no Rust sources — skipping lint)"; fi

test:
	@if [ -n "$(HAS_RUST)" ]; then cargo test --all --all-targets; else echo "(no Rust sources — skipping test)"; fi

build:
	@if [ -n "$(HAS_RUST)" ]; then cargo build --all-targets; else echo "(no Rust sources — skipping build)"; fi

build-release:
	@if [ -n "$(HAS_RUST)" ]; then cargo build --release --all-targets; else echo "(no Rust sources — skipping build-release)"; fi

doc:
	@if [ -n "$(HAS_RUST)" ]; then cargo doc --no-deps; else echo "(no Rust sources — skipping doc)"; fi

clean:
	cargo clean