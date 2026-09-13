#!/usr/bin/env python3
"""Unit tests for check-cratesio-deps.py (closes #110).

Exercises the cargo pre-release exclusion rule end-to-end:
  - a pre-release candidate is rejected against a normal `^X.Y.Z`
    or `~X.Y.Z` requirement,
  - the same pre-release is accepted when the requirement's lower
    bound explicitly names it,
  - normal-version comparison continues to pick the highest match,
  - the highest-pre-release pick is the pre-release only when it
    sorts ABOVE all normal candidates.

Run with:
    python3 .github/scripts/test-check-cratesio-deps.py

Exit 0 on success, non-zero with a `FAIL:` line on the first
failure. Invoked from CI as a `python3 -m unittest`-style harness
but kept dependency-free so the runner doesn't need pytest.
"""
from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
SCRIPT_PATH = SCRIPT_DIR / "check-cratesio-deps.py"


def _load_module():
    spec = importlib.util.spec_from_file_location("check_cratesio_deps", SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _run_cli(req: str, versions: list[str]) -> tuple[int, str]:
    """Invoke the script as a subprocess so we also cover the
    CLI entry-point, not just the in-process functions."""
    proc = subprocess.run(
        [sys.executable, str(SCRIPT_PATH), req, *versions],
        capture_output=True,
        text=True,
        check=False,
    )
    return proc.returncode, (proc.stdout + proc.stderr).strip()


def _expect(condition: bool, message: str) -> None:
    if not condition:
        print(f"FAIL: {message}", file=sys.stderr)
        sys.exit(1)


def main() -> int:
    module = _load_module()

    # In-process checks (faster, easier to inspect).
    # Caret without pre: pre-release candidate is rejected.
    _expect(
        not module.satisfies("^0.4.2", "0.4.3-rc.1"),
        "caret ^0.4.2 must reject 0.4.3-rc.1 (closes #110)",
    )
    _expect(
        not module.satisfies("^0.4.2", "0.4.3-rc.1"),
        "caret ^0.4.2 must reject 0.4.3-rc.1 (alias)",
    )
    # Caret with pre lower bound: pre-release candidate accepted.
    _expect(
        module.satisfies("^0.4.3-rc.1", "0.4.3-rc.1"),
        "caret ^0.4.3-rc.1 must accept 0.4.3-rc.1",
    )
    _expect(
        module.satisfies("^0.4.3-rc.1", "0.4.3"),
        "caret ^0.4.3-rc.1 must accept 0.4.3 (post-pre)",
    )
    # Tilde without pre: pre-release candidate is rejected.
    _expect(
        not module.satisfies("~0.4.2", "0.4.3-rc.1"),
        "tilde ~0.4.2 must reject 0.4.3-rc.1",
    )
    # Comparison without pre on lower bound: pre-release excluded.
    _expect(
        not module.satisfies(">=0.4.0, <0.5.0", "0.4.2-rc.5"),
        "range >=0.4.0, <0.5.0 must reject 0.4.2-rc.5",
    )
    # Comparison with explicit pre on lower bound: pre-release allowed.
    _expect(
        module.satisfies(">=0.4.0-rc.5, <0.5.0", "0.4.2-rc.5"),
        "range >=0.4.0-rc.5, <0.5.0 must accept 0.4.2-rc.5",
    )
    # Ordering: pre-release sorts BELOW normal of the same x.y.z.
    _expect(
        module.cmp(module.parse("0.4.3-rc.1"), module.parse("0.4.3")) < 0,
        "ordering must place 0.4.3-rc.1 BELOW 0.4.3 (semver 2.0 §11)",
    )
    _expect(
        module.cmp(module.parse("0.4.3"), module.parse("0.4.3-rc.1")) > 0,
        "ordering must place 0.4.3 ABOVE 0.4.3-rc.1 (alias)",
    )
    # Highest-match logic: caret ^0.4.2 with candidates 0.4.2,
    # 0.4.3-rc.1 and 0.4.3 must pick 0.4.3 (the pre-release is
    # excluded per cargo's pre-release rule).
    rc, out = _run_cli("^0.4.2", ["0.4.2", "0.4.3-rc.1", "0.4.3"])
    _expect(rc == 0, f"^0.4.2 CLI must exit 0; got rc={rc} stderr={out!r}")
    _expect(out == "0.4.3", f"^0.4.2 must pick 0.4.3 (not 0.4.3-rc.1); got {out!r}")
    # Tilde ~0.4.2 with candidates 0.4.2, 0.4.3, 0.4.3-rc.1:
    # pre-release excluded, so 0.4.3 wins.
    rc, out = _run_cli("~0.4.2", ["0.4.2", "0.4.3", "0.4.3-rc.1"])
    _expect(rc == 0, f"~0.4.2 CLI must exit 0; got rc={rc}")
    _expect(
        out == "0.4.3",
        f"~0.4.2 must pick 0.4.3 (pre-release excluded); got {out!r}",
    )
    # Caret ^0.4.3-rc.1 admits the pre-release, but 0.4.3 still wins
    # because it sorts higher in semver order.
    rc, out = _run_cli("^0.4.3-rc.1", ["0.4.3-rc.1", "0.4.3"])
    _expect(rc == 0, f"^0.4.3-rc.1 CLI must exit 0; got rc={rc}")
    _expect(
        out == "0.4.3",
        f"^0.4.3-rc.1 must pick the higher 0.4.3; got {out!r}",
    )
    # No-match exits 1 (and prints nothing).
    rc, out = _run_cli("^9.9.9", ["1.0.0"])
    _expect(rc == 1, f"no-match must exit 1; got rc={rc}")
    _expect(out == "", f"no-match must print nothing; got {out!r}")
    print("ok — check-cratesio-deps.py pre-release rule passes all tests")
    return 0


if __name__ == "__main__":
    sys.exit(main())