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
from typing import Any

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO))

from tests.external.run_tcl9_tests import _IN_SCOPE, _bundle  # noqa: E402
from tests.test_wasm_real_tcl import (  # noqa: E402
    _run_wasm,
)

TCLSH = REPO / "tmp" / "tcl9.0.3" / "unix" / "tclsh"
TESTS_DIR = REPO / "tmp" / "tcl9.0.3" / "tests"
OUTPUT = REPO / "tmp" / "perf-output"

# Per-file hard cap on the WASM run.  With per-file dynamic budgets
# scaled off the matching tclsh run (see :data:`WASM_TIME_MULT`), this
# acts only as a safety ceiling for files where tclsh itself is slow;
# the dynamic budget is what trims most files.
TIMEOUT_S = 15

# Per-file budgets are derived from the matching tclsh run:
# the WASM run is allowed at most ``WASM_TIME_MULT`` × tclsh wall time
# and ``WASM_MEM_MULT`` × tclsh peak RSS, with a sane floor so suites
# that finish in microseconds (or use trivial RAM) still get a
# reasonable envelope.  The memory cap is the primary defence — it
# catches the runaway-allocation pattern (``obj.test``-style
# loops past the watchdog) cleanly via a ``memory.grow`` failure.
# The time cap is mostly a redundant safety net atop the global
# :data:`TIMEOUT_S` watchdog.  The floors stay generous because
# WASM is materially slower than C on the same workload (heavy
# list operations spend 5–20× tclsh's wall time) and a tight
# multiplier of a microsecond-fast tclsh run would make every
# non-trivial WASM test trap.
WASM_TIME_MULT = 2.0
WASM_MEM_MULT = 4.0
WASM_TIME_FLOOR_S = 10.0
# Memory floor sits at 1 GiB because the WASM runtime is materially
# heavier per-test than C tclsh — bundles like ``mathop.test`` /
# ``expr.test`` instantiate full parse-cache + per-arena state and
# routinely cross 256 MiB even though their tclsh footprint is a
# fraction of that.  The 4× multiplier still trims blatantly broken
# bundles (``obj.test`` was consuming multiple GB before the
# watchdog noticed); this floor only relaxes the lower bound for
# files where tclsh's measured RSS happens to be tiny because the
# test surface is small.
WASM_MEM_FLOOR_BYTES = 1024 * 1024 * 1024
# Hard cap on the Python-side compile step (lower_to_ir → source-inliner →
# build_cfg → wasm_codegen_module).  Without this the sweep can hang
# indefinitely on a single file when any compile stage spins (no
# wasmtime-side epoch deadline reaches Python code).  Keep it well above
# the per-file ``TIMEOUT_S`` budget so legitimate large bundles still
# finish — obj.test compiles in a few seconds normally.
COMPILE_TIMEOUT_S = 60

_SUMMARY_RE = re.compile(r"Total\s+(\d+)\s+Passed\s+(\d*)\s+Skipped\s+(\d*)\s+Failed\s+(\d*)")


def _parse_summary(text: str):
    m = _SUMMARY_RE.search(text)
    if m is None:
        return None
    return tuple(int(x) if x else 0 for x in m.groups())


