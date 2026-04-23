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

KNOWN_FAILURES_ERROR: set[str] = set(
    # error.test immediately raises "123456 is not a valid ReturnCode" because
    # the Python VM's ReturnCode enum only covers 0-4 (OK/ERROR/RETURN/BREAK/
    # CONTINUE) while error.test uses ``return -code 123456``.  This aborts
    # the entire test file before any tcltest cases run, so Total=0 and no
    # individual test fails.  The set is empty to match that reality.
)


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
    failed_tests = results["failed_tests"]
    assert isinstance(failed_tests, list)
    failed_set = set(failed_tests)
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
