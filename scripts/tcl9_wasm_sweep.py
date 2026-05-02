"""In-process Tcl 9 tcltest sweep with per-bundle timing.

Mirrors what scripts/tcltest_sweep/run_one.sh does, but skips the
``uv run pytest`` startup cost.  Uses subprocess-per-bundle so the
OS can hard-kill bundles stuck in tight WASM/host loops where
SIGALRM doesn't get a chance to fire (e.g. ``expr.test``,
``obj.test``).

When called with --worker, runs a single named bundle and prints
its JSON record to stdout.  Without --worker, drives the full
sweep by re-execing itself once per name.

Output: NDJSON to --out, one record per bundle with name, outcome,
summary fields, trap, and durations.
"""

from __future__ import annotations

import json
import shutil
import signal
import sys
import tempfile
import time
import traceback
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(_REPO_ROOT))

from tests.external.run_tcl9_tests import (  # noqa: E402
    _IN_SCOPE,
    _bundle,
    _first_failing,
    _parse_summary,
    _tcl9_tcltest,
    _tcl9_test_file,
)
from tests.test_wasm_real_tcl import (  # noqa: E402
    _compile_tcl_with_diag,
    _resolve_trap,
    _run_wasm,
)

_TIMEOUT_MARK = "__SWEEP_TIMEOUT__"


class _BundleTimeout(Exception):
    pass


def _alarm_handler(signum, frame):  # noqa: ARG001
    raise _BundleTimeout()


def run_one(stem: str, *, timeout_s: int) -> dict:
    """Compile + run one tcltest bundle, returning a sweep record."""
    rec: dict = {
        "name": stem,
        "outcome": "unknown",
        "compile_ms": None,
        "run_ms": None,
        "total": 0,
        "passed": 0,
        "skipped": 0,
        "failed": 0,
        "trap": "",
        "first_failing": "",
        "stderr_tail": "",
    }

    try:
        test_path = _tcl9_test_file(f"{stem}.test")
    except Exception as exc:
        rec["outcome"] = "skip"
        rec["trap"] = f"missing test file: {exc}"
        return rec

    src = _bundle(test_path)
    diag = None

    # Compile (no timeout — cheap, worst-case ~1s)
    t_compile = time.monotonic()
    try:
        wasm, diag = _compile_tcl_with_diag(src, f"{stem}.test")
    except Exception as exc:
        rec["outcome"] = "compile_fail"
        rec["compile_ms"] = (time.monotonic() - t_compile) * 1000
        rec["trap"] = str(exc)[:200]
        return rec
    rec["compile_ms"] = (time.monotonic() - t_compile) * 1000

    # Run with SIGALRM-based timeout
    signal.signal(signal.SIGALRM, _alarm_handler)
    signal.alarm(timeout_s)
    t_run = time.monotonic()
    try:
        with tempfile.TemporaryDirectory(prefix="tcl9test-") as host_tmp:
            helpers_dir = _tcl9_tcltest().parent.parent.parent / "tests"
            for helper in (
                "tcltests.tcl",
                "internals.tcl",
                "encodingVectors.tcl",
                "pkgIndex.tcl",
            ):
                src_path = helpers_dir / helper
                if src_path.exists():
                    shutil.copyfile(src_path, Path(host_tmp) / helper)
            try:
                result = _run_wasm(
                    wasm,
                    capture_stdout=True,
                    capture_stderr=True,
                    preopen_tmpdir=host_tmp,
                )
                stdout = result[1] if len(result) >= 2 else ""
                stderr = result[2] if len(result) >= 3 else ""
                rec["run_ms"] = (time.monotonic() - t_run) * 1000
                summary = _parse_summary(stdout)
                if summary is None:
                    rec["outcome"] = "no_summary"
                    rec["stderr_tail"] = stderr[-300:]
                    rec["stdout_tail"] = stdout[-300:]
                else:
                    total, passed, skipped, failed = summary
                    rec.update(
                        {
                            "total": total,
                            "passed": passed,
                            "skipped": skipped,
                            "failed": failed,
                        }
                    )
                    rec["first_failing"] = _first_failing(stdout) or ""
                    rec["outcome"] = "pass" if failed == 0 else "fail"
                    if failed > 0:
                        rec["stdout_tail"] = stdout[-400:]
            except _BundleTimeout:
                rec["outcome"] = "timeout"
                rec["run_ms"] = timeout_s * 1000.0
            except Exception as exc:
                rec["outcome"] = "trap"
                rec["run_ms"] = (time.monotonic() - t_run) * 1000
                stderr_text = getattr(exc, "tcl_stderr", "")
                trap_msg = _resolve_trap(exc, stderr_text, diag)
                rec["trap"] = trap_msg[:200]
                rec["stderr_tail"] = stderr_text[-300:]
    finally:
        signal.alarm(0)

    return rec


