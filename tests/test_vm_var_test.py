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

KNOWN_FAILURES_INCR: set[str] = {
    # errorInfo format: "reading increment" annotation
    "incr-2.30",  # errorInfo: "reading increment" annotation
    "incr-2.31",  # errorInfo: "reading increment" annotation (compiled)
    "incr-2.32",  # errorInfo + "expected integer but got a list"
    "incr-2.33",  # errorInfo + "expected integer but got a list" (dict)
    # Write trace via upvar alias not propagated
    "incr-1.28",  # readonly trace on upvar alias not caught by bytecode incr
    "incr-2.28",  # readonly trace on upvar alias not caught (non-compiled)
    # Compiler: braced var name with parens treated as array ref
    "incr-1.30",  # incr {array($foo)} — braces should make it a scalar name
}

KNOWN_FAILURES_SET_OLD: set[str] = {
    # upvar alias + array set
    "set-old-8.38.2",  # array exists on upvar alias to array returns 0
    "set-old-8.38.3",  # array set on upvar alias to scalar element
    # Namespace validation for array set
    "set-old-8.38.5",  # array set bogusnamespace::var — no parent ns check
    "set-old-8.38.6",  # array set bogusnamespace::var (repeated)
    "set-old-8.38.7",  # array set bogusnamespace::var(0) — no parent ns check
    # array statistics format difference
    "set-old-8.49",  # simplified format vs Tcl's bucket-based statistics
    # Output quoting difference
    "set-old-8.56",  # backslash quoting of error msg differs from Tcl
    # Search invalidation: trace add on array element
    "set-old-9.10",  # trace add var a(b) should invalidate active searches
}

KNOWN_FAILURES_UPVAR: set[str] = {
    # Array element trace through upvar alias
    "upvar-5.4",  # array element read trace not firing
    "upvar-5.5",  # array element write trace not firing
    "upvar-5.6",  # array element unset trace not firing
    "upvar-5.7",  # trace details wrong
    # Upvar aliasing: copy instead of link
    "upvar-6.3",  # aliased var not reflecting changes
    "upvar-6.4",  # self-referencing upvar errorCode
    # Multi-level upvar resolution
    "upvar-7.1",  # upvar through multiple levels wrong
    # Namespace context
    "upvar-8.2.1",  # upvar in namespace context
    # Array element alias as scalar vs array
    "upvar-8.8",  # set b(2) on alias to array element — should error
    # Namespace variable validation
    "upvar-8.9",  # ns var referring to proc var: different error message
    # info frame missing line key
    "upvar-10.1",
    # Namespace-qualified upvar
    "upvar-NS-1.3",  # namespace error format
    "upvar-NS-1.4",  # namespace error format
    "upvar-NS-1.9",  # namespace error format
    # info frame missing line key
    "upvar-NS-3.1",
    "upvar-NS-3.2",
    "upvar-NS-3.3",
}

KNOWN_FAILURES_UPLEVEL: set[str] = {
    # Namespace command resolution
    "uplevel-6.1",  # uplevel in namespace eval doesn't resolve ns-shadowed commands
    # Unimplemented introspection
    "uplevel-8.0",  # ::tcl::unsupported::representation not implemented
}

KNOWN_FAILURES_SET: set[str] = {
    # Trace handling
    "set-1.15",  # write trace not modifying value
    "set-4.4",  # read-only trace blocking set
    # Complex array key
    "set-1.26",  # "{a},hej" key not accessible
    # errorInfo format
    "set-2.1",  # variable name quoting in errorInfo
    "set-2.4",  # truncated errorInfo
    "set-4.1",  # indirect call via $z not reflected
}

KNOWN_FAILURES_INCR_OLD: set[str] = {
    # errorInfo format: missing "while executing" annotation
    "incr-old-2.4",  # errorInfo lacks "while executing" context
    "incr-old-2.5",  # errorInfo lacks "(reading increment)" annotation
    "incr-old-2.6",  # trace error errorInfo lacks "while executing" context
    # Error message: "expected integer but got ..." vs "expected integer but got a list"
    "incr-old-2.10",  # error message format difference
}

