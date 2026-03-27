#!/usr/bin/env python3
"""Benchmark C++ native core types vs pure-Python implementations.

Measures timing and memory (Python heap, process RSS, and C++ heap via
mallinfo2) for SourcePosition, Range, and DocumentBuffer operations.

    # Python-only:
    PYTHONPATH=. python3 scripts/bench_native_types.py

    # With C++ native module:
    PYTHONPATH=builddir/native:. python3 scripts/bench_native_types.py
"""

from __future__ import annotations

import gc
import resource
import time
import tracemalloc

# Detect native module availability.
try:
    from _tcl_lsp_native import DocumentBuffer as NativeDocumentBuffer
    from _tcl_lsp_native import Range as NativeRange
    from _tcl_lsp_native import SourcePosition as NativeSourcePosition
    from _tcl_lsp_native import memory_stats as cpp_memory_stats

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


def _fmt_bytes(b: int | float) -> str:
    b = int(b)
    if abs(b) < 1024:
        return f"{b} B"
    if abs(b) < 1024 * 1024:
        return f"{b / 1024:.1f} KB"
    return f"{b / (1024 * 1024):.2f} MB"


def _get_rss_bytes() -> int:
    """Current process RSS in bytes (cross-platform via resource module)."""
    # ru_maxrss is in KB on Linux, bytes on macOS.
    import platform
    usage = resource.getrusage(resource.RUSAGE_SELF)
    if platform.system() == "Darwin":
        return usage.ru_maxrss  # already bytes
    return usage.ru_maxrss * 1024  # KB -> bytes


def _get_cpp_heap() -> int | None:
    """Current C++ heap used_bytes via mallinfo2, or None if unavailable."""
    if not HAS_NATIVE:
        return None
    return cpp_memory_stats().used_bytes


class MemorySnapshot:
    """Capture memory from all three sources: tracemalloc, RSS, C++ heap."""

    def __init__(self):
        self.py_peak: int = 0         # tracemalloc peak (Python heap only)
        self.rss_before: int = 0      # RSS before
        self.rss_after: int = 0       # RSS after
        self.cpp_before: int | None = None  # C++ heap before
        self.cpp_after: int | None = None   # C++ heap after

    @property
    def rss_delta(self) -> int:
        return self.rss_after - self.rss_before

    @property
    def cpp_delta(self) -> int | None:
        if self.cpp_before is None or self.cpp_after is None:
            return None
        return self.cpp_after - self.cpp_before


def bench(label: str, fn, iterations: int = ITERATIONS) -> tuple[float, MemorySnapshot]:
    """Run fn() iterations times, return (ns_per_call, memory_snapshot)."""
    gc.collect()
    gc.disable()

    snap = MemorySnapshot()
    snap.rss_before = _get_rss_bytes()
    snap.cpp_before = _get_cpp_heap()
    tracemalloc.start()

    start = time.perf_counter_ns()
    for _ in range(iterations):
        fn()
    elapsed_ns = time.perf_counter_ns() - start

    _, snap.py_peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()
    snap.rss_after = _get_rss_bytes()
    snap.cpp_after = _get_cpp_heap()
    gc.enable()

    ns_per = elapsed_ns / iterations
    parts = [f"{_fmt_ns(ns_per):>10s}", f"py_peak {_fmt_bytes(snap.py_peak):>10s}"]
    if snap.cpp_delta is not None:
        parts.append(f"cpp_delta {_fmt_bytes(snap.cpp_delta):>10s}")
    print(f"  {label:.<45s} {' '.join(parts)}")
    return ns_per, snap


