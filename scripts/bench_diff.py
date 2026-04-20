#!/usr/bin/env python3
"""Diff two bench-harness JSON outputs into a markdown-friendly table.

Usage::

    python scripts/bench_wasm_runtime.py ... --json > before.json
    # apply changes, rebuild runtime
    python scripts/bench_wasm_runtime.py ... --json > after.json
    python scripts/bench_diff.py before.json after.json
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def _delta(before: float, after: float) -> str:
    if before == 0:
        return "n/a"
    delta = (after - before) / before * 100.0
    sign = "+" if delta >= 0 else ""
    return f"{sign}{delta:.1f}%"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("before", type=Path)
    ap.add_argument("after", type=Path)
    args = ap.parse_args()

    b = json.loads(args.before.read_text())
    a = json.loads(args.after.read_text())

    label = a.get("label") or b.get("label") or "<unnamed>"

    rows = [
        ("label", label, label),
        ("iterations", b.get("iterations", 0), a.get("iterations", 0)),
        ("wasm bytes", b.get("wasm_bytes", 0), a.get("wasm_bytes", 0)),
        ("mean (ms)", b["mean_ns"] / 1e6, a["mean_ns"] / 1e6),
        ("p50 (ms)", b["p50_ns"] / 1e6, a["p50_ns"] / 1e6),
        ("p90 (ms)", b["p90_ns"] / 1e6, a["p90_ns"] / 1e6),
        ("p99 (ms)", b["p99_ns"] / 1e6, a["p99_ns"] / 1e6),
        ("min (ms)", b["min_ns"] / 1e6, a["min_ns"] / 1e6),
        ("max (ms)", b["max_ns"] / 1e6, a["max_ns"] / 1e6),
        ("total (s)", b["total_s"], a["total_s"]),
    ]

    print(f"Bench diff — {label}")
    print()
    print("| metric | before | after | Δ% |")
    print("|---|---|---|---|")
    for name, before, after in rows:
        if name in ("label",):
            print(f"| {name} | {before} | {after} | — |")
            continue
        if name in ("iterations", "wasm bytes"):
            print(f"| {name} | {before} | {after} | — |")
            continue
        delta = _delta(before, after)
        print(f"| {name} | {before:.3f} | {after:.3f} | {delta} |")
    return 0


if __name__ == "__main__":
    sys.exit(main())
