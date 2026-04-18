#!/usr/bin/env python3
"""Run every registered Tcl 9 test file through the WASM pipeline and
emit a compact JSON triage report.

Each test file runs in a fresh subprocess so a runaway compile or a
hard runtime trap can't wedge the whole batch. The parent process
collects per-file JSON results and aggregates them into
``tmp/tcl9-report.json``.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, "/home/user/tcl-lsp")

from tests.external.run_tcl9_tests import _IN_SCOPE  # noqa: E402

_WORKER = Path("/home/user/tcl-lsp/scripts/tcl9_triage_worker.py")


def _run_one(stem: str, subsystem: str, *, timeout: int) -> dict:
    filename = f"{stem}.test"
    test_path = Path(f"/home/user/tcl-lsp/tmp/tcl9.0.3/tests/{filename}")
    if not test_path.exists():
        return {
            "file": filename,
            "subsystem": subsystem,
            "stage": "missing",
            "status": "skip",
            "category": "E",
            "notes": "test file missing",
        }
    try:
        proc = subprocess.run(
            [sys.executable, str(_WORKER), stem, subsystem],
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        return {
            "file": filename,
            "subsystem": subsystem,
            "stage": "timeout",
            "status": "fail",
            "trap_site": f"worker timeout ({timeout}s)",
            "stderr_tail": "",
            "category": "E",
        }
    except Exception as exc:  # noqa: BLE001
        return {
            "file": filename,
            "subsystem": subsystem,
            "stage": "spawn",
            "status": "fail",
            "trap_site": f"spawn error: {exc}",
            "stderr_tail": "",
            "category": "E",
        }

    # Worker prints a single JSON line on stdout; stderr is preserved
    # as-is for post-mortem in case of parse failure.
    out = proc.stdout.strip().splitlines()
    if not out:
        return {
            "file": filename,
            "subsystem": subsystem,
            "stage": "worker",
            "status": "fail",
            "trap_site": "worker produced no output",
            "stderr_tail": proc.stderr[-400:],
            "category": "E",
        }
    try:
        return json.loads(out[-1])
    except json.JSONDecodeError:
        return {
            "file": filename,
            "subsystem": subsystem,
            "stage": "worker",
            "status": "fail",
            "trap_site": "worker JSON parse failed",
            "stderr_tail": (proc.stderr or out[-1])[-400:],
            "category": "E",
        }


def main() -> int:
    out_path = Path("tmp/tcl9-report.json")
    timeout = int((sys.argv[1:] or ["180"])[0])

    # Sort by source-file size so the quick wins land in the report
    # first — a crash or timeout late in the run still leaves useful
    # triage data from the small files.
    tests_dir = Path("/home/user/tcl-lsp/tmp/tcl9.0.3/tests")

    def _size(entry: tuple[str, str]) -> int:
        p = tests_dir / f"{entry[0]}.test"
        return p.stat().st_size if p.exists() else 0

    ordered = sorted(_IN_SCOPE, key=_size)

    results: list[dict] = []
    for i, (stem, subsystem) in enumerate(ordered, start=1):
        status = _run_one(stem, subsystem, timeout=timeout)
        cat = status.get("category")
        counts = (
            f"{status.get('passed', 0)}/{status.get('total', 0)}"
            if status.get("total")
            else status.get("stage", "")
        )
        print(
            f"[{i:3}/{len(ordered)}] {stem:16} [{str(cat):4}] {counts}  "
            f"first_fail={status.get('first_failing_test') or ''}",
            flush=True,
        )
        results.append(status)
        # Checkpoint after every file so a crash doesn't lose progress.
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(json.dumps(results, indent=2, sort_keys=True) + "\n")

    print(f"\nWrote {len(results)} entries to {out_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
