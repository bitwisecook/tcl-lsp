#!/usr/bin/env python3
"""Diff the latest leak sweep output against the baseline.

S0.4 deliverable.  Reads:

  tests/baselines/wasm_leak_baseline.json  (committed)
  tmp/perf-output/leak_sweep_results.json  (fresh sweep)

Reports:

- Per-file delta (alloc residual, ``double_free``, ``retain_after_defer``).
- Aggregate totals.
- CI-failure exit code (in ``--strict``) on any of:
    * ``alloc_residual`` goes up vs baseline (leak introduced or grown),
    * ``double_free`` count is non-zero anywhere — any true
      over-release / UAF-release is a hard error tracking the
      over-release bug class S2's failed attempt hit,
    * ``retain_after_defer`` count goes up vs baseline.
  ``retain_after_defer != 0`` itself is NOT a hard error — the runtime
  bails defensively (no slab corruption) but the slot is missing the
  logical reference it thinks it owns.  Treat it as a quality metric
  that signals where retain discipline is sloppy.

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
    base_rad = sum(r.get("retain_after_defer") or 0 for r in baseline.values())
    new_rad = sum(r.get("retain_after_defer") or 0 for r in new.values())

    print("LEAK SWEEP DIFF")
    print(
        f"  alloc residual:     baseline={base_alloc:>10d}  new={new_alloc:>10d}  "
        f"delta={new_alloc - base_alloc:+d}",
    )
    print(
        f"  double frees:       baseline={base_df:>10d}  new={new_df:>10d}  "
        f"delta={new_df - base_df:+d}",
    )
    print(
        f"  retain-after-defer: baseline={base_rad:>10d}  new={new_rad:>10d}  "
        f"delta={new_rad - base_rad:+d}",
    )
    print()

    regressions: list[str] = []
    improvements: list[str] = []
    hard_errors: list[str] = []
    for stem in all_stems:
        b = baseline.get(stem) or {}
        n = new.get(stem) or {}
        ba = b.get("alloc_residual")
        na = n.get("alloc_residual")
        bd = b.get("double_free", 0) or 0
        nd = n.get("double_free", 0) or 0
        brad = b.get("retain_after_defer", 0) or 0
        nrad = n.get("retain_after_defer", 0) or 0

        if na is None and ba is not None:
            regressions.append(f"{stem}  MISSING (was {ba} alloc)")
            continue
        if ba is None and na is not None:
            improvements.append(f"{stem}  NEW ({na} alloc, {nd} df, {nrad} rad)")
            if nd != 0:
                hard_errors.append(f"{stem}  double-free non-zero ({nd}) — true over-release")
            continue
        if na > ba:
            regressions.append(f"{stem}  alloc {ba:>8d} -> {na:>8d}  (+{na - ba})")
        elif na < ba:
            improvements.append(f"{stem}  alloc {ba:>8d} -> {na:>8d}  ({na - ba})")
        if nd > bd:
            regressions.append(f"{stem}  double-free {bd} -> {nd}  (+{nd - bd})")
        elif nd < bd:
            improvements.append(f"{stem}  double-free {bd} -> {nd}  ({nd - bd})")
        if nrad > brad:
            regressions.append(f"{stem}  retain-after-defer {brad} -> {nrad}  (+{nrad - brad})")
        elif nrad < brad:
            improvements.append(f"{stem}  retain-after-defer {brad} -> {nrad}  ({nrad - brad})")
        if nd != 0:
            hard_errors.append(f"{stem}  double-free non-zero ({nd}) — true over-release")

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

    if hard_errors:
        print(f"HARD ERRORS ({len(hard_errors)}):")
        for line in hard_errors:
            print(f"  {line}")
        print()

    # Hard-error gate: any double_free > 0 fails strict mode regardless
    # of baseline movement.  Soft gate: regressions in alloc residual
    # or retain_after_defer fail strict only if they grew vs baseline.
    if args.strict and (hard_errors or regressions):
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