def run_wasm(
    bundle_src: str,
    label: str,
    source_dir: Path | None = None,
    *,
    timeout_s: float = TIMEOUT_S,
    memory_size: int | None = None,
):
    """Compile + run, return dict with timing + tcltest summary.

    ``timeout_s`` and ``memory_size`` cap the run-step budget against
    the matching tclsh footprint (see :data:`WASM_TIME_MULT` /
    :data:`WASM_MEM_MULT`).  Both default to lenient values so direct
    callers (debug invocations) get the historic envelope.
    """
    out: dict[str, Any] = {"label": label, "compile_error": None, "run_error": None}
    t0 = time.perf_counter_ns()

    # Run the compile step in a SUBPROCESS with a hard timeout so a
    # single hung file can't stall the whole sweep.  Threads were
    # the original implementation but each abandoned worker keeps
    # the full module memory live (8 GB across 30+ files in the
    # tcltest in-scope set), and the garbage collector can't reap
    # threads whose references are still held by the
    # ``ThreadPoolExecutor`` internal data structures.  Subprocesses
    # OS-kill cleanly on timeout via ``proc.kill()``.
    proc = subprocess.Popen(
        [
            sys.executable,
            "-c",
            "import sys, json, base64, pickle\n"
            "src, label, sd = json.loads(sys.stdin.read())\n"
            "sys.path.insert(0, %r)\n"
            "from tests.test_wasm_real_tcl import _compile_tcl_with_diag\n"
            "from pathlib import Path\n"
            "wasm, diag = _compile_tcl_with_diag(src, label, source_dir=Path(sd) if sd else None)\n"
            "sys.stdout.buffer.write(base64.b64encode(pickle.dumps((wasm, diag))))\n" % str(REPO),
        ],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    stdin_payload = json.dumps([bundle_src, label, str(source_dir) if source_dir else None]).encode(
        "utf-8"
    )
    try:
        stdout_b, stderr_b = proc.communicate(input=stdin_payload, timeout=COMPILE_TIMEOUT_S)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.communicate()
        out["compile_error"] = f"compile timeout ({COMPILE_TIMEOUT_S}s)"
        out["compile_ns"] = time.perf_counter_ns() - t0
        return out
    if proc.returncode != 0:
        msg = stderr_b.decode("utf-8", errors="replace")[-400:]
        out["compile_error"] = msg or f"compile worker exit {proc.returncode}"
        out["compile_ns"] = time.perf_counter_ns() - t0
        return out
    try:
        import base64 as _b64
        import pickle as _pickle

        wasm, diag = _pickle.loads(_b64.b64decode(stdout_b))
    except BaseException as exc:
        out["compile_error"] = f"compile worker decode: {exc}"
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
                timeout_s=timeout_s,
                memory_size=memory_size,
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
    """Run tclsh against the test file directly and return timing + summary.

    Captures per-process peak RSS by sampling ``/proc/<pid>/status``'s
    ``VmHWM`` (high-water mark) just before the child exits.
    ``RUSAGE_CHILDREN`` reports a monotonic-max across all wait()-ed
    children — which silently under-counts every subsequent child that
    uses less RAM than the running peak — so it's not useful here.
    The high-water mark is sampled in a loop while the child is alive,
    and the last successful read becomes the reported peak.  On
    timeout the loop is aborted and the most recent sample is used.
    """
    out: dict[str, Any] = {"label": label, "run_error": None, "timed_out": False}
    t0 = time.perf_counter_ns()
    peak_kb = 0
    try:
        proc = subprocess.Popen(
            [str(TCLSH), str(test_path)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=str(TESTS_DIR),
        )
    except OSError as exc:
        out["run_error"] = str(exc)
        out["run_ns"] = time.perf_counter_ns() - t0
        return out
    status_path = Path(f"/proc/{proc.pid}/status")
    deadline = time.perf_counter() + TIMEOUT_S
    while proc.poll() is None:
        if time.perf_counter() > deadline:
            proc.kill()
            proc.communicate()
            out["run_ns"] = time.perf_counter_ns() - t0
            out["timed_out"] = True
            out["maxrss_bytes"] = peak_kb * 1024
            return out
        try:
            for line in status_path.read_text().splitlines():
                if line.startswith("VmHWM:"):
                    peak_kb = max(peak_kb, int(line.split()[1]))
                    break
        except (FileNotFoundError, ProcessLookupError, ValueError, PermissionError):
            # The proc may have exited between poll() and read(); the
            # next poll() will catch that and exit the loop.
            pass
        time.sleep(0.001)
    stdout_b, stderr_b = proc.communicate()
    out["run_ns"] = time.perf_counter_ns() - t0
    out["returncode"] = proc.returncode
    out["maxrss_bytes"] = peak_kb * 1024
    stdout = stdout_b.decode("utf-8", errors="replace")
    stderr = stderr_b.decode("utf-8", errors="replace")
    summary = _parse_summary(stdout)
    if summary:
        out["total"], out["passed"], out["skipped"], out["failed"] = summary
    out["stdout_tail"] = stdout[-300:]
    out["stderr_tail"] = stderr[-300:]
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

        # Run tclsh FIRST so the WASM run can size its budgets off the
        # C VM's footprint — ``WASM_TIME_MULT`` × wall time and
        # ``WASM_MEM_MULT`` × peak RSS, with floors that keep
        # microsecond-fast files from getting unrealistic budgets.
        tclsh = run_tclsh(test_path, f"tcl9_{stem}.test")
        tclsh_run_s = tclsh.get("run_ns", 0) / 1e9
        wasm_time_s = max(WASM_TIME_FLOOR_S, WASM_TIME_MULT * tclsh_run_s)
        # Cap at the global ``TIMEOUT_S`` so a slow tclsh (say 10s)
        # doesn't push WASM past the safety ceiling.
        wasm_time_s = min(wasm_time_s, TIMEOUT_S)
        tclsh_mem = tclsh.get("maxrss_bytes") or 0
        wasm_mem_bytes = max(WASM_MEM_FLOOR_BYTES, int(WASM_MEM_MULT * tclsh_mem))
        wasm = run_wasm(
            bundle_src,
            f"tcl9_{stem}.test",
            source_dir=test_path.parent,
            timeout_s=wasm_time_s,
            memory_size=wasm_mem_bytes,
        )
        wasm["timeout_s"] = wasm_time_s
        wasm["memory_cap_bytes"] = wasm_mem_bytes

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
            else ("compile-FAIL" if wasm.get("compile_error") else "run-FAIL")
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
        cap_ms = wasm.get("timeout_s", 0) * 1e3
        cap_mib = wasm.get("memory_cap_bytes", 0) / (1024 * 1024)
        print(
            f"  wasm: compile {c_ms:6.0f} ms ({wb} B) run {w_ms:7.0f} ms"
            f"  → {w_summary} [cap {cap_ms:.0f} ms / {cap_mib:.0f} MiB]",
            file=sys.stderr,
        )
        print(
            f"  tcl:                                run {t_ms:7.0f} ms  → {t_summary}",
            file=sys.stderr,
        )

        # Flush incrementally so we don't lose the report if a later
        # file crashes Python.
        (OUTPUT / "tcltest_results.json").write_text(json.dumps(rows, indent=2, sort_keys=True))

    print("\nWrote tcltest_results.json", file=sys.stderr)


if __name__ == "__main__":
    try:
        main()
    except BaseException:
        traceback.print_exc()
        raise