def bench_memory_holding(label: str, create_fn, count: int = 10) -> MemorySnapshot:
    """Measure memory of holding `count` objects. Returns snapshot."""
    gc.collect()

    snap = MemorySnapshot()
    snap.rss_before = _get_rss_bytes()
    snap.cpp_before = _get_cpp_heap()
    tracemalloc.start()

    objects = [create_fn() for _ in range(count)]

    _, snap.py_peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()
    snap.rss_after = _get_rss_bytes()
    snap.cpp_after = _get_cpp_heap()

    parts = [f"py_peak {_fmt_bytes(snap.py_peak):>10s}"]
    parts.append(f"rss_delta {_fmt_bytes(snap.rss_delta):>10s}")
    if snap.cpp_delta is not None:
        parts.append(f"cpp_delta {_fmt_bytes(snap.cpp_delta):>10s}")
    print(f"  {label:.<45s} {' '.join(parts)}")

    del objects
    gc.collect()
    return snap


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

    bench(f"from_source small ({prefix})", lambda: DB.from_source("hello\nworld"), iterations=10_000)
    bench(f"from_source large ({prefix})", lambda: DB.from_source(LARGE_SOURCE), iterations=100)

    buf = DB.from_source(LARGE_SOURCE)

    bench(f"offset_to_position ({prefix})", lambda: buf.offset_to_position(115_000))
    bench(f"position_to_offset ({prefix})", lambda: buf.position_to_offset(5000, 10))
    bench(f"offset_to_line_col ({prefix})", lambda: buf.offset_to_line_col(115_000))
    bench(f"range_from_offsets ({prefix})", lambda: buf.range_from_offsets(1000, 200_000))
    bench(f"chunk_line_range ({prefix})", lambda: buf.chunk_line_range(1000, 200_000))
    bench(f"lines access ({prefix})", lambda: buf.lines, iterations=10_000)

    print(f"\n  Memory holding (10x large buffers, {prefix}):")
    bench_memory_holding(
        f"10x large buffer ({prefix})",
        lambda: DB.from_source(LARGE_SOURCE),
        count=10,
    )


