"""Regression gate for the Tcl 9 core test slice through the **Zig WASM runtime**.

**Scope.**  This gate exercises the WASM ship target — the Zig runtime
(``runtime/zig/``) plus the WASM codegen (``core/compiler/codegen/wasm/``).
Unlike the Python-VM-side sibling
(``tests/test_vm_tcl9_core_baseline.py``), this *is* a production
correctness gate: regressions here represent real WASM-runtime gaps that
block correctness against upstream Tcl 9.

Runs the focused harness (``scripts/run_tcl9_wasm_core.py``) and asserts
that no stem regresses against the committed baseline at
``tests/baselines/tcl9-tcltest-wasm/summary.json``.

The full sweep takes 3-5 minutes wall-clock with 4 workers, so this
test is marked ``slow`` and gated behind the ``RUN_WASM_TCL9_CORE``
environment variable.  CI does not run it on every PR;
``make test-tcl9-wasm-core`` runs it explicitly.

Hand-off rules for fixing failures live in
``tests/baselines/tcl9-tcltest-wasm/README.md`` — read that before
editing the runtime in response to a regression here.  Crucially:
never edit ``tcltest.tcl`` / ``init.tcl`` / the upstream ``.test``
files, never add a new monkey-patch, and fix the root cause in
``runtime/zig/`` or ``core/compiler/codegen/wasm/``.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
HARNESS = REPO_ROOT / "scripts" / "run_tcl9_wasm_core.py"
BASELINE = REPO_ROOT / "tests" / "baselines" / "tcl9-tcltest-wasm" / "summary.json"
REPORT = REPO_ROOT / "tmp" / "tcl9-wasm-core-report.json"

GATE_ENV_VAR = "RUN_WASM_TCL9_CORE"


pytestmark = [
    pytest.mark.slow,
    pytest.mark.skipif(
        not os.environ.get(GATE_ENV_VAR),
        reason=(
            f"Set {GATE_ENV_VAR}=1 to run the full Tcl 9 core slice "
            "through the WASM runtime (takes 3-5 minutes wall-clock)."
        ),
    ),
]


# Worker count and per-stem wall-clock timeout the gate passes to the
# harness.  Kept as module-level constants so the subprocess invocation
# and ``_harness_upper_bound_seconds`` cannot drift apart — the harness
# script's own defaults are tuned for interactive use (workers=2) and
# differ from what the gate wants.
_GATE_WORKERS = 4
_GATE_PER_STEM_TIMEOUT_S = 240


def _harness_upper_bound_seconds(
    num_stems: int = 68,
    workers: int = _GATE_WORKERS,
    per_stem: int = _GATE_PER_STEM_TIMEOUT_S,
) -> int:
    """Conservative ceiling for how long the harness can run.

    Worst case = every stem hits its per-stem wall-clock timeout, no
    parallelism win.  We add a generous startup overhead bucket on
    top so a transient slow CI runner doesn't flake the gate.
    """
    serialized_worst_case = num_stems * per_stem
    parallel_worst_case = -(-serialized_worst_case // max(1, workers))  # ceil-div
    boot_overhead = 60  # zig build (if cold) + wasmtime engine creation, etc.
    return parallel_worst_case + boot_overhead


def _run_harness() -> dict:
    """Invoke the harness, return the JSON report it wrote."""
    REPORT.parent.mkdir(parents=True, exist_ok=True)
    if REPORT.exists():
        REPORT.unlink()
    timeout = _harness_upper_bound_seconds()
    try:
        result = subprocess.run(
            [
                sys.executable,
                str(HARNESS),
                "--no-baseline",
                "--workers",
                str(_GATE_WORKERS),
                "--timeout",
                str(_GATE_PER_STEM_TIMEOUT_S),
            ],
            cwd=str(REPO_ROOT),
            capture_output=True,
            text=True,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired as exc:
        stdout = (
            (exc.stdout or b"").decode("utf-8", errors="replace")
            if isinstance(exc.stdout, bytes)
            else (exc.stdout or "")
        )
        stderr = (
            (exc.stderr or b"").decode("utf-8", errors="replace")
            if isinstance(exc.stderr, bytes)
            else (exc.stderr or "")
        )
        pytest.fail(
            f"Harness exceeded the gate's wall-clock ceiling ({timeout}s).\n"
            "This is much longer than a healthy run (~3.5 min).  Likely "
            "a new test stem is hanging — run "
            "`make refresh-tcl9-wasm-core-baseline` interactively to see "
            "which stem is timing out.\n"
            f"stdout (tail):\n{stdout[-2000:]}\n"
            f"stderr (tail):\n{stderr[-2000:]}"
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
    """Every stem must match or improve on the committed WASM baseline.

    Regressions are anything that makes the floor worse:

    * a stem went from clean (no crash) to crashed,
    * ``passed`` dropped below the recorded ``passed_min``,
    * ``failed`` rose above the recorded ``failed_max``,
    * a stem in the baseline disappeared from the report,
    * a stem appeared in the report without a baseline entry (new
      stem added without refreshing the baseline).
    """
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
        if row["crashed"] and not base.get("crashed"):
            regressions.append(
                f"{stem}: was clean (no crash) in baseline, now crashed: "
                f"{row['crash_type']} — {row['crash_msg']}"
            )
            continue
        passed_min = int(base.get("passed_min", 0))
        if row["passed"] < passed_min:
            regressions.append(f"{stem}: passed {row['passed']} < baseline floor {passed_min}")
        failed_max = int(base.get("failed_max", 0))
        if row["failed"] > failed_max:
            regressions.append(f"{stem}: failed {row['failed']} > baseline ceiling {failed_max}")

    for stem in rows:
        if stem not in base_stems:
            new_unknown.append(stem)

    msgs: list[str] = []
    if regressions:
        msgs.append("Regressions:\n  " + "\n  ".join(regressions))
    if new_unknown:
        msgs.append(
            "New stems with no baseline (run `--refresh-baseline`):\n  " + "\n  ".join(new_unknown)
        )

    if msgs:
        pytest.fail("\n\n".join(msgs))
