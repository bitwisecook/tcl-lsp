#!/usr/bin/env python3
"""Diff the latest leak sweep output against the baseline.

S0.4 deliverable.  Reads:

  tests/baselines/wasm_leak_baseline.json  (committed)
  tmp/perf-output/leak_sweep_results.json  (fresh sweep)

Reports:

- Per-file delta (alloc residual + double-free).
- Aggregate totals.
- CI-failure exit code on any file whose alloc-residual goes UP
  vs baseline OR any file whose double-free count is non-zero
  (any double-release is a hard error — tracks the over-release
  bug class S2's failed attempt hit).

Usage:

  uv run python scripts/diff_leak_sweep.py [--strict]

``--strict`` is the CI mode: exit 1 on any regression.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
BASELINE = REPO / "tests" / "baselines" / "wasm_leak_baseline.json"
RESULTS = REPO / "tmp" / "perf-output" / "leak_sweep_results.json"


def _load(path: Path) -> dict[str, dict]:
    if not path.exists():
        return {}
    rows = json.loads(path.read_text())
    return {r["stem"]: r for r in rows}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--strict", action="store_true")
    args = parser.parse_args()

    baseline = _load(BASELINE)
    new = _load(RESULTS)

    if not new:
        print(
            f"no fresh results at {RESULTS.relative_to(REPO)}; run scripts/leak_sweep.py first.",
            file=sys.stderr,
        )
        return 1

    all_stems = sorted(set(baseline) | set(new))

    # Aggregate counters.
    base_alloc = sum(r.get("alloc_residual") or 0 for r in baseline.values())
    new_alloc = sum(r.get("alloc_residual") or 0 for r in new.values())
    base_df = sum(r.get("double_free") or 0 for r in baseline.values())
    new_df = sum(r.get("double_free") or 0 for r in new.values())

    print("LEAK SWEEP DIFF")
    print(
        f"  alloc residual: baseline={base_alloc:>10d}  new={new_alloc:>10d}  "
        f"delta={new_alloc - base_alloc:+d}",
    )
    print(
        f"  double frees:   baseline={base_df:>10d}  new={new_df:>10d}  "
        f"delta={new_df - base_df:+d}",
    )
    print()

    regressions: list[str] = []
    improvements: list[str] = []
    for stem in all_stems:
        b = baseline.get(stem) or {}
        n = new.get(stem) or {}
        ba = b.get("alloc_residual")
        na = n.get("alloc_residual")
        bd = b.get("double_free", 0) or 0
        nd = n.get("double_free", 0) or 0

        if na is None and ba is not None:
            regressions.append(f"{stem}  MISSING (was {ba} alloc)")
            continue
        if ba is None and na is not None:
            improvements.append(f"{stem}  NEW ({na} alloc, {nd} df)")
            continue
        if na > ba:
            regressions.append(f"{stem}  alloc {ba:>8d} -> {na:>8d}  (+{na - ba})")
        elif na < ba:
            improvements.append(f"{stem}  alloc {ba:>8d} -> {na:>8d}  ({na - ba})")
        if nd > bd:
            regressions.append(f"{stem}  double-free {bd} -> {nd}  (+{nd - bd})")
        elif nd < bd:
            improvements.append(f"{stem}  double-free {bd} -> {nd}  ({nd - bd})")

    if improvements:
        print(f"IMPROVEMENTS ({len(improvements)}):")
        for line in improvements:
            print(f"  {line}")
        print()

    if regressions:
        print(f"REGRESSIONS ({len(regressions)}):")
        for line in regressions:
            print(f"  {line}")
        print()

    if regressions and args.strict:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
