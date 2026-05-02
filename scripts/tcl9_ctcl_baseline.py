"""C Tcl 9.0.3 baseline sweep — run each .test file under native tclsh.

Captures wall time, exit code, peak RSS (via GNU ``time -v``), and the
tcltest summary line for each file in ``_IN_SCOPE``.  Mirrors the
WASM sweep so the two CSVs can be diffed side-by-side.

Each test file is invoked as

    tclsh tests/<name>.test -verbose ""

with ``cwd`` set to ``tmp/tcl9.0.3/tests/`` so all relative-path
``source`` lookups inside test files (``tcltests.tcl``,
``encodingVectors.tcl``, etc.) resolve.

Output: NDJSON to --out, one record per bundle.

Build the tclsh first::

    mkdir -p /tmp/tcl-build && cd /tmp/tcl-build && \
        ./tmp/tcl9.0.3/unix/configure --prefix=/tmp/tcl-install && \
        make -j$(nproc) tclsh

Then run::

    uv run --extra dev python scripts/tcl9_ctcl_baseline.py \
        --out /tmp/c-tcl-sweep.ndjson
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

_REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(_REPO))
from tests.external.run_tcl9_tests import _IN_SCOPE  # noqa: E402

_TCLSH = os.environ.get("CTCL_TCLSH", "/tmp/tcl-build/tclsh")
_LIB = os.environ.get("CTCL_LIB", "/tmp/tcl-build")
_TESTS_DIR = _REPO / "tmp" / "tcl9.0.3" / "tests"

_SUMMARY_RE = re.compile(r"Total\s+(\d+)\s+Passed\s+(\d*)\s+Skipped\s+(\d*)\s+Failed\s+(\d*)")
_TIME_RSS_RE = re.compile(r"Maximum resident set size \(kbytes\):\s*(\d+)")
_TIME_USER_RE = re.compile(r"User time \(seconds\):\s*([\d.]+)")
_TIME_SYS_RE = re.compile(r"System time \(seconds\):\s*([\d.]+)")
_TIME_WALL_RE = re.compile(r"Elapsed \(wall clock\) time .*?:\s*([\d:.]+)")


def _wall_to_sec(s: str) -> float:
    parts = s.split(":")
    if len(parts) == 2:
        m, sec = parts
        return float(m) * 60 + float(sec)
    if len(parts) == 3:
        h, m, sec = parts
        return float(h) * 3600 + float(m) * 60 + float(sec)
    return float(s)


def run_one(stem: str, *, timeout_s: int) -> dict:
    test_path = _TESTS_DIR / f"{stem}.test"
    rec: dict = {"name": stem}
    if not test_path.exists():
        rec["outcome"] = "skip"
        rec["trap"] = f"missing {test_path}"
        return rec

    env = dict(os.environ)
    env["LD_LIBRARY_PATH"] = _LIB
    env["TCLLIBPATH"] = ""
    # Quiet modules path so msgcat etc. don't hit the network.
    env.pop("TCL_LIBRARY", None)

    cmd = [
        "/usr/bin/time",
        "-v",
        _TCLSH,
        f"{stem}.test",
        "-verbose",
        "",
    ]
    t0 = time.monotonic()
    try:
        cp = subprocess.run(
            cmd,
            cwd=str(_TESTS_DIR),
            env=env,
            capture_output=True,
            text=True,
            timeout=timeout_s,
        )
        wall = time.monotonic() - t0
        stdout, stderr = cp.stdout, cp.stderr
        rc = cp.returncode
        rec["timed_out"] = False
    except subprocess.TimeoutExpired as exc:
        wall = time.monotonic() - t0
        stdout = (
            (exc.stdout or b"").decode(errors="replace")
            if isinstance(exc.stdout, bytes)
            else (exc.stdout or "")
        )
        stderr = (
            (exc.stderr or b"").decode(errors="replace")
            if isinstance(exc.stderr, bytes)
            else (exc.stderr or "")
        )
        rc = -1
        rec["timed_out"] = True

    rec["wall_ms"] = wall * 1000.0
    rec["rc"] = rc

    # Parse GNU time stats (printed to stderr).
    rss_kb = None
    user_s = None
    sys_s = None
    time_wall = None
    if m := _TIME_RSS_RE.search(stderr):
        rss_kb = int(m.group(1))
    if m := _TIME_USER_RE.search(stderr):
        user_s = float(m.group(1))
    if m := _TIME_SYS_RE.search(stderr):
        sys_s = float(m.group(1))
    if m := _TIME_WALL_RE.search(stderr):
        time_wall = _wall_to_sec(m.group(1))
    rec["max_rss_kb"] = rss_kb
    rec["user_s"] = user_s
    rec["sys_s"] = sys_s
    rec["time_wall_s"] = time_wall

    # Parse tcltest summary if present.
    m = _SUMMARY_RE.search(stdout)
    if m:
        rec["total"] = int(m.group(1))
        rec["passed"] = int(m.group(2) or 0)
        rec["skipped"] = int(m.group(3) or 0)
        rec["failed"] = int(m.group(4) or 0)
        if rec["failed"] == 0:
            rec["outcome"] = "pass"
        else:
            rec["outcome"] = "fail"
    else:
        if rec.get("timed_out"):
            rec["outcome"] = "timeout"
        elif rc == 0:
            rec["outcome"] = "no_summary"
        else:
            rec["outcome"] = "trap"
        rec["total"] = rec["passed"] = rec["skipped"] = rec["failed"] = 0

    rec["stderr_tail"] = stderr[-300:]
    rec["stdout_tail"] = stdout[-300:]
    return rec


def main() -> int:
    import argparse

    p = argparse.ArgumentParser()
    p.add_argument("--names", default=None)
    p.add_argument("--timeout", type=int, default=120)
    p.add_argument("--out", default="/tmp/c-tcl-sweep.ndjson")
    args = p.parse_args()

    if args.names:
        names = [s.strip() for s in Path(args.names).read_text().splitlines() if s.strip()]
    else:
        names = [stem for stem, _ in _IN_SCOPE]

    out_path = Path(args.out)
    sweep_start = time.monotonic()
    with out_path.open("w") as out_f:
        total = len(names)
        for i, stem in enumerate(names, 1):
            t0 = time.monotonic()
            try:
                rec = run_one(stem, timeout_s=args.timeout)
            except Exception as exc:
                rec = {"name": stem, "outcome": "harness_error", "trap": str(exc)[-300:]}
            rec["wall_ms_outer"] = (time.monotonic() - t0) * 1000.0
            out_f.write(json.dumps(rec) + "\n")
            out_f.flush()
            elapsed = time.monotonic() - sweep_start
            rss = rec.get("max_rss_kb")
            rss_s = f"rss={rss / 1024:5.1f}MB" if rss else "rss=?    "
            tot, pas, fail = rec.get("total", 0), rec.get("passed", 0), rec.get("failed", 0)
            print(
                f"[{i:3d}/{total}] {stem:18s} "
                f"{rec.get('outcome', '?'):10s} "
                f"wall={rec.get('wall_ms', 0):7.0f}ms  "
                f"{rss_s}  "
                f"T{tot:4d} P{pas:4d} F{fail:3d}  "
                f"(elapsed={elapsed:.0f}s)",
                file=sys.stderr,
                flush=True,
            )

    print(f"sweep done: {out_path}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
