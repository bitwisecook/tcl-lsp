"""Run error.test natively through the VM's tcltest.

Covers error.test from the Tcl 9.0.3 test suite.
Reference (tclsh 9.0): 309P/8S/0F.

Known failures are tracked so that regressions are caught immediately
while expected failures don't block CI.
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
#
# Each set lists Tcl test names that are expected to fail in our VM.
# When a VM bug is fixed the test will unexpectedly pass — the set
# must be updated (removing the entry) to keep CI green.

KNOWN_FAILURES_ERROR: set[str] = {
    # return -code / errorCode handling
    "error-1.6",
    "error-1.7",
    # errorInfo / stack trace formatting
    "error-2.3",
    "error-2.6",
    "error-3.1",
    # catch / return interaction
    "error-4.2",
    "error-4.3",
    "error-4.5",
    "error-4.6",
    "error-4.7",
    "error-4.8",
    # nested error propagation
    "error-5.1",
    "error-5.2",
    # errorInfo truncation / formatting
    "error-6.10",
    # error in proc bodies
    "error-7.1",
    # try / throw / on handling
    "error-8.8",
    "error-8.9",
    "error-8.10",
    "error-8.11",
    # return -level
    "error-9.6",
    # return -code with numeric codes
    "error-10.7",
    "error-10.10",
    "error-10.11",
    "error-10.12",
    # errorstack / options dict
    "error-13.3",
    "error-13.4",
    "error-13.5",
    "error-13.6",
    "error-13.7",
    "error-13.8",
    "error-13.9",
    "error-13.10",
    # lassign / multi-return
    "error-14.8",
    "error-14.9",
    # error message quoting
    "error-15.4",
    "error-15.5",
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


# Test class


class TestErrorNative:
    """Run tmp/tcl9.0.3/tests/error.test through the VM."""

    def test_error(self) -> None:
        results = _run_test_file("error.test")
        _check_results(results, KNOWN_FAILURES_ERROR, "error.test")
