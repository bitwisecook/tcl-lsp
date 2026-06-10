"""Shared classifier for Tcl 9 tcltest sweep results.

Used by ``refresh_tcl9_baseline.py`` and ``diff_tcl9_tcltest.py``.
"""

from __future__ import annotations


def classify(w: dict) -> str:
    if w.get("compile_error"):
        return "compile-fail"
    if w.get("run_error"):
        return "run-trap"
    if "total" not in w:
        return "no-summary"
    failed = w.get("failed", 0)
    passed = w.get("passed", 0)
    total = w.get("total", 0)
    if total == 0:
        return "no-summary"
    if failed == 0 and passed > 0 and passed == total - w.get("skipped", 0):
        return "pass"
    if passed > 0:
        return "partial"
    return "no-pass"
