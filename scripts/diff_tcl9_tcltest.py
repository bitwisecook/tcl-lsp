#!/usr/bin/env python3
"""Diff the latest tcltest sweep output against the baseline.

Reads:
  tests/baselines/tcl9_tcltest_baseline.json — historical baseline
  tmp/perf-output/tcltest_results.json       — fresh sweep output

Reports:
  - Status distribution (pass/partial/run-trap/...) before-and-after.
  - Per-stem improvements and regressions, with passed/total counts.
  - Total tests passing across the suite.

Usage:
  uv run python scripts/diff_tcl9_tcltest.py
"""

import json
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
BASELINE = REPO / "tests" / "baselines" / "tcl9_tcltest_baseline.json"
RESULTS = REPO / "tmp" / "perf-output" / "tcltest_results.json"


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


# Status hierarchy: higher rank = better outcome.
RANK = {
    "pass": 6, "partial": 5, "no-summary": 4, "no-pass": 3,
    "run-trap": 2, "compile-fail": 1, "missing": 0, "unknown": 0,
}


def main() -> None:
    base_data = json.loads(BASELINE.read_text())
    base_rows = base_data.get("rows", base_data) if isinstance(base_data, dict) else base_data
    base_by_stem = {r["stem"]: r for r in base_rows}

    new_rows = json.loads(RESULTS.read_text())
    new_by_stem = {r["stem"]: r for r in new_rows}

    print(f"baseline rows: {len(base_rows)}, new rows: {len(new_rows)}\n")

    base_dist: dict[str, int] = {}
    new_dist: dict[str, int] = {}
    for r in base_rows:
        s = r.get("wasm_status", "unknown")
        base_dist[s] = base_dist.get(s, 0) + 1
    for r in new_rows:
        s = classify(r.get("wasm", {}))
        new_dist[s] = new_dist.get(s, 0) + 1

    print("STATUS DISTRIBUTION")
    keys = sorted(set(list(base_dist.keys()) + list(new_dist.keys())))
    print(f"  {'status':<14}  {'baseline':>10}  {'new':>10}  {'delta':>10}")
    for k in keys:
        b = base_dist.get(k, 0)
        n = new_dist.get(k, 0)
        d = n - b
        marker = " up" if d > 0 else (" down" if d < 0 else "")
        print(f"  {k:<14}  {b:>10}  {n:>10}  {d:>+10}{marker}")
    print()

    base_pass_total = sum(r.get("wasm_passed", 0) for r in base_rows)
    new_pass_total = sum(r.get("wasm", {}).get("passed", 0) for r in new_rows)
    print(
        f"TOTAL TESTS PASSING: baseline={base_pass_total}  "
        f"new={new_pass_total}  delta={new_pass_total - base_pass_total:+d}\n"
    )

    regressions: list[tuple] = []
    improvements: list[tuple] = []
    for stem in sorted(set(list(base_by_stem.keys()) + list(new_by_stem.keys()))):
        bs = base_by_stem.get(stem, {}).get("wasm_status", "missing")
        bp = base_by_stem.get(stem, {}).get("wasm_passed", 0)
        bt = base_by_stem.get(stem, {}).get("wasm_total", 0)
        nrow = new_by_stem.get(stem)
        if not nrow:
            ns, np, nt = "missing", 0, 0
        else:
            w = nrow.get("wasm", {})
            ns = classify(w)
            np = w.get("passed", 0)
            nt = w.get("total", 0)
        if RANK.get(ns, 0) > RANK.get(bs, 0):
            improvements.append((stem, bs, bp, bt, ns, np, nt))
        elif RANK.get(ns, 0) < RANK.get(bs, 0):
            regressions.append((stem, bs, bp, bt, ns, np, nt))
        elif np != bp:
            (improvements if np > bp else regressions).append(
                (stem, bs, bp, bt, ns, np, nt),
            )

    print(f"IMPROVEMENTS ({len(improvements)}):")
    for stem, bs, bp, bt, ns, np, nt in improvements:
        print(f"  {stem:<18}  {bs:<12} P={bp}/{bt}  ->  {ns:<12} P={np}/{nt}")
    print(f"\nREGRESSIONS ({len(regressions)}):")
    for stem, bs, bp, bt, ns, np, nt in regressions:
        print(f"  {stem:<18}  {bs:<12} P={bp}/{bt}  ->  {ns:<12} P={np}/{nt}")


if __name__ == "__main__":
    main()
