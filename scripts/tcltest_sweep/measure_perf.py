"""Perf baseline for the Tcl 9 tcltest bundles.

Reports per-bundle:

* ``bundle_bytes`` / ``bundle_lines`` — input script size.
* ``wasm_bytes`` — compiled module size.
* ``compile_ms_min`` / ``compile_ms_median`` — time to lower + CFG +
  codegen across ``n_runs`` invocations.
* ``exec_ms_min`` / ``exec_ms_median`` — time to instantiate in
  wasmtime + run ``::top``.

Uses the same bundle-assembly path as
``tests/external/run_tcl9_tests.py`` so the timings are directly
comparable to the test suite.  Writes a JSON array to
``tests/external/perf-baseline-tcl9.json`` (override with a path
argument).

Usage:

    uv run python scripts/tcltest_sweep/measure_perf.py
    uv run python scripts/tcltest_sweep/measure_perf.py other-baseline.json
"""

from __future__ import annotations

import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from tests.external.run_tcl9_tests import _bundle, _tcl9_test_file  # noqa: E402
from tests.test_wasm_real_tcl import _compile_tcl_with_diag, _run_wasm  # noqa: E402


def measure(name: str, n_runs: int = 3) -> dict:
    """Compile + run the bundle n times and return min timings."""
    test_path = _tcl9_test_file(name + ".test")
    src = _bundle(test_path)
    n_bytes = len(src.encode("utf-8"))
    n_lines = src.count("\n") + 1

    compile_times = []
    wasm_bytes = None
    for _ in range(n_runs):
        t0 = time.perf_counter()
        wasm_bytes, _diag = _compile_tcl_with_diag(src, name + ".test")
        compile_times.append(time.perf_counter() - t0)

    exec_times = []
    for _ in range(n_runs):
        with tempfile.TemporaryDirectory(prefix="perf-") as tmpd:
            t0 = time.perf_counter()
            try:
                _ = _run_wasm(
                    wasm_bytes,
                    capture_stdout=True,
                    capture_stderr=True,
                    preopen_tmpdir=tmpd,
                )
                elapsed = time.perf_counter() - t0
            except Exception:
                elapsed = time.perf_counter() - t0
        exec_times.append(elapsed)

    return {
        "bundle": name,
        "bundle_bytes": n_bytes,
        "bundle_lines": n_lines,
        "wasm_bytes": len(wasm_bytes) if wasm_bytes else 0,
        "compile_ms_min": round(min(compile_times) * 1000, 1),
        "compile_ms_median": round(sorted(compile_times)[len(compile_times) // 2] * 1000, 1),
        "exec_ms_min": round(min(exec_times) * 1000, 1),
        "exec_ms_median": round(sorted(exec_times)[len(exec_times) // 2] * 1000, 1),
    }


if __name__ == "__main__":
    import json

    default_out = Path(__file__).resolve().parents[2] / "tests/external/perf-baseline-tcl9.json"
    out_path = Path(sys.argv[1]) if len(sys.argv) > 1 else default_out
    # Small: llength (~30 tests, passes clean)
    # Medium: concat (also clean, slightly larger)
    # Large: interp (~200 test cases, 106 KB source)
    results = []
    for name in ["llength", "concat", "interp"]:
        print(f"measuring {name}...", file=sys.stderr)
        try:
            r = measure(name, n_runs=3)
            results.append(r)
            print(json.dumps(r, indent=2))
        except Exception as exc:
            print(f"  failed: {exc}", file=sys.stderr)
            results.append({"bundle": name, "error": str(exc)[:200]})
    out_path.write_text(json.dumps(results, indent=2) + "\n")
    print(f"\nWrote {out_path}", file=sys.stderr)
