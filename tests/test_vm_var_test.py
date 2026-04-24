"""Run variable-related test files natively through the VM's tcltest.

Covers incr.test, set-old.test, upvar.test, uplevel.test, set.test,
incr-old.test, and var.test from the Tcl 9.0.3 test suite
(Phases 4b–5d of the VM test conformance plan).

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
#
# Each set lists Tcl test names that are expected to fail in our VM.
# When a VM bug is fixed the test will unexpectedly pass — the set
# must be updated (removing the entry) to keep CI green.

KNOWN_FAILURES_INCR: set[str] = set(
    # incr.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
)

KNOWN_FAILURES_SET_OLD: set[str] = set(
    # set-old.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
)

KNOWN_FAILURES_UPVAR: set[str] = set(
    # upvar.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
)

KNOWN_FAILURES_UPLEVEL: set[str] = set()

KNOWN_FAILURES_SET: set[str] = set(
    # set.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
)

KNOWN_FAILURES_INCR_OLD: set[str] = set(
    # incr-old.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
)

KNOWN_FAILURES_VAR: set[str] = set(
    # var.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
)


# Test runner


def _run_test_file(test_file: str) -> dict[str, object]:
    """Source a .test file through the VM and return results.

    Returns a dict with keys: Total, Passed, Skipped, Failed,
    failed_tests (list of test names), and output (captured stdout).
    """
    interp = TclInterp(source_init=False)
    setup_test_support(interp)

    # Reset tcltest state for a clean run
    tcltest_cmds._reset_state()

    # Capture output so test failures are visible in pytest output
    buf = io.StringIO()
    interp.channels["stdout"] = buf

    tests_dir = ensure_tcl_source("9.0")
    path = tests_dir / test_file
    script = path.read_text()

    try:
        interp.eval(script)
    except Exception as exc:
        # Some .test files may trigger errors at the top level;
        # capture them but don't abort — the tcltest counters are
        # still meaningful for the tests that did run.
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
    """Assert that failures are exactly the known set.

    - Unexpected failures (not in known set) -> test fails
    - Unexpected passes (in known set but passed) -> test fails
      (forces cleanup of the known-failure set when bugs are fixed)
    """
    failed_tests = results["failed_tests"]
    assert isinstance(failed_tests, list)
    failed_set = set(failed_tests)
    total = results["Total"]
    passed = results["Passed"]
    skipped = results["Skipped"]

    # Print summary
    print(
        f"\n{test_file}: {total} total, {passed} passed, "
        f"{skipped} skipped, {len(failed_set)} failed"
    )

    unexpected_failures = failed_set - known_failures
    unexpected_passes = known_failures - failed_set

    if unexpected_failures:
        # Show failure details for new failures
        output = results["output"]
        if isinstance(output, str) and output:
            for name in sorted(unexpected_failures):
                # Extract the relevant failure block
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


class TestIncrNative:
    """Run tmp/tcl9.0.3/tests/incr.test through the VM."""

    def test_incr(self) -> None:
        results = _run_test_file("incr.test")
        _check_results(results, KNOWN_FAILURES_INCR, "incr.test")


class TestSetOldNative:
    """Run tmp/tcl9.0.3/tests/set-old.test through the VM."""

    def test_set_old(self) -> None:
        results = _run_test_file("set-old.test")
        _check_results(results, KNOWN_FAILURES_SET_OLD, "set-old.test")


class TestUpvarNative:
    """Run tmp/tcl9.0.3/tests/upvar.test through the VM."""

    def test_upvar(self) -> None:
        results = _run_test_file("upvar.test")
        _check_results(results, KNOWN_FAILURES_UPVAR, "upvar.test")


class TestUplevelNative:
    """Run tmp/tcl9.0.3/tests/uplevel.test through the VM."""

    def test_uplevel(self) -> None:
        results = _run_test_file("uplevel.test")
        _check_results(results, KNOWN_FAILURES_UPLEVEL, "uplevel.test")


class TestSetNative:
    """Run tmp/tcl9.0.3/tests/set.test through the VM."""

    def test_set(self) -> None:
        results = _run_test_file("set.test")
        _check_results(results, KNOWN_FAILURES_SET, "set.test")


class TestIncrOldNative:
    """Run tmp/tcl9.0.3/tests/incr-old.test through the VM."""

    def test_incr_old(self) -> None:
        results = _run_test_file("incr-old.test")
        _check_results(results, KNOWN_FAILURES_INCR_OLD, "incr-old.test")


class TestVarNative:
    """Run tmp/tcl9.0.3/tests/var.test through the VM."""

    def test_var(self) -> None:
        results = _run_test_file("var.test")
        _check_results(results, KNOWN_FAILURES_VAR, "var.test")