def main() -> int:
    import argparse
    import subprocess

    p = argparse.ArgumentParser()
    p.add_argument("--names", default=None)
    p.add_argument("--timeout", type=int, default=60)
    p.add_argument("--out", default="/tmp/tcltest-sweep-fast.ndjson")
    p.add_argument(
        "--worker",
        default=None,
        help="Internal: run a single named bundle and print its record to stdout.",
    )
    p.add_argument(
        "--inproc",
        action="store_true",
        help="Run all bundles in-process (faster, but one stuck test halts the sweep).",
    )
    args = p.parse_args()

    # Worker mode: run one bundle in-process and print its JSON record.
    if args.worker is not None:
        rec = run_one(args.worker, timeout_s=args.timeout)
        sys.stdout.write(json.dumps(rec))
        sys.stdout.flush()
        return 0

    # Determine names
    if args.names:
        names = [s.strip() for s in Path(args.names).read_text().splitlines() if s.strip()]
    else:
        names = [stem for stem, _ in _IN_SCOPE]

    out_path = Path(args.out)
    with out_path.open("w") as out_f:
        total = len(names)
        sweep_start = time.monotonic()
        for i, stem in enumerate(names, 1):
            t0 = time.monotonic()
            try:
                if args.inproc:
                    rec = run_one(stem, timeout_s=args.timeout)
                else:
                    # Subprocess-per-bundle: OS can SIGKILL stuck workers.
                    # Add a 10s buffer over the in-bundle timeout so SIGALRM
                    # (when it works) wins; a hard SIGKILL is the fallback.
                    cp = subprocess.run(
                        [
                            sys.executable,
                            __file__,
                            "--worker",
                            stem,
                            "--timeout",
                            str(args.timeout),
                        ],
                        capture_output=True,
                        text=True,
                        timeout=args.timeout + 15,
                    )
                    if cp.returncode == 0 and cp.stdout.strip():
                        rec = json.loads(cp.stdout.strip())
                    else:
                        rec = {
                            "name": stem,
                            "outcome": "worker_error",
                            "trap": (cp.stderr or cp.stdout)[-300:],
                        }
            except subprocess.TimeoutExpired:
                rec = {
                    "name": stem,
                    "outcome": "timeout",
                    "trap": "subprocess hard timeout (SIGKILL)",
                }
            except Exception:
                rec = {
                    "name": stem,
                    "outcome": "harness_error",
                    "trap": traceback.format_exc()[-300:],
                }
            rec["wall_ms"] = (time.monotonic() - t0) * 1000
            out_f.write(json.dumps(rec) + "\n")
            out_f.flush()
            elapsed = time.monotonic() - sweep_start
            print(
                f"[{i:3d}/{total}] {stem:20s} "
                f"{rec.get('outcome', '?'):12s} "
                f"wall={rec['wall_ms']:7.0f}ms  "
                f"(sweep_elapsed={elapsed:.0f}s)",
                file=sys.stderr,
                flush=True,
            )

    print(f"sweep done: {out_path}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
