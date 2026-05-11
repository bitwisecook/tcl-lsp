#!/usr/bin/env python3
"""Refresh tests/baselines/tcl9_tcltest_baseline.json from the latest sweep.

Reads ``tmp/perf-output/tcltest_results.json`` (the rich per-row
output of ``scripts/dev/run_tcl9_tcltest_sweep.py``), classifies each
row's wasm-side outcome, and writes the slim summary baseline that
``scripts/dev/diff_tcl9_tcltest.py`` consumes.

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
  uv run python scripts/dev/refresh_tcl9_baseline.py
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _tcl9_classify import classify  # noqa: E402

REPO = Path(__file__).resolve().parents[2]
RESULTS = REPO / "tmp" / "perf-output" / "tcltest_results.json"
BASELINE = REPO / "tests" / "baselines" / "tcl9_tcltest_baseline.json"


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
