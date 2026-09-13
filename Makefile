.DEFAULT_GOAL := help

# Closes #164: install / uninstall / reinstall / prefix-check target
# pair plus the developer-ergonomics ladder (init / pre-commit /
# pre-push / ci / watch / doc-open / tree / meta / fmt-watch /
# watch-test). The default PREFIX follows the GNU Coding Standards
# (https://www.gnu.org/prep/standards/html_node/Directory-Variables.html)
# — `/usr/local` for unprivileged installs. Override on the
# command line for system packagers (`make install PREFIX=/usr`,
# `make install DESTDIR=/tmp/stage`).
PREFIX ?= /usr/local
DESTDIR ?=
BINDIR ?= $(PREFIX)/bin
INSTALL_TRACK := .install-track
MUSL_TARGET := x86_64-unknown-linux-musl

.PHONY: help validate fmt fmt-check guard-artifacts lint test build build-release build-cli build-musl doc package bench-no-run clean \
        install install-musl uninstall reinstall prefix-check \
        init pre-commit pre-push ci watch watch-test fmt-watch doc-open tree meta
HAS_RUST := $(shell find . -name '*.rs' -not -path './target/*' 2>/dev/null | head -1)

help:
	@echo "voxora — Makefile"
	@echo ""
	@echo "Build / test / lint:"
	@echo "  validate       Run the full pre-commit gauntlet (fmt-check, lint, test, build)"
	@echo "                 plus the strict doc pair (default + --no-default-features)"
	@echo "                 and the per-crate \`cargo package\` guard (catches a workspace"
	@echo "                 dependency requirement that exists in spirit but is not yet"
	@echo "                 on crates.io; \`cargo package --workspace\` would NOT catch it"
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
	@echo "  bench-no-run   Compile every bench target across the workspace (closes #56)."
	@echo "                 Mirrors the \`bench\` CI job; does NOT execute benches."
	@echo "                 For real RTF measurement see docs/quality/benchmarks.md."
	@echo "  clean          Remove build artifacts (target/)"
	@echo ""
	@echo "Install / uninstall (closes #164):"
	@echo "  install         Build release \`voxora-cli\` and install it to \$$(BINDIR)/voxora"
	@echo "                  (default BINDIR=/usr/local/bin). Records the absolute path in"
	@echo "                  \`.install-track\` for the matching \`uninstall\` target."
	@echo "                  Honors \`PREFIX\` and \`DESTDIR\` (GNU Coding Standards)."
	@echo "  install-musl    Same, but uses the fully-static musl build under"
	@echo "                  target/\$$(MUSL_TARGET)/release/voxora-cli. Fails fast if the"
	@echo "                  musl target is not installed."
	@echo "  uninstall       Reads \`.install-track\` and removes exactly the path the"
	@echo "                  previous \`make install\` placed. Refuses to guess if the"
	@echo "                  track file is missing (avoids \`rm\` of a path we never wrote)."
	@echo "  reinstall       \`uninstall\` + \`install\` (idempotent dev loop)."
	@echo "  prefix-check    Echoes the effective PREFIX / DESTDIR / BINDIR / install path"
	@echo "                  so operators can dry-run packaging before \`sudo make install\`."
	@echo ""
	@echo "Developer ergonomics:"
	@echo "  init            Best-effort one-shot dev setup: \`cargo install\` the operator"
	@echo "                  helpers (cargo-deny, cargo-outdated, cargo-audit, cargo-watch,"
	@echo "                  cargo-sort, cargo-msrv, tokei-cli, cargo-cyclonedx). Skips a"
	@echo "                  tool silently if \`cargo install\` is unavailable."
	@echo "  pre-commit      T0 + T1 (closes #164): \`cargo fmt --all --check\` + the"
	@echo "                  tracked-artifact guard + \`cargo clippy --workspace --all-targets\`."
	@echo "  pre-push        T0 + T1 + T2: pre-commit + \`cargo test --workspace --all-targets\`"
	@echo "                  + \`cargo doc --no-deps --workspace\` (strict)."
	@echo "  ci              T3 mirror: full pre-push + \`cargo build --workspace --locked\`"
	@echo "                  + per-crate \`cargo package\` + \`cargo bench --workspace --no-run\`."
	@echo "                  Matches what .github/workflows/ci.yml runs, locally."
	@echo "  watch           \`cargo watch -x check\` (falls back to a find-loop polling"
	@echo "                  \`cargo check\` if cargo-watch is not installed)."
	@echo "  watch-test      Same, but \`-x test\`."
	@echo "  fmt-watch       \`cargo watch -x 'fmt --all'\`."
	@echo "  doc-open        Open \`target/doc/index.html\` in the platform's default browser"
	@echo "                  (\`xdg-open\` on Linux, \`open\` on macOS)."
	@echo "  tree            \`cargo tree --workspace --edges normal --no-dedupe\`."
	@echo "  meta            \`cargo metadata --format-version 1\` to \`target/cargo-metadata.json\`."

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
		if rustup target list --installed 2>/dev/null | grep -q '$(MUSL_TARGET)'; then \
			cargo build --release -p voxora-cli --target $(MUSL_TARGET); \
		else \
			echo "musl target not installed. Run: rustup target add $(MUSL_TARGET)"; \
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

bench-no-run:
	@if [ -n "$(HAS_RUST)" ]; then cargo bench --workspace --no-run; else echo "(no Rust sources — skipping bench-no-run)"; fi

clean:
	cargo clean

# ─────────────────────────────────────────────────────────────────────────
# Closes #164: install / uninstall / reinstall / prefix-check
# ─────────────────────────────────────────────────────────────────────────

