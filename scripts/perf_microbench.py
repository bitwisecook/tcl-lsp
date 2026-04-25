#!/usr/bin/env python3
"""Micro-benchmark common Tcl primitives on WASM vs tclsh 9.0.

Each benchmark wraps a tight loop of N iterations and reports
median wall-time minus the per-call baseline (wasmtime store
setup or tclsh process spawn).  This is the apples-to-apples
"work cost" number.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
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


def bench_wasm(src: str, label: str, iters: int = 9, warm: int = 2):
    wasm = _compile(src, label)
    with tempfile.TemporaryDirectory(prefix="micro-") as preopen:
        for _ in range(warm):
            _run_wasm(wasm, preopen)
        return time_n(lambda: _run_wasm(wasm, preopen), iters)


def bench_tclsh(src: str, label: str, iters: int = 9, warm: int = 2):
    src_path = OUTPUT / f"micro_{label}.tcl"
    src_path.write_text(src)
    for _ in range(warm):
        subprocess.run([str(TCLSH), str(src_path)], capture_output=True, timeout=30)
    return time_n(
        lambda: subprocess.run(
            [str(TCLSH), str(src_path)], capture_output=True, timeout=30
        ),
        iters,
    )


def bench_baseline_wasm(iters: int = 11):
    return bench_wasm("# noop\n", "noop", iters=iters)


def bench_baseline_tclsh(iters: int = 11):
    return bench_tclsh("exit 0\n", "noop", iters=iters)


PRIMITIVES = {
    "set+read variable": (
        "for {set i 0} {$i < $N} {incr i} { set v hello; set _ $v }",
        100_000,
    ),
    "incr loop": (
        "set x 0\nfor {set i 0} {$i < $N} {incr i} { incr x }",
        200_000,
    ),
    "expr arithmetic (braced)": (
        "set t 0\nfor {set i 0} {$i < $N} {incr i} { set t [expr {$t + $i * 3 - 1}] }",
        100_000,
    ),
    "list append + lindex": (
        "set L [list]\nfor {set i 0} {$i < $N} {incr i} { lappend L $i }\n"
        "set s 0\nforeach v $L { set s [expr {$s + $v}] }",
        20_000,
    ),
    "string operations": (
        "set s \"\"\nfor {set i 0} {$i < $N} {incr i} { append s x; set len [string length $s] }",
        5_000,
    ),
    "proc call (no args)": (
        "proc f {} { return 42 }\nfor {set i 0} {$i < $N} {incr i} { f }",
        50_000,
    ),
    "proc call (3 args, expr)": (
        "proc add3 {a b c} { return [expr {$a + $b + $c}] }\n"
        "set t 0\nfor {set i 0} {$i < $N} {incr i} { set t [add3 $i $t 1] }",
        50_000,
    ),
    "if/else branch": (
        "set t 0\nfor {set i 0} {$i < $N} {incr i} { "
        "if {$i % 2 == 0} { incr t } else { incr t -1 } }",
        100_000,
    ),
    "foreach over list": (
        "set L {a b c d e f g h i j}\n"
        "set t 0\nfor {set i 0} {$i < $N} {incr i} { "
        "foreach v $L { incr t } }",
        20_000,
    ),
    "dict set+get": (
        "set d [dict create]\nfor {set i 0} {$i < $N} {incr i} { "
        "dict set d k$i $i }\nset t 0\n"
        "for {set i 0} {$i < $N} {incr i} { incr t [dict get $d k$i] }",
        5_000,
    ),
    "namespace+proc lookup": (
        "namespace eval ::ns { proc do {x} { return [expr {$x * 2}] } }\n"
        "set t 0\nfor {set i 0} {$i < $N} {incr i} { set t [::ns::do $i] }",
        20_000,
    ),
}


def main():
    print("Measuring baselines...", file=sys.stderr)
    base_wasm = bench_baseline_wasm()
    base_tcl = bench_baseline_tclsh()
    print(f"  wasm noop median:  {base_wasm / 1e6:6.2f} ms", file=sys.stderr)
    print(f"  tclsh noop median: {base_tcl / 1e6:6.2f} ms", file=sys.stderr)

    out = {
        "baseline": {"wasm_noop_ns": base_wasm, "tclsh_noop_ns": base_tcl},
        "benchmarks": [],
    }

    for label, (template, n) in PRIMITIVES.items():
        src = f"set N {n}\n{template}\n"
        print(f"\n=== {label}  N={n}", file=sys.stderr)
        try:
            wt = bench_wasm(src, label.replace(" ", "_").replace("/", "_"))
        except BaseException as exc:
            print(f"  wasm FAIL: {exc}", file=sys.stderr)
            out["benchmarks"].append({"label": label, "n": n, "wasm_error": str(exc)})
            continue
        try:
            tt = bench_tclsh(src, label.replace(" ", "_").replace("/", "_"))
        except BaseException as exc:
            print(f"  tclsh FAIL: {exc}", file=sys.stderr)
            out["benchmarks"].append({"label": label, "n": n, "wasm_ns": wt, "tcl_error": str(exc)})
            continue

        wasm_work = max(wt - base_wasm, 0)
        tcl_work = max(tt - base_tcl, 0)
        wasm_per_op_ns = wasm_work / n if n else 0
        tcl_per_op_ns = tcl_work / n if n else 0
        ratio = wasm_per_op_ns / tcl_per_op_ns if tcl_per_op_ns else None
        print(
            f"  wasm  total={wt / 1e6:7.2f}ms  work={wasm_work / 1e6:7.2f}ms  per-op={wasm_per_op_ns:7.0f} ns",
            file=sys.stderr,
        )
        print(
            f"  tcl   total={tt / 1e6:7.2f}ms  work={tcl_work / 1e6:7.2f}ms  per-op={tcl_per_op_ns:7.0f} ns",
            file=sys.stderr,
        )
        if ratio is not None:
            print(
                f"  per-op ratio (wasm/tcl): {ratio:5.2f}x  ({'WASM faster' if ratio < 1 else 'tclsh faster'} by {1 / ratio if ratio < 1 else ratio:.2f}x)",
                file=sys.stderr,
            )

        out["benchmarks"].append(
            {
                "label": label,
                "n": n,
                "wasm_total_ns": wt,
                "tclsh_total_ns": tt,
                "wasm_work_ns": wasm_work,
                "tclsh_work_ns": tcl_work,
                "wasm_per_op_ns": wasm_per_op_ns,
                "tclsh_per_op_ns": tcl_per_op_ns,
                "wasm_over_tcl_ratio": ratio,
            }
        )

    (OUTPUT / "microbench_results.json").write_text(
        json.dumps(out, indent=2, sort_keys=True)
    )
    print("\nWrote microbench_results.json", file=sys.stderr)


if __name__ == "__main__":
    main()
