"""Run info test files natively through the VM's tcltest.

Covers info.test from the Tcl 9.0.3 test suite (Phase 5c of the VM
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

KNOWN_FAILURES_INFO: set[str] = {
    # info cmdcount not implemented
    "info-3.1",  # testinfocmdcount compiled cmdcount
    "info-3.2",  # testinfocmdcount compiled cmdcount
    "info-3.3",  # testinfocmdcount compiled cmdcount
    # Missing wrong # args validation
    "info-2.3",  # extra args to info args
    "info-3.4",  # extra args to info cmdcount
    "info-4.5",  # extra args to info commands
    "info-7.9",  # extra args to info exists
    "info-8.3",  # extra args to info globals
    # info default behaviour
    "info-2.6",  # test setup failure (subst bar)
    # info globals scope tracking
    "info-8.4",  # variable existence not tracked correctly
    # Error message format
    "info-9.5",  # "bad level" vs "wrong # args"
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


class TestInfoNative:
    """Run tmp/tcl9.0.3/tests/info.test through the VM."""

    def test_info(self) -> None:
        results = _run_test_file("info.test")
        _check_results(results, KNOWN_FAILURES_INFO, "info.test")
