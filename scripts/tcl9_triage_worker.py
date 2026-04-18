#!/usr/bin/env python3
"""Run one Tcl 9 test file bundle and print a single-line JSON result
to stdout. Invoked by ``scripts/tcl9_triage_run.py`` as a subprocess so
a runtime trap / OOM does not crash the batch.

Usage:
    python scripts/tcl9_triage_worker.py <stem> <subsystem>
"""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, "/home/user/tcl-lsp")

from tests.external.run_tcl9_tests import (  # noqa: E402
    _bundle,
    _first_failing,
    _infer_category,
    _parse_summary,
    _stderr_tail,
)
from tests.test_wasm_real_tcl import (  # noqa: E402
    _compile_tcl_with_diag,
    _resolve_trap,
    _run_wasm,
)


def main() -> int:
    stem = sys.argv[1]
    subsystem = sys.argv[2]
    filename = f"{stem}.test"
    test_path = Path(f"/home/user/tcl-lsp/tmp/tcl9.0.3/tests/{filename}")
    bundle = _bundle(test_path)

    compiled = False
    ran = False
    stdout = ""
    stderr = ""
    trap_site: str | None = None
    summary = None
    diag = None
    wasm = None

    try:
        wasm, diag = _compile_tcl_with_diag(bundle, filename)
        compiled = True
    except Exception as exc:  # noqa: BLE001
        result = {
            "file": filename,
            "subsystem": subsystem,
            "stage": "compile",
            "status": "fail",
            "trap_site": None,
            "stderr_tail": str(exc)[-400:],
            "category": "B",
        }
        print(json.dumps(result))
        return 0

    try:
        with tempfile.TemporaryDirectory(prefix="tcl9test-") as host_tmp:
            result_tuple = _run_wasm(
                wasm,
                capture_stdout=True,
                capture_stderr=True,
                preopen_tmpdir=host_tmp,
            )
        ran = True
        stdout = result_tuple[1] if len(result_tuple) >= 2 else ""
        stderr = result_tuple[2] if len(result_tuple) >= 3 else ""
        summary = _parse_summary(stdout)
    except Exception as trap:  # noqa: BLE001
        stderr = getattr(trap, "tcl_stderr", "") or str(trap)
        trap_site = _resolve_trap(trap, stderr, diag)

    category = _infer_category(
        compiled=compiled,
        ran=ran,
        summary=summary,
        trap_site=trap_site,
        stderr=stderr,
        deferred=False,
    )
    total, passed, skipped, failed = summary or (0, 0, 0, 0)
    result = {
        "file": filename,
        "subsystem": subsystem,
        "stage": "run",
        "status": "pass" if category == "pass" else "fail",
        "total": total,
        "passed": passed,
        "skipped": skipped,
        "failed": failed,
        "first_failing_test": _first_failing(stdout),
        "trap_site": trap_site,
        "stderr_tail": _stderr_tail(stderr),
        "category": category,
    }
    print(json.dumps(result))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