KNOWN_FAILURES_VAR: set[str] = {
    # variable command: namespace scoping
    "var-1.11",  # variable qualified name
    "var-1.12",  # variable qualified name
    "var-1.13",  # variable qualified name
    "var-1.15",  # variable in nested namespace
    "var-1.16",  # variable in nested namespace
    "var-1.17",  # variable in nested namespace
    "var-1.18",  # variable in nested namespace
    "var-1.20",  # variable resolution order
    "var-1.21",  # variable resolution order
    # Error handling
    "var-3.5",  # wrong # args for variable
    "var-3.7",  # wrong # args for variable
    "var-3.9",  # namespace not found
    "var-3.11",  # namespace not found
    # Namespace variable resolution
    "var-5.2",  # namespace var alias
    "var-5.3",  # namespace var alias
    # Variable cleanup on namespace delete
    "var-6.1",  # namespace delete cleans up vars
    "var-6.2",  # namespace delete cleans up vars
    "var-6.3",  # namespace delete cleans up vars
    # Namespace var persistence / info vars / unset
    "var-7.9",  # namespace var persistence across evals
    "var-7.14",  # namespace var with traces
    "var-7.15",  # namespace var with traces
    # Variable traces through namespace
    "var-8.1",  # variable trace in namespace
    "var-8.2",  # variable trace in namespace
    # Array nesting / compilation
    "var-10.1",  # array set nesting (cascading)
    "var-10.2",  # array set nesting (cascading)
    "var-12.1",  # array set compilation (cascading)
    "var-15.2",  # array set compilation (cascading)
    "var-17.2",  # array set compilation (cascading)
    # Array set with traces / compiled ops
    "var-20.9",  # array set compiled w/ trace
    "var-20.10",  # array set bad varname
    "var-20.11",  # array set bad initializer
    "var-20.12",  # array set bad initializer
    "var-21.0",  # compiled unset OBOE
    # Array for loop
    "var-23.1",
    "var-23.2",
    "var-23.3",
    "var-23.4",
    "var-23.5",
    "var-23.6",
    "var-23.7",
    "var-23.9",
    "var-23.10",
    "var-23.11",
    "var-23.12",
    "var-23.13",
    "var-23.14",
    # Array for loop (nested)
    "var-24.1",
    "var-24.2",
    "var-24.3",
    "var-24.4",
    "var-24.5",
    "var-24.6",
    "var-24.7",
    "var-24.8",
    "var-24.9",
    "var-24.10",
    "var-24.11",
    "var-24.12",
    "var-24.13",
    "var-24.14",
    "var-24.15",
    "var-24.16",
    "var-24.19",
    "var-24.21",
    "var-24.23",
    # Array for loop (error)
    "var-25.1",
    "var-25.2",
    "var-25.3",
    "var-25.4",
    "var-25.5",
    # Array for loop (compiled)
    "var-26.1",
    "var-26.2",
    "var-26.3",
    "var-26.4",
    "var-26.5",
    "var-26.6",
    "var-26.7",
    "var-26.8",
    "var-26.9.1",
    "var-26.9.2",
    "var-26.10.1",
    "var-26.10.2",
    "var-26.11",
    "var-26.12",
    "var-26.13",
    "var-26.14",
    "var-26.15",
    "var-26.16",
    "var-26.17",
    "var-27.1",
    "var-27.2",
    "var-27.3",
    "var-27.4",
    "var-27.5",
    "var-27.6",
    "var-27.7",
    "var-27.8",
    "var-27.9.1",
    "var-27.9.2",
    "var-27.10.1",
    "var-27.10.2",
    "var-27.11",
    "var-27.12",
    "var-27.13",
    "var-27.14",
    "var-27.15",
    "var-27.16",
    "var-27.17",
    # Namespace var cleanup / deletion
    "var-28.1",
    "var-28.2",
    "var-28.3",
    "var-28.4",
    "var-28.5",
    "var-29.1",
    "var-29.2",
    "var-29.3",
    "var-29.4",
    "var-29.5",
    "var-29.6",
    "var-29.7",
    "var-30.1",
    "var-30.2",
    "var-30.3",
    "var-30.4",
    "var-30.5",
    "var-30.6",
    "var-30.7",
    "var-30.8",
    "var-30.9",
    "var-30.10",
    "var-30.11",
    "var-30.12",
    "var-30.13",
    "var-31.1",
    "var-31.2",
    "var-31.3",
}


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
    failed_set = set(results["failed_tests"])  # type: ignore[arg-type]
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
