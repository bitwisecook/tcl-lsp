#!/usr/bin/env python3
"""Benchmark C++ native core types vs pure-Python implementations.

Measures timing and memory for SourcePosition, Range, and DocumentBuffer
operations. Run with the native module on PYTHONPATH to compare both:

    # Python-only:
    python3 scripts/bench_native_types.py

    # With C++ native module:
    PYTHONPATH=builddir/native python3 scripts/bench_native_types.py
"""

from __future__ import annotations

import gc
import os
import sys
import time
import tracemalloc

# Detect native module availability.
try:
    from _tcl_lsp_native import DocumentBuffer as NativeDocumentBuffer
    from _tcl_lsp_native import Range as NativeRange
    from _tcl_lsp_native import SourcePosition as NativeSourcePosition

    HAS_NATIVE = True
except ImportError:
    HAS_NATIVE = False

from core.analysis.semantic_model import Range as PyRange
from core.common.document_buffer import DocumentBuffer as PyDocumentBuffer
from core.parsing.tokens import SourcePosition as PySourcePosition

ITERATIONS = 100_000
LARGE_SOURCE = "set x [expr {$a + $b}]\n" * 10_000  # ~230KB, 10K lines


def _fmt_ns(ns: float) -> str:
    if ns < 1_000:
        return f"{ns:.0f} ns"
    if ns < 1_000_000:
        return f"{ns / 1_000:.1f} \u00b5s"
    return f"{ns / 1_000_000:.2f} ms"


def _fmt_bytes(b: int) -> str:
    if b < 1024:
        return f"{b} B"
    if b < 1024 * 1024:
        return f"{b / 1024:.1f} KB"
    return f"{b / (1024 * 1024):.2f} MB"


def bench(label: str, fn, iterations: int = ITERATIONS) -> tuple[float, int]:
    """Run fn() iterations times, return (ns_per_call, peak_memory_bytes)."""
    gc.collect()
    gc.disable()
    tracemalloc.start()

    start = time.perf_counter_ns()
    for _ in range(iterations):
        fn()
    elapsed_ns = time.perf_counter_ns() - start

    _, peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()
    gc.enable()

    ns_per = elapsed_ns / iterations
    print(f"  {label:.<50s} {_fmt_ns(ns_per):>10s}  peak {_fmt_bytes(peak):>10s}")
    return ns_per, peak


def bench_source_position(SP):
    name = SP.__module__ if hasattr(SP, "__module__") else type(SP).__name__
    prefix = "native" if "native" in str(name) else "python"

    print(f"\n--- SourcePosition ({prefix}) ---")
    bench(f"create ({prefix})", lambda: SP(line=10, character=5, offset=42))
    p = SP(line=10, character=5, offset=42)
    bench(f"hash ({prefix})", lambda: hash(p))
    q = SP(line=10, character=5, offset=42)
    bench(f"eq ({prefix})", lambda: p == q)
    bench(f"repr ({prefix})", lambda: repr(p))


def bench_range(R, SP):
    prefix = "native" if "native" in str(getattr(R, "__module__", "")) else "python"

    print(f"\n--- Range ({prefix}) ---")
    s = SP(line=0, character=0, offset=0)
    e = SP(line=5, character=10, offset=50)
    bench(f"create ({prefix})", lambda: R(start=s, end=e))
    r = R(start=s, end=e)
    bench(f"hash ({prefix})", lambda: hash(r))
    bench(f"zero ({prefix})", lambda: R.zero())


