"""Regression gate for the Tcl 9 core test slice through the VM.

Runs the focused harness (``scripts/run_tcl9_vm_core.py``) and asserts
that no stem regresses against the committed baseline at
``tests/baselines/tcl9-tcltest-vm/summary.json``.

The full sweep takes 3–5 minutes wall-clock, so this test is marked
``slow`` and is gated behind the ``RUN_VM_TCL9_CORE`` environment
variable.  CI does not run it on every PR; ``make test-tcl9-vm-core``
runs it explicitly.

Hand-off rules for fixing failures live in
``tests/baselines/tcl9-tcltest-vm/README.md`` — read that before
editing the VM in response to a regression here.  Crucially: never
edit ``tcltest.tcl`` / ``init.tcl`` / the upstream ``.test`` files,
and never add a new monkey-patch in lieu of a real fix.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
HARNESS = REPO_ROOT / "scripts" / "run_tcl9_vm_core.py"
BASELINE = REPO_ROOT / "tests" / "baselines" / "tcl9-tcltest-vm" / "summary.json"
REPORT = REPO_ROOT / "tmp" / "tcl9-vm-core-report.json"

GATE_ENV_VAR = "RUN_VM_TCL9_CORE"


pytestmark = pytest.mark.skipif(
    not os.environ.get(GATE_ENV_VAR),
    reason=(
        f"Set {GATE_ENV_VAR}=1 to run the full Tcl 9 core slice "
        "(takes 3-5 minutes wall-clock)."
    ),
)


def _run_harness() -> dict:
    """Invoke the harness, return the JSON report it wrote."""
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    if REPORT.exists():
        REPORT.unlink()
    result = subprocess.run(
        [sys.executable, str(HARNESS), "--no-baseline"],
        cwd=str(REPO_ROOT),
        capture_output=True,
        text=True,
        timeout=15 * 60,
        check=False,
    )
    if result.returncode != 0:
        pytest.fail(
            f"Harness exited {result.returncode}\n"
            f"stdout:\n{result.stdout[-2000:]}\n"
            f"stderr:\n{result.stderr[-2000:]}"
        )
    if not REPORT.exists():
        pytest.fail(
            f"Harness did not write {REPORT}\n"
            f"stdout:\n{result.stdout[-2000:]}\n"
            f"stderr:\n{result.stderr[-2000:]}"
        )
    return json.loads(REPORT.read_text())


def _load_baseline() -> dict:
    if not BASELINE.exists():
        pytest.fail(
            f"Baseline {BASELINE} not found — run "
            f"`python {HARNESS.relative_to(REPO_ROOT)} --refresh-baseline` "
            "to capture it."
        )
    return json.loads(BASELINE.read_text())


def test_no_stem_regresses_against_baseline() -> None:
    """Every stem must match or improve on the committed baseline."""
    report = _run_harness()
    baseline = _load_baseline()
    base_stems: dict[str, dict] = baseline["stems"]

    rows = {row["stem"]: row for row in report["rows"]}
    regressions: list[str] = []
    new_unknown: list[str] = []

    for stem, base in base_stems.items():
        row = rows.get(stem)
        if row is None:
            regressions.append(f"{stem}: missing from report")
            continue
        # Crash regressions
        if row["crashed"] and not base.get("crashed"):
            regressions.append(
                f"{stem}: was clean (no crash) in baseline, now crashed: "
                f"{row['crash_type']} — {row['crash_msg']}"
            )
            continue
        # Pass-count regressions
        passed_min = int(base.get("passed_min", 0))
        if row["passed"] < passed_min:
            regressions.append(
                f"{stem}: passed {row['passed']} < baseline floor {passed_min}"
            )
        failed_max = int(base.get("failed_max", 0))
        if row["failed"] > failed_max:
            regressions.append(
                f"{stem}: failed {row['failed']} > baseline ceiling {failed_max}"
            )

    for stem in rows:
        if stem not in base_stems:
            new_unknown.append(stem)

    msgs: list[str] = []
    if regressions:
        msgs.append("Regressions:\n  " + "\n  ".join(regressions))
    if new_unknown:
        msgs.append(
            "New stems with no baseline (run `--refresh-baseline`):\n  "
            + "\n  ".join(new_unknown)
        )

    if msgs:
        pytest.fail("\n\n".join(msgs))