def main():
    print("=" * 78)
    print("Benchmark: C++ native types vs pure-Python")
    print(f"Native module available: {HAS_NATIVE}")
    print(f"Iterations: {ITERATIONS:,}")
    print(f"Large source: {len(LARGE_SOURCE):,} bytes, {LARGE_SOURCE.count(chr(10)):,} lines")
    print(f"Process RSS at start: {_fmt_bytes(_get_rss_bytes())}")
    if HAS_NATIVE:
        s = cpp_memory_stats()
        print(f"C++ heap at start: used={_fmt_bytes(s.used_bytes)}, "
              f"total={_fmt_bytes(s.total_allocated)}")
    print("=" * 78)

    # Always benchmark Python.
    bench_source_position(PySourcePosition)
    bench_range(PyRange, PySourcePosition)
    bench_document_buffer(PyDocumentBuffer)

    if HAS_NATIVE:
        bench_source_position(NativeSourcePosition)
        bench_range(NativeRange, NativeSourcePosition)
        bench_document_buffer(NativeDocumentBuffer)

        print("\n" + "=" * 78)
        print("COMPARISON SUMMARY — TIMING")
        print("=" * 78)

        comparisons = [
            ("SourcePosition create",
             lambda: PySourcePosition(line=10, character=5, offset=42),
             lambda: NativeSourcePosition(line=10, character=5, offset=42)),
            ("SourcePosition hash",
             lambda: hash(PySourcePosition(1, 2, 3)),
             lambda: hash(NativeSourcePosition(1, 2, 3))),
            ("Range create",
             lambda: PyRange(start=PySourcePosition(0, 0, 0), end=PySourcePosition(1, 1, 1)),
             lambda: NativeRange(start=NativeSourcePosition(0, 0, 0), end=NativeSourcePosition(1, 1, 1))),
        ]

        py_buf = PyDocumentBuffer.from_source(LARGE_SOURCE)
        native_buf = NativeDocumentBuffer.from_source(LARGE_SOURCE)

        comparisons += [
            ("DB.from_source (230KB)",
             lambda: PyDocumentBuffer.from_source(LARGE_SOURCE),
             lambda: NativeDocumentBuffer.from_source(LARGE_SOURCE)),
            ("DB.offset_to_position",
             lambda: py_buf.offset_to_position(115_000),
             lambda: native_buf.offset_to_position(115_000)),
            ("DB.position_to_offset",
             lambda: py_buf.position_to_offset(5000, 10),
             lambda: native_buf.position_to_offset(5000, 10)),
            ("DB.range_from_offsets",
             lambda: py_buf.range_from_offsets(1000, 200_000),
             lambda: native_buf.range_from_offsets(1000, 200_000)),
            ("DB.chunk_line_range",
             lambda: py_buf.chunk_line_range(1000, 200_000),
             lambda: native_buf.chunk_line_range(1000, 200_000)),
        ]

        print(f"\n  {'Operation':<35s} {'Python':>10s} {'C++':>10s} {'Speedup':>10s}")
        print(f"  {'-' * 35} {'-' * 10} {'-' * 10} {'-' * 10}")

        for label, py_fn, native_fn in comparisons:
            gc.collect()
            gc.disable()

            # Use fewer iterations for from_source (it's slow in Python).
            iters = 1_000 if "from_source" in label else ITERATIONS

            start = time.perf_counter_ns()
            for _ in range(iters):
                py_fn()
            py_ns = (time.perf_counter_ns() - start) / iters

            start = time.perf_counter_ns()
            for _ in range(iters):
                native_fn()
            native_ns = (time.perf_counter_ns() - start) / iters

            gc.enable()

            speedup = py_ns / native_ns if native_ns > 0 else float("inf")
            print(f"  {label:<35s} {_fmt_ns(py_ns):>10s} {_fmt_ns(native_ns):>10s} {speedup:>9.1f}x")

        print(f"\n{'=' * 78}")
        print("COMPARISON SUMMARY — MEMORY (10x large buffers)")
        print("=" * 78)

        gc.collect()

        # Python buffers.
        rss_before = _get_rss_bytes()
        cpp_before = _get_cpp_heap()
        tracemalloc.start()
        py_bufs = [PyDocumentBuffer.from_source(LARGE_SOURCE) for _ in range(10)]
        _, py_peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()
        py_rss_delta = _get_rss_bytes() - rss_before
        py_cpp_delta = (_get_cpp_heap() or 0) - (cpp_before or 0)
        del py_bufs
        gc.collect()

        # Native buffers.
        rss_before = _get_rss_bytes()
        cpp_before = _get_cpp_heap()
        tracemalloc.start()
        native_bufs = [NativeDocumentBuffer.from_source(LARGE_SOURCE) for _ in range(10)]
        _, native_peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()
        native_rss_delta = _get_rss_bytes() - rss_before
        native_cpp_delta = (_get_cpp_heap() or 0) - (cpp_before or 0)
        del native_bufs
        gc.collect()

        print(f"\n  {'Metric':<35s} {'Python':>12s} {'C++':>12s}")
        print(f"  {'-' * 35} {'-' * 12} {'-' * 12}")
        print(f"  {'Python heap (tracemalloc)':<35s} {_fmt_bytes(py_peak):>12s} {_fmt_bytes(native_peak):>12s}")
        print(f"  {'C++ heap delta (mallinfo2)':<35s} {_fmt_bytes(py_cpp_delta):>12s} {_fmt_bytes(native_cpp_delta):>12s}")
        print(f"  {'RSS delta (getrusage)':<35s} {_fmt_bytes(py_rss_delta):>12s} {_fmt_bytes(native_rss_delta):>12s}")

        print(f"\n  Note: Python allocates source + line_starts + lines cache on the Python heap.")
        print(f"  C++ allocates source + line_starts on the C++ heap (invisible to tracemalloc).")
        print(f"  RSS captures both heaps. C++ heap delta via mallinfo2 is the true C++ cost.")

    else:
        print("\n[Native module not available — run with PYTHONPATH=builddir/native:.]")


if __name__ == "__main__":
    main()
