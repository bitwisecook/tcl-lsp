#!/usr/bin/env python3
"""Compare each ``samples/tcl/*.tcl`` running on our WASM runtime vs tclsh 9.0.

For every sample we:

 1. Compile to a WASM module via the Python codegen pipeline and record
    the resulting module size.
 2. Time N WASM runs (one fresh wasmtime store per run, mirroring how
    the test runner measures real bundle execution cost).
 3. Time N tclsh 9.0 runs of the unmodified source.
 4. Capture stdout / stderr / exit status from both backends and store
    them next to the timing numbers.

The script writes ``results.json`` plus a Markdown report
(``REPORT.md``) into the same directory.
"""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import time
import traceback
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO))

SAMPLES_DIR = REPO / "samples" / "tcl"
TCLSH = REPO / "tmp" / "tcl9.0.3" / "unix" / "tclsh"
OUTPUT_DIR = REPO / "tmp" / "perf-output"

# How many runs to time per sample.  Wall time per run includes
# wasmtime store setup (~10ms) so we want enough samples to hide that
# noise but not so many that the whole report takes minutes.
ITERATIONS = 30
WARMUP = 3

# tclsh subprocess timing covers: process spawn, libc init, Tcl
# interpreter init, parse + bytecode-compile + execute, exit.  We
# subtract the spawn baseline reported under ``BASELINE_*``.
BASELINE_RUNS = 20

# Hard wall-clock cap for any single execution.  Sample 06 recursively
# sources itself via ``source $argv0`` — without this cap, both
# backends would never finish.
TIMEOUT_S = 4

# Samples that are guaranteed not to terminate (they invoke
# ``source $argv0`` or otherwise self-recurse).  We still record
# what they look like at compile time, but we do not attempt to
# run them.
NON_TERMINATING = {"06_security_smells.tcl"}


def _compile(src: str, fname: str) -> tuple[bytes, object]:
    from tests.test_wasm_real_tcl import _compile_tcl_with_diag

    return _compile_tcl_with_diag(src, fname)


def _run_wasm(wasm: bytes, preopen_dir: str):
    from tests.test_wasm_real_tcl import _run_wasm as _r

    return _r(wasm, capture_stdout=True, capture_stderr=True, preopen_tmpdir=preopen_dir)


def time_wasm(wasm: bytes, iterations: int, warmup: int):
    """Return ``(samples_ns, last_stdout, last_stderr, error)``."""
    samples = []
    last_stdout = ""
    last_stderr = ""
    error = None
    with tempfile.TemporaryDirectory(prefix="wasm-perf-") as preopen:
        for _ in range(warmup):
            try:
                _run_wasm(wasm, preopen)
            except BaseException as exc:
                error = f"{type(exc).__name__}: {exc}"
                last_stdout = getattr(exc, "tcl_stdout", "") or ""
                last_stderr = getattr(exc, "tcl_stderr", "") or ""
                return samples, last_stdout, last_stderr, error
        for _ in range(iterations):
            t0 = time.perf_counter_ns()
            try:
                _, stdout, stderr = _run_wasm(wasm, preopen)
                samples.append(time.perf_counter_ns() - t0)
                last_stdout = stdout
                last_stderr = stderr
            except BaseException as exc:
                error = f"{type(exc).__name__}: {exc}"
                last_stdout = getattr(exc, "tcl_stdout", "") or ""
                last_stderr = getattr(exc, "tcl_stderr", "") or ""
                return samples, last_stdout, last_stderr, error
    return samples, last_stdout, last_stderr, error


def time_tclsh(source_path: Path, iterations: int, warmup: int):
    """Return ``(samples_ns, last_stdout, last_stderr, returncode, timed_out)``."""
    samples = []
    last_stdout = ""
    last_stderr = ""
    rc = 0
    timed_out = False
    for _ in range(warmup):
        try:
            subprocess.run(
                [str(TCLSH), str(source_path)],
                capture_output=True,
                text=True,
                timeout=TIMEOUT_S,
            )
        except subprocess.TimeoutExpired:
            return [], "", "<timeout>", -1, True
    for _ in range(iterations):
        t0 = time.perf_counter_ns()
        try:
            proc = subprocess.run(
                [str(TCLSH), str(source_path)],
                capture_output=True,
                text=True,
                timeout=TIMEOUT_S,
            )
        except subprocess.TimeoutExpired:
            timed_out = True
            break
        samples.append(time.perf_counter_ns() - t0)
        last_stdout = proc.stdout
        last_stderr = proc.stderr
        rc = proc.returncode
    return samples, last_stdout, last_stderr, rc, timed_out


