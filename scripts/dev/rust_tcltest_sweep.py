#!/usr/bin/env python3
"""Run the Tcl 9 tcltest suite against the **Rust runtime** and score it.

This is the Rust-runtime analogue of ``run_tcl9_tcltest_sweep.py`` (which
drives the Zig/WASM backend). It runs each ``tmp/tcl9.0.3/tests/*.test`` file
through the runtime's ``run_script`` example — which sources the file like
``tclsh`` (``--init`` bootstraps ``init.tcl`` so the real ``tcltest`` package
loads) — and parses tcltest's own summary line:

    <file>: Total N Passed P Skipped S Failed F

Per file we classify the outcome:

  * ``pass``    — a summary with zero failures
  * ``partial`` — a summary with some failures (P/Total reported)
  * ``error``   — no summary: the file errored/crashed before cleanupTests
  * ``timeout`` — exceeded the per-file cap

Each file gets a generous timeout: these suites exercise things the runtime
does not implement yet, so errors / hangs are expected and must not stop the
sweep. The aggregate pass-rate is the **baseline** for the Rust runtime
(``docs/design/runtime/rust-runtime-port.md`` scoreboard).

Usage:
    TCL_LIBRARY=tmp/tcl9.0.3/library python3 scripts/dev/rust_tcltest_sweep.py \
        [--timeout S] [--json out.json] [files...]
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
TESTS_DIR = REPO / "tmp" / "tcl9.0.3" / "tests"
LIBRARY = REPO / "tmp" / "tcl9.0.3" / "library"
RUN_SCRIPT = REPO / "runtime" / "rust" / "target" / "release" / "examples" / "run_script"

# tcltest's end-of-file summary, e.g.
#   incr.test:	Total	69	Passed	61	Skipped	0	Failed	8
SUMMARY_RE = re.compile(r"Total\s+(\d+)\s+Passed\s+(\d+)\s+Skipped\s+(\d+)\s+Failed\s+(\d+)")


def run_one(test: Path, timeout_s: float) -> dict:
    """Run one .test file through the runtime; return its scored outcome."""
    try:
        proc = subprocess.run(
            [str(RUN_SCRIPT), "--init", str(test)],
            env={
                "TCL_LIBRARY": str(LIBRARY),
                "PATH": "/usr/bin:/bin",
            },
            capture_output=True,
            timeout=timeout_s,
            cwd=str(TESTS_DIR),
        )
    except subprocess.TimeoutExpired:
        return {"file": test.name, "status": "timeout"}

    out = (proc.stdout + proc.stderr).decode("utf-8", "replace")
    # The last summary line wins (a file may print several).
    summary = None
    for m in SUMMARY_RE.finditer(out):
        summary = m
    if summary:
        total, passed, skipped, failed = (int(g) for g in summary.groups())
        return {
            "file": test.name,
            "status": "pass" if failed == 0 else "partial",
            "total": total,
            "passed": passed,
            "skipped": skipped,
            "failed": failed,
        }
    # No summary: errored/crashed before cleanupTests. Capture the last line.
    last = next((ln for ln in reversed(out.splitlines()) if ln.strip()), "")
    return {"file": test.name, "status": "error", "reason": last[:120]}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--timeout", type=float, default=20.0)
    ap.add_argument("--json", type=Path, default=None)
    ap.add_argument("files", nargs="*", help="specific .test files (default: all)")
    args = ap.parse_args()

    if not RUN_SCRIPT.exists():
        print(
            f"run_script not built: {RUN_SCRIPT}\n"
            "  build: (cd runtime/rust && cargo build --release --example run_script)",
            file=sys.stderr,
        )
        return 2

    files = [TESTS_DIR / f for f in args.files] if args.files else sorted(TESTS_DIR.glob("*.test"))

    results = []
    agg_total = agg_passed = agg_failed = agg_skipped = 0
    n_pass = n_partial = n_error = n_timeout = 0
    for test in files:
        r = run_one(test, args.timeout)
        results.append(r)
        st = r["status"]
        if st in ("pass", "partial"):
            agg_total += r["total"]
            agg_passed += r["passed"]
            agg_failed += r["failed"]
            agg_skipped += r["skipped"]
            n_pass += st == "pass"
            n_partial += st == "partial"
            mark = "OK " if st == "pass" else "PART"
            print(
                f"  {mark} {r['file']:<24} {r['passed']}/{r['total']}"
                f" (skip {r['skipped']}, fail {r['failed']})"
            )
        elif st == "error":
            n_error += 1
            print(f"  ERR  {r['file']:<24} {r['reason']}")
        else:
            n_timeout += 1
            print(f"  T/O  {r['file']:<24} (>{args.timeout}s)")

    n_files = len(results)
    print("\n=== Rust runtime — Tcl 9 tcltest baseline ===")
    print(
        f"files: {n_files}  "
        f"(pass {n_pass}, partial {n_partial}, error {n_error}, timeout {n_timeout})"
    )
    pct = (100.0 * agg_passed / agg_total) if agg_total else 0.0
    print(
        f"tests: {agg_passed}/{agg_total} passed ({pct:.1f}%), "
        f"{agg_failed} failed, {agg_skipped} skipped "
        f"(across {n_pass + n_partial} files that ran to a summary)"
    )

    if args.json:
        args.json.write_text(
            json.dumps(
                {
                    "files": results,
                    "aggregate": {
                        "n_files": n_files,
                        "n_pass": n_pass,
                        "n_partial": n_partial,
                        "n_error": n_error,
                        "n_timeout": n_timeout,
                        "tests_total": agg_total,
                        "tests_passed": agg_passed,
                        "tests_failed": agg_failed,
                        "tests_skipped": agg_skipped,
                    },
                },
                indent=2,
            )
        )
        print(f"wrote {args.json}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
