#!/usr/bin/env python3
"""Run the in-scope Tcl 9 tcltest suites on both backends.

Reuses ``tests.external.run_tcl9_tests._IN_SCOPE`` for the test list
and ``_bundle`` for the WASM compilation path. tclsh runs the
``.test`` file directly (its built-in tcltest auto-loads from the
zip-embedded library).

Per file we capture:

  * compile time + WASM bytes (runtime cost of our codegen)
  * WASM wall time + total/passed/skipped/failed (or trap site)
  * tclsh wall time + total/passed/skipped/failed
  * comparison rows ready for the markdown sub-reports

Each file is given a generous timeout (``TIMEOUT_S``) — these test
suites are large and exercise things the WASM runtime doesn't yet
implement, so traps / hangs are expected and must not stop the
sweep.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
import tempfile
import time
import traceback
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO))

from tests.external.run_tcl9_tests import _IN_SCOPE, _bundle  # noqa: E402
from tests.test_wasm_real_tcl import (  # noqa: E402
    _compile_tcl_with_diag,
    _run_wasm,
)

TCLSH = REPO / "tmp" / "tcl9.0.3" / "unix" / "tclsh"
TESTS_DIR = REPO / "tmp" / "tcl9.0.3" / "tests"
OUTPUT = REPO / "tmp" / "perf-output"

TIMEOUT_S = 60

_SUMMARY_RE = re.compile(
    r"Total\s+(\d+)\s+Passed\s+(\d*)\s+Skipped\s+(\d*)\s+Failed\s+(\d*)"
)


def _parse_summary(text: str):
    m = _SUMMARY_RE.search(text)
    if m is None:
        return None
    return tuple(int(x) if x else 0 for x in m.groups())


def run_wasm(bundle_src: str, label: str):
    """Compile + run, return dict with timing + tcltest summary."""
    out = {"label": label, "compile_error": None, "run_error": None}
    t0 = time.perf_counter_ns()
    try:
        wasm, diag = _compile_tcl_with_diag(bundle_src, label)
    except BaseException as exc:
        out["compile_error"] = str(exc)[-400:]
        out["compile_ns"] = time.perf_counter_ns() - t0
        return out
    out["compile_ns"] = time.perf_counter_ns() - t0
    out["wasm_bytes"] = len(wasm)

    with tempfile.TemporaryDirectory(prefix="tcl9bench-") as preopen:
        try:
            t0 = time.perf_counter_ns()
            ret = _run_wasm(
                wasm,
                capture_stdout=True,
                capture_stderr=True,
                preopen_tmpdir=preopen,
            )
            out["run_ns"] = time.perf_counter_ns() - t0
            stdout = ret[1] if len(ret) >= 2 else ""
            stderr = ret[2] if len(ret) >= 3 else ""
            summary = _parse_summary(stdout)
            if summary:
                out["total"], out["passed"], out["skipped"], out["failed"] = summary
            out["stdout_tail"] = stdout[-300:]
            out["stderr_tail"] = stderr[-300:]
        except BaseException as exc:
            out["run_ns"] = time.perf_counter_ns() - t0
            out["run_error"] = str(exc)[-400:]
            out["stdout_tail"] = (getattr(exc, "tcl_stdout", "") or "")[-300:]
            out["stderr_tail"] = (getattr(exc, "tcl_stderr", "") or "")[-300:]
    return out


def run_tclsh(test_path: Path, label: str):
    """Run tclsh against the test file directly and return timing + summary."""
    out = {"label": label, "run_error": None, "timed_out": False}
    t0 = time.perf_counter_ns()
    try:
        proc = subprocess.run(
            [str(TCLSH), str(test_path)],
            capture_output=True,
            text=True,
            timeout=TIMEOUT_S,
            cwd=str(TESTS_DIR),
        )
    except subprocess.TimeoutExpired:
        out["run_ns"] = time.perf_counter_ns() - t0
        out["timed_out"] = True
        return out
    out["run_ns"] = time.perf_counter_ns() - t0
    out["returncode"] = proc.returncode
    summary = _parse_summary(proc.stdout)
    if summary:
        out["total"], out["passed"], out["skipped"], out["failed"] = summary
    out["stdout_tail"] = proc.stdout[-300:]
    out["stderr_tail"] = proc.stderr[-300:]
    return out


def main():
    print(f"tclsh:        {TCLSH}", file=sys.stderr)
    print(f"in-scope: {len(_IN_SCOPE)} test files", file=sys.stderr)

    rows = []
    for stem, subsystem in _IN_SCOPE:
        test_path = TESTS_DIR / f"{stem}.test"
        if not test_path.exists():
            print(f"  [{stem}] missing test file, skipping", file=sys.stderr)
            continue
        print(f"\n=== {stem}.test ({subsystem}) ===", file=sys.stderr)

        # Build the bundle (tcltest + preamble + test) for WASM.
        try:
            bundle_src = _bundle(test_path)
        except BaseException as exc:
            print(f"  bundle FAIL: {exc}", file=sys.stderr)
            rows.append(
                {
                    "stem": stem,
                    "subsystem": subsystem,
                    "bundle_error": str(exc)[-400:],
                }
            )
            continue
        bundle_size = len(bundle_src.encode("utf-8"))
        try:
            test_size = len(test_path.read_text(encoding="utf-8"))
        except UnicodeDecodeError:
            test_size = test_path.stat().st_size

        wasm = run_wasm(bundle_src, f"tcl9_{stem}.test")
        tclsh = run_tclsh(test_path, f"tcl9_{stem}.test")

        row = {
            "stem": stem,
            "subsystem": subsystem,
            "test_bytes": test_size,
            "bundle_bytes": bundle_size,
            "wasm": wasm,
            "tclsh": tclsh,
        }
        rows.append(row)

        # Pretty status line for the log.
        w_summary = (
            f"{wasm.get('passed', 0)}P/{wasm.get('failed', 0)}F/"
            f"{wasm.get('skipped', 0)}S of {wasm.get('total', 0)}"
            if "total" in wasm
            else (
                "compile-FAIL" if wasm.get("compile_error")
                else "run-FAIL"
            )
        )
        t_summary = (
            f"{tclsh.get('passed', 0)}P/{tclsh.get('failed', 0)}F/"
            f"{tclsh.get('skipped', 0)}S of {tclsh.get('total', 0)}"
            if "total" in tclsh
            else ("TIMEOUT" if tclsh.get("timed_out") else "no-summary")
        )
        w_ms = wasm.get("run_ns", 0) / 1e6
        t_ms = tclsh.get("run_ns", 0) / 1e6
        c_ms = wasm.get("compile_ns", 0) / 1e6
        wb = wasm.get("wasm_bytes", 0)
        print(
            f"  wasm: compile {c_ms:6.0f} ms ({wb} B) run {w_ms:7.0f} ms  → {w_summary}",
            file=sys.stderr,
        )
        print(
            f"  tcl:                                run {t_ms:7.0f} ms  → {t_summary}",
            file=sys.stderr,
        )

        # Flush incrementally so we don't lose the report if a later
        # file crashes Python.
        (OUTPUT / "tcltest_results.json").write_text(
            json.dumps(rows, indent=2, sort_keys=True)
        )

    print("\nWrote tcltest_results.json", file=sys.stderr)


if __name__ == "__main__":
    try:
        main()
    except BaseException:
        traceback.print_exc()
        raise