# The install path always goes through `install -m 0755`, which
# honors DESTDIR by treating the absolute target as relative to
# the staging root. The exact destination on the host is therefore
# `$(DESTDIR)$(BINDIR)/voxora` — the `.install-track` file stores
# that absolute path so `uninstall` can remove exactly what we
# wrote, even when DESTDIR is in play (the staging-root path).
INSTALL_BIN := $(DESTDIR)$(BINDIR)/voxora

prefix-check:
	@echo "PREFIX     = $(PREFIX)"
	@echo "DESTDIR    = $(DESTDIR)"
	@echo "BINDIR     = $(BINDIR)"
	@echo "INSTALL_BIN= $(INSTALL_BIN)"
	@echo "MUSL_TARGET= $(MUSL_TARGET)"

install: build-cli prefix-check
	@mkdir -p "$(DESTDIR)$(BINDIR)"
	@install -m 0755 target/release/voxora-cli "$(INSTALL_BIN)"
	@echo "$(INSTALL_BIN)" > $(INSTALL_TRACK)
	@echo "✓ installed $(INSTALL_BIN); uninstall via \`make uninstall\` (track file: $(INSTALL_TRACK))"

install-musl: prefix-check
	@if ! rustup target list --installed 2>/dev/null | grep -q '$(MUSL_TARGET)'; then \
		echo "musl target not installed. Run: rustup target add $(MUSL_TARGET)"; \
		exit 1; \
	fi
	@cargo build --release -p voxora-cli --target $(MUSL_TARGET)
	@mkdir -p "$(DESTDIR)$(BINDIR)"
	@install -m 0755 target/$(MUSL_TARGET)/release/voxora-cli "$(INSTALL_BIN)"
	@echo "$(INSTALL_BIN)" > $(INSTALL_TRACK)
	@echo "✓ installed static musl build to $(INSTALL_BIN)"

uninstall:
	@if [ ! -f $(INSTALL_TRACK) ]; then \
		echo "::error::no $(INSTALL_TRACK) file; refusing to guess what to remove. Run \`make install\` first, or remove the binary manually." >&2; \
		exit 1; \
	fi
	@track=$$(cat $(INSTALL_TRACK)); \
	if [ -e "$$track" ]; then \
		rm -f "$$track" && echo "✓ removed $$track"; \
	else \
		echo "::notice::$$track not present (already uninstalled); skipping"; \
	fi
	@rm -f $(INSTALL_TRACK)

reinstall: uninstall install

# ─────────────────────────────────────────────────────────────────────────
# Closes #164: developer ergonomics
# ─────────────────────────────────────────────────────────────────────────

# Best-effort: `cargo install` exits non-zero if a tool is already
# at the latest version, so we wrap each install in `|| true` to
# keep going even when one tool fails. The intent is "give the
# operator a working dev loop in one command", not "guarantee
# every tool is installed".
init:
	@echo "Installing developer tooling (best-effort; each tool is skipped on failure):"
	@for tool in cargo-deny cargo-outdated cargo-audit cargo-watch cargo-sort cargo-msrv tokei-cli cargo-cyclonedx; do \
		echo "  → cargo install $$tool --locked"; \
		cargo install $$tool --locked >/dev/null 2>&1 || echo "    (skipped: $$tool)"; \
	done
	@echo "✓ dev tooling install complete (check output above for skipped tools)"

pre-commit: fmt-check guard-artifacts lint

pre-push: pre-commit test doc

ci: pre-push build package bench-no-run

# `cargo watch` is the gold-standard; we fall back to a 5-second
# poll loop with `cargo check` if it isn't installed so the
# developer loop keeps working on a fresh clone.
watch:
	@if command -v cargo-watch >/dev/null 2>&1; then \
		cargo watch -x check; \
	else \
		echo "(cargo-watch not installed; falling back to a 5s poll loop. Run \`make init\` to install cargo-watch.)"; \
		while true; do cargo check --workspace --all-targets --locked || true; sleep 5; done; \
	fi

watch-test:
	@if command -v cargo-watch >/dev/null 2>&1; then \
		cargo watch -x test; \
	else \
		echo "(cargo-watch not installed; falling back to a 5s poll loop)"; \
		while true; do cargo test --workspace --all-targets || true; sleep 5; done; \
	fi

fmt-watch:
	@if command -v cargo-watch >/dev/null 2>&1; then \
		cargo watch -x 'fmt --all'; \
	else \
		echo "(cargo-watch not installed; cannot watch-format. Run \`make init\`.)"; \
		exit 1; \
	fi

doc-open:
	@if [ ! -f target/doc/index.html ]; then \
		echo "::error::target/doc/index.html not found; run \`make doc\` first" >&2; \
		exit 1; \
	fi
	@if command -v xdg-open >/dev/null 2>&1; then \
		xdg-open target/doc/index.html; \
	elif command -v open >/dev/null 2>&1; then \
		open target/doc/index.html; \
	else \
		echo "::error::no \`xdg-open\` (Linux) or \`open\` (macOS) available; open target/doc/index.html manually" >&2; \
		exit 1; \
	fi

tree:
	@if [ -n "$(HAS_RUST)" ]; then cargo tree --workspace --edges normal --no-dedupe; else echo "(no Rust sources — skipping tree)"; fi

meta:
	@if [ -n "$(HAS_RUST)" ]; then mkdir -p target && cargo metadata --format-version 1 > target/cargo-metadata.json && echo "✓ wrote target/cargo-metadata.json"; else echo "(no Rust sources — skipping meta)"; fi
