#!/usr/bin/env python3
"""Benchmark C++ native lexer vs pure-Python lexer.

Measures tokenisation timing for various source sizes and complexity levels.

    # Python-only:
    PYTHONPATH=. python3 scripts/bench_lexer.py

    # With C++ native module:
    PYTHONPATH=builddir/native:. python3 scripts/bench_lexer.py
"""

from __future__ import annotations

import gc
import time

# Detect native module availability.
try:
    from _tcl_lsp_native import NativeTclLexer, LexerConfig

    HAS_NATIVE = True
except ImportError:
    HAS_NATIVE = False

from core.parsing.lexer import TclLexer as PyTclLexer

# Test sources of varying sizes.
SMALL_SOURCE = 'set x [expr {$a + $b}]'
MEDIUM_SOURCE = SMALL_SOURCE + '\n' * 100 + 'puts "hello $name"\n' * 100
LARGE_SOURCE = 'set x [expr {$a + $b}]\n' * 10_000  # ~230KB, 10K lines

# Realistic Tcl code with varied token types.
COMPLEX_SOURCE = """
# Configuration
package require http 2.7
namespace eval ::myapp {
    variable version 1.0
    variable config [dict create \\
        host "localhost" \\
        port 8080 \\
        debug true \\
    ]
}

proc ::myapp::handle_request {sock method path} {
    variable config
    set debug [dict get $config debug]
    if {$debug} {
        puts stderr "Request: $method $path"
    }
    switch -exact -- $method {
        GET {
            set body [read_file $path]
            puts $sock "HTTP/1.1 200 OK\\r\\n"
            puts $sock "Content-Length: [string length $body]\\r\\n"
            puts $sock "\\r\\n"
            puts -nonewline $sock $body
        }
        POST {
            set data [read $sock [dict get $::env CONTENT_LENGTH]]
            set result [process_data $data]
            puts $sock "HTTP/1.1 200 OK\\r\\n"
        }
        default {
            puts $sock "HTTP/1.1 405 Method Not Allowed\\r\\n"
        }
    }
}

foreach {key val} [array get ::env] {
    if {[string match "MYAPP_*" $key]} {
        set name [string range $key 6 end]
        dict set ::myapp::config [string tolower $name] $val
    }
}
""" * 20  # ~20KB, varied tokens


def _fmt_ns(ns: float) -> str:
    if ns < 1_000:
        return f"{ns:.0f} ns"
    if ns < 1_000_000:
        return f"{ns / 1_000:.1f} \u00b5s"
    return f"{ns / 1_000_000:.2f} ms"


def bench(label: str, fn, iterations: int) -> float:
    """Run fn() iterations times, return ns per call."""
    gc.collect()
    gc.disable()

    start = time.perf_counter_ns()
    for _ in range(iterations):
        fn()
    elapsed_ns = time.perf_counter_ns() - start

    gc.enable()

    ns_per = elapsed_ns / iterations
    print(f"  {label:.<55s} {_fmt_ns(ns_per):>10s}")
    return ns_per


def bench_python_lexer(source: str, label: str, iterations: int) -> float:
    """Benchmark tokenise_all with the Python lexer."""
    return bench(
        f"{label} (python)",
        lambda: PyTclLexer(source).tokenise_all(),
        iterations,
    )


def bench_native_lexer(source: str, label: str, iterations: int) -> float:
    """Benchmark tokenise_all with the C++ native lexer."""
    return bench(
        f"{label} (native)",
        lambda: NativeTclLexer(source).tokenise_all(),
        iterations,
    )


def main():
    print("=" * 78)
    print("Benchmark: C++ native lexer vs pure-Python lexer")
    print(f"Native module available: {HAS_NATIVE}")
    print("=" * 78)

    test_cases = [
        ("Small (1 line, 22 chars)", SMALL_SOURCE, 50_000),
        ("Medium (200 lines, ~3KB)", MEDIUM_SOURCE, 5_000),
        ("Complex (800 lines, ~20KB)", COMPLEX_SOURCE, 500),
        ("Large (10K lines, ~230KB)", LARGE_SOURCE, 100),
    ]

    # Count tokens for each source.
    print("\nToken counts:")
    for label, source, _ in test_cases:
        toks = PyTclLexer(source).tokenise_all()
        print(f"  {label}: {len(toks)} tokens, {len(source):,} bytes")

    # Benchmark Python lexer.
    print("\n--- Python lexer ---")
    py_results = {}
    for label, source, iters in test_cases:
        py_results[label] = bench_python_lexer(source, label, iters)

    if HAS_NATIVE:
        # Verify native lexer produces the same number of tokens.
        print("\n--- Verification ---")
        for label, source, _ in test_cases:
            py_toks = PyTclLexer(source).tokenise_all()
            native_toks = NativeTclLexer(source).tokenise_all()
            match = len(py_toks) == len(native_toks)
            print(f"  {label}: Python={len(py_toks)}, Native={len(native_toks)} {'OK' if match else 'MISMATCH!'}")

        # Benchmark native lexer.
        print("\n--- Native lexer ---")
        native_results = {}
        for label, source, iters in test_cases:
            native_results[label] = bench_native_lexer(source, label, iters)

        # Comparison.
        print("\n" + "=" * 78)
        print("COMPARISON SUMMARY — LEXER TIMING")
        print("=" * 78)
        print(f"\n  {'Source':<35s} {'Python':>10s} {'C++':>10s} {'Speedup':>10s}")
        print(f"  {'-' * 35} {'-' * 10} {'-' * 10} {'-' * 10}")

        for label, source, iters in test_cases:
            py_ns = py_results[label]
            native_ns = native_results[label]
            speedup = py_ns / native_ns if native_ns > 0 else float("inf")
            print(f"  {label:<35s} {_fmt_ns(py_ns):>10s} {_fmt_ns(native_ns):>10s} {speedup:>9.1f}x")

    else:
        print("\n[Native module not available — run with PYTHONPATH=builddir/native:.]")


if __name__ == "__main__":
    main()
