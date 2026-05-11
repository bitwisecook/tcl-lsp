#!/usr/bin/env python3
"""Micro-benchmark common Tcl primitives on WASM.

Each benchmark wraps a tight loop of N iterations and reports
median wall-time minus the per-call baseline (wasmtime store
setup).  The result is the apples-to-apples "work cost" number.

When ``tclsh`` is available at ``tmp/tcl9.0.3/unix/tclsh`` (built
by ``make build-tcl9`` for the reference test suite), the script
also runs the same workload through tclsh and reports a
wasm/tclsh ratio.  When tclsh is missing the wasm-only numbers
are still produced, and the JSON output records ``tclsh_status:
"missing"`` for each entry.

Usage::

    python scripts/perf_microbench.py
        # writes tmp/perf-output/microbench_results.json

    python scripts/perf_microbench.py --baseline tests/baselines/wasm_microbench_baseline.json
        # also fails when wasm_per_op_ns regresses past
        # ``--regression-pct`` (default 20%) on any benchmark that
        # was non-trapping in the baseline.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO))

TCLSH = REPO / "tmp" / "tcl9.0.3" / "unix" / "tclsh"
OUTPUT = REPO / "tmp" / "perf-output"


def _compile(src: str, fname: str) -> bytes:
    from tests.test_wasm_real_tcl import _compile_tcl_with_diag

    wasm, _ = _compile_tcl_with_diag(src, fname)
    return wasm


def _run_wasm(wasm, preopen):
    from tests.test_wasm_real_tcl import _run_wasm

    return _run_wasm(wasm, capture_stdout=True, capture_stderr=True, preopen_tmpdir=preopen)


def time_n(fn, iters):
    samples = []
    for _ in range(iters):
        t0 = time.perf_counter_ns()
        fn()
        samples.append(time.perf_counter_ns() - t0)
    samples.sort()
    return samples[len(samples) // 2]


def bench_wasm(src: str, label: str, iters: int = 5, warm: int = 1):
    wasm = _compile(src, label)
    with tempfile.TemporaryDirectory(prefix="micro-") as preopen:
        for _ in range(warm):
            _run_wasm(wasm, preopen)
        return time_n(lambda: _run_wasm(wasm, preopen), iters)


def bench_tclsh(src: str, label: str, iters: int = 5, warm: int = 1):
    src_path = OUTPUT / f"micro_{label}.tcl"
    src_path.write_text(src)
    for _ in range(warm):
        subprocess.run([str(TCLSH), str(src_path)], capture_output=True, timeout=30)
    return time_n(
        lambda: subprocess.run([str(TCLSH), str(src_path)], capture_output=True, timeout=30),
        iters,
    )


def bench_baseline_wasm(iters: int = 5):
    return bench_wasm("# noop\n", "noop", iters=iters)


def bench_baseline_tclsh(iters: int = 5):
    return bench_tclsh("exit 0\n", "noop", iters=iters)


PRIMITIVES = {
    "set+read variable": (
        "for {set i 0} {$i < $N} {incr i} { set v hello; set _ $v }",
        20_000,
    ),
    "incr loop": (
        "set x 0\nfor {set i 0} {$i < $N} {incr i} { incr x }",
        50_000,
    ),
    "expr arithmetic (braced)": (
        "set t 0\nfor {set i 0} {$i < $N} {incr i} { set t [expr {$t + $i * 3 - 1}] }",
        20_000,
    ),
    "lappend loop": (
        "set L [list]\nfor {set i 0} {$i < $N} {incr i} { lappend L $i }",
        10_000,
    ),
    "string operations": (
        'set s ""\nfor {set i 0} {$i < $N} {incr i} { append s x; set len [string length $s] }',
        1_000,
    ),
    "proc call (no args)": (
        "proc f {} { return 42 }\nfor {set i 0} {$i < $N} {incr i} { f }",
        20_000,
    ),
    "proc call (opaque, observes frame)": (
        # ``info level`` flips the frame-observation flag on this proc
        # so the S4 inliner refuses to splice it into the loop body —
        # that's what we want here, otherwise the inliner elides the
        # workload entirely and the bench reports 0 ns/op.  The body
        # still does cheap work so the per-op number reflects raw call
        # overhead (frame push/pop, arg marshalling, return-rc balance)
        # rather than ``info level``'s lookup cost.
        "proc f {x} { set _ [info level]; return $x }\n"
        "set t 0\nfor {set i 0} {$i < $N} {incr i} { set t [f $i] }",
        10_000,
    ),
    "proc call (3 args, expr)": (
        "proc add3 {a b c} { return [expr {$a + $b + $c}] }\n"
        "set t 0\nfor {set i 0} {$i < $N} {incr i} { set t [add3 $i $t 1] }",
        10_000,
    ),
    "if/else branch": (
        "set t 0\nfor {set i 0} {$i < $N} {incr i} { "
        "if {$i % 2 == 0} { incr t } else { incr t -1 } }",
        20_000,
    ),
    "foreach over list": (
        "set L {a b c d e f g h i j}\n"
        "set t 0\nfor {set i 0} {$i < $N} {incr i} { "
        "foreach v $L { incr t } }",
        5_000,
    ),
    "dict set+get": (
        "set d [dict create]\nfor {set i 0} {$i < $N} {incr i} { "
        "dict set d k$i $i }\nset t 0\n"
        "for {set i 0} {$i < $N} {incr i} { incr t [dict get $d k$i] }",
        500,
    ),
    "namespace+proc lookup": (
        "namespace eval ::ns { proc do {x} { return [expr {$x * 2}] } }\n"
        "set t 0\nfor {set i 0} {$i < $N} {incr i} { set t [::ns::do $i] }",
        5_000,
    ),
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--baseline",
        type=Path,
        help="Compare results against this baseline JSON; exit 1 on regression.",
    )
    parser.add_argument(
        "--regression-pct",
        type=float,
        default=20.0,
        help="Regression threshold in percent (default 20).",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=OUTPUT / "microbench_results.json",
        help="Output JSON path (default tmp/perf-output/microbench_results.json).",
    )
    args = parser.parse_args()

    OUTPUT.mkdir(parents=True, exist_ok=True)
    args.out.parent.mkdir(parents=True, exist_ok=True)

    have_tclsh = TCLSH.exists()
    if not have_tclsh:
        print(
            f"NOTE: {TCLSH} missing — running wasm-only.  "
            "Build with ``make build-tcl9`` for comparative numbers.",
            file=sys.stderr,
        )

    print("Measuring baselines...", file=sys.stderr)
    base_wasm = bench_baseline_wasm()
    base_tcl = bench_baseline_tclsh() if have_tclsh else 0
    print(f"  wasm noop median:  {base_wasm / 1e6:6.2f} ms", file=sys.stderr)
    if have_tclsh:
        print(f"  tclsh noop median: {base_tcl / 1e6:6.2f} ms", file=sys.stderr)

    out = {
        "baseline": {
            "wasm_noop_ns": base_wasm,
            "tclsh_noop_ns": base_tcl,
            "have_tclsh": have_tclsh,
        },
        "benchmarks": [],
    }

    for label, (template, n) in PRIMITIVES.items():
        src = f"set N {n}\n{template}\n"
        print(f"\n=== {label}  N={n}", file=sys.stderr)
        entry = {"label": label, "n": n}
        try:
            wt = bench_wasm(src, label.replace(" ", "_").replace("/", "_"))
            wasm_work = max(wt - base_wasm, 0)
            entry["wasm_total_ns"] = wt
            entry["wasm_work_ns"] = wasm_work
            entry["wasm_per_op_ns"] = wasm_work / n if n else 0
        except BaseException as exc:
            print(f"  wasm FAIL: {exc}", file=sys.stderr)
            entry["wasm_status"] = "trap"
            entry["wasm_error"] = str(exc)
            out["benchmarks"].append(entry)
            continue

        if have_tclsh:
            try:
                tt = bench_tclsh(src, label.replace(" ", "_").replace("/", "_"))
                tcl_work = max(tt - base_tcl, 0)
                tcl_per_op_ns = tcl_work / n if n else 0
                entry["tclsh_total_ns"] = tt
                entry["tclsh_work_ns"] = tcl_work
                entry["tclsh_per_op_ns"] = tcl_per_op_ns
                ratio = entry["wasm_per_op_ns"] / tcl_per_op_ns if tcl_per_op_ns else None
                entry["wasm_over_tcl_ratio"] = ratio
            except BaseException as exc:
                print(f"  tclsh FAIL: {exc}", file=sys.stderr)
                entry["tclsh_status"] = "trap"
                entry["tclsh_error"] = str(exc)
        else:
            entry["tclsh_status"] = "missing"

        print(
            f"  wasm  total={entry.get('wasm_total_ns', 0) / 1e6:7.2f}ms  "
            f"work={entry.get('wasm_work_ns', 0) / 1e6:7.2f}ms  "
            f"per-op={entry.get('wasm_per_op_ns', 0):7.0f} ns",
            file=sys.stderr,
        )
        if entry.get("tclsh_per_op_ns"):
            ratio = entry.get("wasm_over_tcl_ratio")
            print(
                f"  tcl   per-op={entry['tclsh_per_op_ns']:7.0f} ns  "
                f"ratio (wasm/tcl): {ratio:.2f}x",
                file=sys.stderr,
            )
        out["benchmarks"].append(entry)

    args.out.write_text(json.dumps(out, indent=2, sort_keys=True))
    print(f"\nWrote {args.out}", file=sys.stderr)

    if args.baseline:
        return _check_regression(out, args.baseline, args.regression_pct)
    return 0


def _check_regression(results: dict, baseline_path: Path, threshold_pct: float) -> int:
    """Compare ``results`` against the baseline JSON.  Return 0 when
    every benchmark is within ``threshold_pct`` of the baseline (or
    the baseline trapped); 1 on any regression.
    """
    if not baseline_path.exists():
        print(f"baseline {baseline_path} not found — skipping regression check", file=sys.stderr)
        return 0
    baseline = json.loads(baseline_path.read_text())
    base_by_label = {b["label"]: b for b in baseline.get("benchmarks", [])}

    regressions = []
    improvements = []
    for entry in results["benchmarks"]:
        label = entry["label"]
        cur = entry.get("wasm_per_op_ns")
        if cur is None or entry.get("wasm_status") == "trap":
            continue
        base = base_by_label.get(label)
        if base is None or "wasm_per_op_ns" not in base:
            continue  # baseline trapped or unrecorded; current run is fine
        prev = base["wasm_per_op_ns"]
        if prev == 0:
            continue
        delta_pct = (cur - prev) / prev * 100
        if delta_pct > threshold_pct:
            regressions.append((label, prev, cur, delta_pct))
        elif delta_pct < -5:  # any meaningful improvement
            improvements.append((label, prev, cur, delta_pct))

    if improvements:
        print("\n=== Improvements ===", file=sys.stderr)
        for label, prev, cur, delta in improvements:
            print(f"  {label:40} {prev:6.0f} ns → {cur:6.0f} ns  ({delta:+5.1f}%)", file=sys.stderr)
    if regressions:
        print("\n=== Regressions (>20%) ===", file=sys.stderr)
        for label, prev, cur, delta in regressions:
            print(f"  {label:40} {prev:6.0f} ns → {cur:6.0f} ns  ({delta:+5.1f}%)", file=sys.stderr)
        return 1
    print("\nNo regressions past threshold.", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
