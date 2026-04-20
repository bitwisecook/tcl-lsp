#!/usr/bin/env python3
"""Micro-bench harness for a compiled Tcl-to-WASM module.

Compile a ``.tcl`` source (or load a pre-built ``.wasm``), instantiate
the module once under ``wasmtime``, and call the module entry point in
a loop — measuring per-call wall-time so runtime changes (bulk-memset
``frame_push``, proc-lookup LRU, diag-site skipping) can be attributed
to concrete deltas.

The runtime's ``_start`` reactor entry runs the full top-level script
on every call — each invocation re-registers procs, re-evaluates the
source, and prints to stdout the same way it did on the previous
call.  That's intentional: we are measuring the end-to-end bundle
execution cost, which is exactly what dominates ``make test-tcl9``
wall time.

Example::

    python scripts/bench_wasm_runtime.py --source tmp/tcl9.0.3/tests/misc.test \\
        --iterations 50 --warmup 5 --json > after.json

    python scripts/bench_wasm_runtime.py --source tmp/tcl9.0.3/tests/misc.test \\
        --iterations 50 --warmup 5 --json > before.json

    python scripts/bench_diff.py before.json after.json

The ``--dispatch-bench`` mode generates a synthetic
tcltest-``test``-dispatch script that isolates per-test overhead
from any command-specific work.
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
import tempfile
import time
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(_REPO_ROOT))

from tests.external.run_tcl9_tests import _bundle  # noqa: E402
from tests.test_wasm_real_tcl import (  # noqa: E402
    _compile_tcl_with_diag,
    _run_wasm,
)

_BENCH_DISPATCH_TCL = r"""
if {"::tcltest" ni [namespace children]} {
    package require tcltest 2.5
    namespace import -force ::tcltest::*
}
::tcltest::loadTestedCommands

# 200 trivial test dispatches — isolates the tcltest::test
# prologue+uplevel+epilogue cost per call.  Each test body is a
# noop ``expr {1+1}`` so the numbers reflect dispatch overhead,
# not body work.  Higher counts (~500+) currently hit a
# pre-existing runtime limit (frame local table full /
# out-of-bounds alloc); 200 is well below it and still gives
# measurable dispatch cost.
for {set i 1} {$i <= 200} {incr i} {
    test dispatch-$i {} {
        expr {1 + 1}
    } 2
}

cleanupTests
"""


def _compile_to_wasm(source_path: Path, filename: str | None = None) -> bytes:
    """Compile a .tcl source file to WASM using the tcltest bundle path."""
    fname = filename or source_path.name
    bundle = _bundle(source_path)
    wasm, _diag = _compile_tcl_with_diag(bundle, fname)
    return wasm


def _run_once(wasm: bytes, preopen_dir: str) -> float:
    """Run ``_run_wasm`` once and return wall-time ns.

    ``_run_wasm`` creates a fresh ``Store`` + linker per call (the
    Zig runtime keeps module-level mutable state in linear memory,
    so a single store instance cannot be safely reused across runs).
    This is the same path ``tests.external.run_tcl9_tests`` uses,
    making the bench numbers directly comparable to triage
    wall-time.
    """
    t0 = time.perf_counter_ns()
    _run_wasm(wasm, capture_stdout=True, capture_stderr=True, preopen_tmpdir=preopen_dir)
    return time.perf_counter_ns() - t0


def _percentile(values: list[float], pct: float) -> float:
    if not values:
        return 0.0
    return statistics.quantiles(values, n=100, method="inclusive")[int(pct) - 1]


def _summarise(samples_ns: list[float]) -> dict[str, float]:
    return {
        "iterations": len(samples_ns),
        "mean_ns": statistics.fmean(samples_ns),
        "stdev_ns": statistics.pstdev(samples_ns) if len(samples_ns) > 1 else 0.0,
        "min_ns": min(samples_ns),
        "max_ns": max(samples_ns),
        "p50_ns": statistics.median(samples_ns),
        "p90_ns": _percentile(samples_ns, 90),
        "p99_ns": _percentile(samples_ns, 99),
        "total_s": sum(samples_ns) / 1e9,
    }


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    src = ap.add_mutually_exclusive_group(required=True)
    src.add_argument("--source", type=Path, help="Path to a .tcl source file")
    src.add_argument("--bundle", type=Path, help="Path to a prebuilt .wasm")
    src.add_argument(
        "--dispatch-bench",
        action="store_true",
        help="Use the synthetic 1000-test dispatch benchmark",
    )
    ap.add_argument("--iterations", type=int, default=20)
    ap.add_argument("--warmup", type=int, default=2)
    ap.add_argument("--json", action="store_true", help="Emit JSON to stdout")
    ap.add_argument(
        "--label",
        type=str,
        default=None,
        help="Free-form label recorded in the JSON output",
    )
    args = ap.parse_args()

    if args.bundle:
        wasm = args.bundle.read_bytes()
        label = args.label or args.bundle.name
    elif args.source:
        wasm = _compile_to_wasm(args.source)
        label = args.label or args.source.name
    else:
        with tempfile.NamedTemporaryFile(mode="w", suffix=".tcl", delete=False) as tmp:
            tmp.write(_BENCH_DISPATCH_TCL)
            tmp_path = Path(tmp.name)
        wasm = _compile_to_wasm(tmp_path, filename="dispatch-bench.tcl")
        tmp_path.unlink()
        label = args.label or "dispatch-bench"

    with tempfile.TemporaryDirectory(prefix="wasmbench-") as d:
        # Warmup: not recorded.
        for _ in range(args.warmup):
            _run_once(wasm, d)
        # Measured iterations.
        samples = [_run_once(wasm, d) for _ in range(args.iterations)]

    summary = _summarise(samples)
    summary["label"] = label
    summary["wasm_bytes"] = len(wasm)

    if args.json:
        json.dump(summary, sys.stdout, indent=2, sort_keys=True)
        sys.stdout.write("\n")
    else:
        print(f"Label:       {label}")
        print(f"Iterations:  {summary['iterations']} ({args.warmup} warmup)")
        print(f"WASM bytes:  {summary['wasm_bytes']}")
        print(f"Mean:        {summary['mean_ns'] / 1e6:.2f} ms")
        print(f"Stdev:       {summary['stdev_ns'] / 1e6:.2f} ms")
        print(f"p50 / p90:   {summary['p50_ns'] / 1e6:.2f} / {summary['p90_ns'] / 1e6:.2f} ms")
        print(f"p99:         {summary['p99_ns'] / 1e6:.2f} ms")
        print(f"Min / Max:   {summary['min_ns'] / 1e6:.2f} / {summary['max_ns'] / 1e6:.2f} ms")
        print(f"Total:       {summary['total_s']:.3f} s")
    return 0


if __name__ == "__main__":
    sys.exit(main())
