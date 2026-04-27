#!/usr/bin/env python3
"""Refresh tests/baselines/tcl9_tcltest_baseline.json from the latest sweep.

Reads ``tmp/perf-output/tcltest_results.json`` (the rich per-row
output of ``scripts/run_tcl9_tcltest_sweep.py``), classifies each
row's wasm-side outcome, and writes the slim summary baseline that
``scripts/diff_tcl9_tcltest.py`` consumes.

Baseline shape:
  {
    "rows": [
      {"stem": "...", "subsystem": "...",
       "tcl_passed": N, "tcl_total": N,
       "wasm_passed": N, "wasm_failed": N, "wasm_total": N,
       "wasm_status": "pass|partial|run-trap|compile-fail|no-summary|no-pass"},
      ...
    ]
  }

Run after a sweep:
  uv run python scripts/refresh_tcl9_baseline.py
"""

import json
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
RESULTS = REPO / "tmp" / "perf-output" / "tcltest_results.json"
BASELINE = REPO / "tests" / "baselines" / "tcl9_tcltest_baseline.json"


def classify(w: dict) -> str:
    if w.get("compile_error"):
        return "compile-fail"
    if w.get("run_error"):
        return "run-trap"
    if "total" not in w:
        return "no-summary"
    failed = w.get("failed", 0)
    passed = w.get("passed", 0)
    total = w.get("total", 0)
    if total == 0:
        return "no-summary"
    if failed == 0 and passed > 0 and passed == total - w.get("skipped", 0):
        return "pass"
    if passed > 0:
        return "partial"
    return "no-pass"


def main() -> None:
    rows_in = json.loads(RESULTS.read_text())
    rows_out = []
    for r in rows_in:
        w = r.get("wasm", {})
        t = r.get("tclsh", {})
        rows_out.append(
            {
                "stem": r["stem"],
                "subsystem": r["subsystem"],
                "tcl_passed": t.get("passed", 0),
                "tcl_total": t.get("total", 0),
                "wasm_failed": w.get("failed", 0),
                "wasm_passed": w.get("passed", 0),
                "wasm_status": classify(w),
                "wasm_total": w.get("total", 0),
            }
        )
    rows_out.sort(key=lambda r: (r["subsystem"], r["stem"]))
    BASELINE.write_text(json.dumps({"rows": rows_out}, indent=2, sort_keys=True) + "\n")
    print(f"wrote {len(rows_out)} rows to {BASELINE.relative_to(REPO)}")


if __name__ == "__main__":
    main()