def bench_document_buffer(DB):
    prefix = "native" if "native" in str(getattr(DB, "__module__", "")) else "python"

    print(f"\n--- DocumentBuffer ({prefix}) ---")

    # Construction.
    bench(f"from_source small ({prefix})", lambda: DB.from_source("hello\nworld"), iterations=10_000)
    bench(f"from_source large ({prefix})", lambda: DB.from_source(LARGE_SOURCE), iterations=100)

    buf = DB.from_source(LARGE_SOURCE)

    # Position conversions.
    bench(f"offset_to_position ({prefix})", lambda: buf.offset_to_position(115_000))
    bench(f"position_to_offset ({prefix})", lambda: buf.position_to_offset(5000, 10))
    bench(f"offset_to_line_col ({prefix})", lambda: buf.offset_to_line_col(115_000))
    bench(f"range_from_offsets ({prefix})", lambda: buf.range_from_offsets(1000, 200_000))
    bench(f"chunk_line_range ({prefix})", lambda: buf.chunk_line_range(1000, 200_000))

    # Lines access.
    bench(f"lines access ({prefix})", lambda: buf.lines, iterations=10_000)

    # Memory for holding the buffer.
    gc.collect()
    tracemalloc.start()
    bufs = [DB.from_source(LARGE_SOURCE) for _ in range(10)]
    _, peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()
    print(f"  {'10x large buffer memory (' + prefix + ')':.<50s} {'':>10s}  peak {_fmt_bytes(peak):>10s}")
    del bufs


def main():
    print("=" * 72)
    print(f"Benchmark: C++ native types vs pure-Python")
    print(f"Native module available: {HAS_NATIVE}")
    print(f"Iterations: {ITERATIONS}")
    print(f"Large source: {len(LARGE_SOURCE)} bytes, {LARGE_SOURCE.count(chr(10))} lines")
    print("=" * 72)

    # Always benchmark Python.
    bench_source_position(PySourcePosition)
    bench_range(PyRange, PySourcePosition)
    bench_document_buffer(PyDocumentBuffer)

    if HAS_NATIVE:
        bench_source_position(NativeSourcePosition)
        bench_range(NativeRange, NativeSourcePosition)
        bench_document_buffer(NativeDocumentBuffer)

        print("\n" + "=" * 72)
        print("COMPARISON SUMMARY")
        print("=" * 72)

        # Quick head-to-head for key operations.
        comparisons = [
            ("SourcePosition create", lambda: PySourcePosition(line=10, character=5, offset=42),
             lambda: NativeSourcePosition(line=10, character=5, offset=42)),
            ("SourcePosition hash", lambda: hash(PySourcePosition(1, 2, 3)),
             lambda: hash(NativeSourcePosition(1, 2, 3))),
            ("Range create", lambda: PyRange(start=PySourcePosition(0, 0, 0), end=PySourcePosition(1, 1, 1)),
             lambda: NativeRange(start=NativeSourcePosition(0, 0, 0), end=NativeSourcePosition(1, 1, 1))),
        ]

        py_buf = PyDocumentBuffer.from_source(LARGE_SOURCE)
        native_buf = NativeDocumentBuffer.from_source(LARGE_SOURCE)

        comparisons += [
            ("DocumentBuffer.offset_to_position",
             lambda: py_buf.offset_to_position(115_000),
             lambda: native_buf.offset_to_position(115_000)),
            ("DocumentBuffer.position_to_offset",
             lambda: py_buf.position_to_offset(5000, 10),
             lambda: native_buf.position_to_offset(5000, 10)),
        ]

        print(f"\n  {'Operation':<40s} {'Python':>10s} {'C++':>10s} {'Speedup':>10s}")
        print(f"  {'-' * 40} {'-' * 10} {'-' * 10} {'-' * 10}")

        for label, py_fn, native_fn in comparisons:
            gc.collect()
            gc.disable()

            start = time.perf_counter_ns()
            for _ in range(ITERATIONS):
                py_fn()
            py_ns = (time.perf_counter_ns() - start) / ITERATIONS

            start = time.perf_counter_ns()
            for _ in range(ITERATIONS):
                native_fn()
            native_ns = (time.perf_counter_ns() - start) / ITERATIONS

            gc.enable()

            speedup = py_ns / native_ns if native_ns > 0 else float("inf")
            print(f"  {label:<40s} {_fmt_ns(py_ns):>10s} {_fmt_ns(native_ns):>10s} {speedup:>9.1f}x")
    else:
        print("\n[Native module not available — run with PYTHONPATH=builddir/native]")


if __name__ == "__main__":
    main()
