"""Run dict test files natively through the VM's tcltest.

Covers dict.test from the Tcl 9.0.3 test suite (Phase 5c of the VM
test conformance plan).

Known failures are tracked per-file so that regressions are caught
immediately while expected failures don't block CI.
"""

from __future__ import annotations

import io

import pytest

from tests.conftest import ensure_tcl_source
from vm.commands import tcltest_cmds
from vm.commands.test_support_cmds import setup_test_support
from vm.interp import TclInterp

pytestmark = pytest.mark.slow

# Known failures

KNOWN_FAILURES_DICT: set[str] = {
    # dict for: scope/variable issues
    "dict-2.3",
    "dict-2.6",
    "dict-2.7",
    "dict-2.8",
    "dict-2.11",
    "dict-2.14",
    # dict filter: missing filter modes or wrong result
    "dict-3.12",
    # dict map: scope/lappend/errorInfo issues
    "dict-4.5",
    "dict-4.6",
    "dict-4.13",
    "dict-4.14",
    "dict-4.14a",
    "dict-4.15",
    "dict-4.15a",
    "dict-4.16",
    "dict-4.16a",
    "dict-4.17",
    # dict with: scope issues
    "dict-5.7",
    "dict-5.12",
    # dict update: scope issues
    "dict-6.8",
    "dict-6.9",
    # dict lappend/append list quoting
    "dict-7.8",
    "dict-7.9",
    "dict-8.4",
    "dict-8.5",
    # dict info/smart-reference
    "dict-9.7",
    "dict-9.8",
    # dict replace/remove edge cases
    "dict-10.2",
    "dict-10.3",
}


# Test runner


def _run_test_file(test_file: str) -> dict[str, object]:
    """Source a .test file through the VM and return results."""
    interp = TclInterp(source_init=False)
    setup_test_support(interp)
    tcltest_cmds._reset_state()
    buf = io.StringIO()
    interp.channels["stdout"] = buf
    tests_dir = ensure_tcl_source("9.0")
    path = tests_dir / test_file
    script = path.read_text()
    try:
        interp.eval(script)
    except Exception as exc:
        buf.write(f"\n*** Top-level error: {exc}\n")
    return {
        "Total": tcltest_cmds._results["Total"],
        "Passed": tcltest_cmds._results["Passed"],
        "Skipped": tcltest_cmds._results["Skipped"],
        "Failed": tcltest_cmds._results["Failed"],
        "failed_tests": list(tcltest_cmds._failed_tests),
        "output": buf.getvalue(),
    }


def _check_results(
    results: dict[str, object],
    known_failures: set[str],
    test_file: str,
) -> None:
    """Assert that failures are exactly the known set."""
    failed_set = set(results["failed_tests"])  # type: ignore[arg-type]
    total = results["Total"]
    passed = results["Passed"]
    skipped = results["Skipped"]
    print(
        f"\n{test_file}: {total} total, {passed} passed, "
        f"{skipped} skipped, {len(failed_set)} failed"
    )
    unexpected_failures = failed_set - known_failures
    unexpected_passes = known_failures - failed_set
    if unexpected_failures:
        output = results["output"]
        if isinstance(output, str) and output:
            for name in sorted(unexpected_failures):
                marker = f"==== {name} FAILED"
                start = output.find(marker)
                if start >= 0:
                    end = output.find(marker, start + len(marker))
                    if end >= 0:
                        end = end + len(marker)
                    else:
                        end = min(start + 500, len(output))
                    print(output[start:end])
    msgs: list[str] = []
    if unexpected_failures:
        msgs.append(f"Unexpected failures: {', '.join(sorted(unexpected_failures))}")
    if unexpected_passes:
        msgs.append(
            f"Unexpected passes (remove from known failures): "
            f"{', '.join(sorted(unexpected_passes))}"
        )
    if msgs:
        pytest.fail("\n".join(msgs))


# Test classes


class TestDictNative:
    """Run tmp/tcl9.0.3/tests/dict.test through the VM."""

    def test_dict(self) -> None:
        results = _run_test_file("dict.test")
        _check_results(results, KNOWN_FAILURES_DICT, "dict.test")
