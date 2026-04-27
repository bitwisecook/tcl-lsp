#!/usr/bin/env python3
"""Run amplified versions of each sample so per-call cost dominates.

The single-shot harness shows that wasmtime store setup (~9 ms) and
tclsh process spawn (~72 ms) drown the actual script work (~0.5 ms).
We can't avoid those baselines for a fair real-world end-to-end
number, but to get a useful cost-per-tcl-operation number we wrap
each runnable sample in a ``for`` loop with N iterations and time
the result.

We pick N per sample so the total script work is in the
~30-300 ms range — large enough that store-setup noise becomes
small relative to the work, small enough that the report still
finishes in a reasonable time.
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

SAMPLES_DIR = REPO / "samples" / "tcl"
TCLSH = REPO / "tmp" / "tcl9.0.3" / "unix" / "tclsh"
OUTPUT = REPO / "tmp" / "perf-output"


def _compile_wasm(src: str, fname: str) -> bytes:
    from tests.test_wasm_real_tcl import _compile_tcl_with_diag

    wasm, _ = _compile_tcl_with_diag(src, fname)
    return wasm


def _run_wasm(wasm: bytes, preopen_dir: str):
    from tests.test_wasm_real_tcl import _run_wasm

    return _run_wasm(wasm, capture_stdout=True, capture_stderr=True, preopen_tmpdir=preopen_dir)


def _wrap(body: str, n: int, drop_puts: bool = True) -> str:
    """Return ``body`` repeated ``n`` times by wrapping it in ``for``.

    For samples whose body itself prints, we keep the puts because
    output equality is what we are scoring against tclsh.  For
    micro-benches (compute-only) we strip puts to avoid drowning
    the time in I/O.
    """
    inner = body
    if drop_puts:
        # Naive but adequate for these short samples — strip lines
        # that begin (after optional indent) with `puts`.
        kept = []
        for line in inner.splitlines():
            stripped = line.lstrip()
            if stripped.startswith("puts "):
                continue
            kept.append(line)
        inner = "\n".join(kept)
    return f"for {{set __i 0}} {{$__i < {n}}} {{incr __i}} {{\n{inner}\n}}\n"


# Sample → (iterations, drop_puts)
STRESS_PROFILE = {
    "01_simple_variables.tcl": (5000, True),
    "02_control_flow_braced.tcl": (5000, True),
    "03_procs_and_namespaces.tcl": (2000, True),
    "08_tricky_edge_cases.tcl": (1000, True),
    "11_minifier_demo.tcl": (1000, True),
    "12_safe_on_uninit.tcl": (1, True),  # only proc defs — no body
    "13_incr_dialect_difference.tcl": (1, True),  # only proc defs
    "compiler_explorer_demo.tcl": (2000, True),
}

# Per-call iteration counts when run via subprocess.
WASM_ITERS = 15
TCLSH_ITERS = 15
WASM_WARM = 2
TCLSH_WARM = 2
TIMEOUT_S = 30


def median(samples_ns):
    s = sorted(samples_ns)
    return s[len(s) // 2] if s else None


def time_wasm(wasm: bytes, iters: int, warm: int):
    samples = []
    last_stdout = ""
    err = None
    with tempfile.TemporaryDirectory(prefix="wasm-stress-") as preopen:
        for _ in range(warm):
            try:
                _run_wasm(wasm, preopen)
            except BaseException as exc:
                err = f"{type(exc).__name__}: {exc}"
                return samples, last_stdout, err
        for _ in range(iters):
            t0 = time.perf_counter_ns()
            try:
                _, stdout, _ = _run_wasm(wasm, preopen)
            except BaseException as exc:
                err = f"{type(exc).__name__}: {exc}"
                return samples, last_stdout, err
            samples.append(time.perf_counter_ns() - t0)
            last_stdout = stdout
    return samples, last_stdout, err


def time_tclsh(src_path: Path, iters: int, warm: int):
    samples = []
    last_stdout = ""
    rc = 0
    err = None
    for _ in range(warm):
        try:
            subprocess.run(
                [str(TCLSH), str(src_path)],
                capture_output=True,
                text=True,
                timeout=TIMEOUT_S,
            )
        except subprocess.TimeoutExpired:
            return samples, last_stdout, "timeout", -1
    for _ in range(iters):
        t0 = time.perf_counter_ns()
        try:
            proc = subprocess.run(
                [str(TCLSH), str(src_path)],
                capture_output=True,
                text=True,
                timeout=TIMEOUT_S,
            )
        except subprocess.TimeoutExpired:
            return samples, last_stdout, "timeout", -1
        samples.append(time.perf_counter_ns() - t0)
        last_stdout = proc.stdout
        rc = proc.returncode
    return samples, last_stdout, err, rc


def main():
    out = []
    print(f"tclsh: {TCLSH}", file=sys.stderr)
    for name, (iters, drop_puts) in STRESS_PROFILE.items():
        src_path = SAMPLES_DIR / name
        body = src_path.read_text()
        stress_src = _wrap(body, iters, drop_puts=drop_puts)
        print(
            f"\n=== {name}  N={iters}  body={len(body)}b  stressed={len(stress_src)}b",
            file=sys.stderr,
        )

        # Write stressed source to disk so tclsh can run it.
        stress_path = OUTPUT / f"stress__{name}"
        stress_path.write_text(stress_src)

        try:
            wasm = _compile_wasm(stress_src, f"stress_{name}")
        except BaseException as exc:
            print(f"  compile FAIL: {exc}", file=sys.stderr)
            out.append({"name": name, "iters": iters, "compile_error": str(exc)})
            continue

        wasm_samples, wasm_stdout, wasm_err = time_wasm(wasm, WASM_ITERS, WASM_WARM)
        tcl_samples, tcl_stdout, tcl_err, tcl_rc = time_tclsh(stress_path, TCLSH_ITERS, TCLSH_WARM)

        wasm_med = median(wasm_samples)
        tcl_med = median(tcl_samples)
        result = {
            "name": name,
            "iters": iters,
            "wasm_bytes": len(wasm),
            "wasm_median_ns": wasm_med,
            "tclsh_median_ns": tcl_med,
            "wasm_error": wasm_err,
            "tclsh_error": tcl_err,
            "tclsh_returncode": tcl_rc,
            "stdout_match": (wasm_stdout or "").strip() == (tcl_stdout or "").strip(),
        }
        out.append(result)
        if wasm_med and tcl_med:
            print(
                f"  wasm={wasm_med / 1e6:7.2f} ms  tclsh={tcl_med / 1e6:7.2f} ms"
                f"  ratio(wasm/tcl)={wasm_med / tcl_med:5.2f}x",
                file=sys.stderr,
            )
        else:
            print(
                f"  wasm={wasm_med}  tcl={tcl_med}  err={wasm_err}/{tcl_err}",
                file=sys.stderr,
            )

    (OUTPUT / "stress_results.json").write_text(json.dumps(out, indent=2, sort_keys=True))
    print("\nWrote stress_results.json", file=sys.stderr)


if __name__ == "__main__":
    main()