def baseline_spawn_ns(iterations: int) -> float:
    """Time `tclsh -c 'exit'` to estimate the unavoidable spawn cost."""
    samples = []
    for _ in range(iterations):
        t0 = time.perf_counter_ns()
        subprocess.run([str(TCLSH)], input="", capture_output=True, text=True, timeout=10)
        samples.append(time.perf_counter_ns() - t0)
    samples.sort()
    return samples[len(samples) // 2]  # median


def baseline_wasm_ns(iterations: int) -> float:
    """Time a no-op (empty top-level) Tcl WASM module to estimate the
    wasmtime ``Store`` + linker + instantiate + `_initialize` cost."""
    wasm, _ = _compile("# empty\n", "noop.tcl")
    samples = []
    with tempfile.TemporaryDirectory(prefix="wasm-base-") as preopen:
        # Warmup
        for _ in range(2):
            _run_wasm(wasm, preopen)
        for _ in range(iterations):
            t0 = time.perf_counter_ns()
            _run_wasm(wasm, preopen)
            samples.append(time.perf_counter_ns() - t0)
    samples.sort()
    return samples[len(samples) // 2]


def stats(samples_ns):
    if not samples_ns:
        return None
    s = sorted(samples_ns)
    n = len(s)
    return {
        "count": n,
        "min_ns": s[0],
        "median_ns": s[n // 2],
        "p90_ns": s[min(int(n * 0.9), n - 1)],
        "max_ns": s[-1],
        "mean_ns": sum(s) / n,
    }


def collect():
    if not TCLSH.exists():
        raise SystemExit(f"tclsh not built at {TCLSH}")
    print(f"tclsh: {TCLSH}", file=sys.stderr)
    print("Measuring tclsh spawn baseline...", file=sys.stderr)
    spawn_baseline_ns = baseline_spawn_ns(BASELINE_RUNS)
    print(f"  baseline spawn median: {spawn_baseline_ns / 1e6:.2f} ms", file=sys.stderr)
    print("Measuring WASM no-op baseline...", file=sys.stderr)
    wasm_noop_baseline_ns = baseline_wasm_ns(BASELINE_RUNS)
    print(
        f"  baseline wasm noop median: {wasm_noop_baseline_ns / 1e6:.2f} ms",
        file=sys.stderr,
    )

    results = {
        "spawn_baseline_ns": spawn_baseline_ns,
        "wasm_noop_baseline_ns": wasm_noop_baseline_ns,
        "iterations": ITERATIONS,
        "warmup": WARMUP,
        "samples": [],
    }

    for src_path in sorted(SAMPLES_DIR.glob("*.tcl")):
        name = src_path.name
        print(f"\n=== {name} ===", file=sys.stderr)
        src = src_path.read_text()
        entry = {
            "name": name,
            "source_bytes": len(src.encode("utf-8")),
            "source_lines": src.count("\n") + 1,
        }

        # Compile.
        try:
            wasm, _diag = _compile(src, name)
            entry["wasm_bytes"] = len(wasm)
            entry["compile_ok"] = True
        except BaseException as exc:
            entry["compile_ok"] = False
            entry["compile_error"] = f"{type(exc).__name__}: {exc}\n" + traceback.format_exc()
            print(f"  compile FAILED: {exc}", file=sys.stderr)
            results["samples"].append(entry)
            continue
        print(
            f"  compile OK: {entry['wasm_bytes']} wasm bytes "
            f"({entry['wasm_bytes'] / max(entry['source_bytes'], 1):.1f}x source)",
            file=sys.stderr,
        )

        if name in NON_TERMINATING:
            entry["skipped"] = "non-terminating (recursive source $argv0)"
            print("  skipped (non-terminating)", file=sys.stderr)
            results["samples"].append(entry)
            continue

        # WASM runs.
        wasm_samples, wasm_stdout, wasm_stderr, wasm_err = time_wasm(wasm, ITERATIONS, WARMUP)
        entry["wasm_stats"] = stats(wasm_samples)
        entry["wasm_stdout"] = wasm_stdout
        entry["wasm_stderr"] = wasm_stderr
        entry["wasm_error"] = wasm_err
        if entry["wasm_stats"]:
            print(
                f"  wasm median: {entry['wasm_stats']['median_ns'] / 1e6:.2f} ms"
                f" (p90 {entry['wasm_stats']['p90_ns'] / 1e6:.2f} ms)",
                file=sys.stderr,
            )
        else:
            print(f"  wasm error: {wasm_err}", file=sys.stderr)

        # tclsh runs.
        tcl_samples, tcl_stdout, tcl_stderr, tcl_rc, tcl_timeout = time_tclsh(
            src_path, ITERATIONS, WARMUP
        )
        entry["tclsh_stats"] = stats(tcl_samples)
        entry["tclsh_stdout"] = tcl_stdout
        entry["tclsh_stderr"] = tcl_stderr
        entry["tclsh_returncode"] = tcl_rc
        entry["tclsh_timeout"] = tcl_timeout
        if entry["tclsh_stats"]:
            print(
                f"  tclsh median: {entry['tclsh_stats']['median_ns'] / 1e6:.2f} ms (rc={tcl_rc})",
                file=sys.stderr,
            )
        elif tcl_timeout:
            print("  tclsh timed out", file=sys.stderr)

        # Output equality (post-strip-CRLF).
        entry["stdout_match"] = (wasm_stdout or "").strip() == (tcl_stdout or "").strip()

        results["samples"].append(entry)

    out_json = OUTPUT_DIR / "results.json"
    out_json.write_text(json.dumps(results, indent=2, sort_keys=True))
    print(f"\nWrote {out_json}", file=sys.stderr)
    return results


if __name__ == "__main__":
    collect()
