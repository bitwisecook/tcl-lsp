"""Run control-flow test files natively through the VM's tcltest.

Covers for-old.test, while-old.test, if-old.test, foreach.test,
switch.test, append.test, eval.test, for.test, source.test, if.test,
and while.test from the Tcl 9.0.3 test suite (Phases 5b–5d of the VM
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
#
# Each set lists Tcl test names that are expected to fail in our VM.
# When a VM bug is fixed the test will unexpectedly pass — the set
# must be updated (removing the entry) to keep CI green.

KNOWN_FAILURES_FOR_OLD: set[str] = set()

KNOWN_FAILURES_WHILE_OLD: set[str] = set()

KNOWN_FAILURES_IF_OLD: set[str] = set()

KNOWN_FAILURES_FOREACH: set[str] = set()

KNOWN_FAILURES_SWITCH: set[str] = set()

KNOWN_FAILURES_APPEND: set[str] = set()

KNOWN_FAILURES_EVAL: set[str] = set()

KNOWN_FAILURES_FOR: set[str] = set()

KNOWN_FAILURES_IF: set[str] = set()

KNOWN_FAILURES_WHILE: set[str] = set()

KNOWN_FAILURES_SOURCE: set[str] = set()


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


class TestForOldNative:
    """Run tmp/tcl9.0.3/tests/for-old.test through the VM."""

    def test_for_old(self) -> None:
        results = _run_test_file("for-old.test")
        _check_results(results, KNOWN_FAILURES_FOR_OLD, "for-old.test")


class TestWhileOldNative:
    """Run tmp/tcl9.0.3/tests/while-old.test through the VM."""

    def test_while_old(self) -> None:
        results = _run_test_file("while-old.test")
        _check_results(results, KNOWN_FAILURES_WHILE_OLD, "while-old.test")


class TestIfOldNative:
    """Run tmp/tcl9.0.3/tests/if-old.test through the VM."""

    def test_if_old(self) -> None:
        results = _run_test_file("if-old.test")
        _check_results(results, KNOWN_FAILURES_IF_OLD, "if-old.test")


class TestForeachNative:
    """Run tmp/tcl9.0.3/tests/foreach.test through the VM."""

    def test_foreach(self) -> None:
        results = _run_test_file("foreach.test")
        _check_results(results, KNOWN_FAILURES_FOREACH, "foreach.test")


class TestSwitchNative:
    """Run tmp/tcl9.0.3/tests/switch.test through the VM."""

    def test_switch(self) -> None:
        results = _run_test_file("switch.test")
        _check_results(results, KNOWN_FAILURES_SWITCH, "switch.test")


class TestAppendNative:
    """Run tmp/tcl9.0.3/tests/append.test through the VM."""

    def test_append(self) -> None:
        results = _run_test_file("append.test")
        _check_results(results, KNOWN_FAILURES_APPEND, "append.test")


class TestEvalNative:
    """Run tmp/tcl9.0.3/tests/eval.test through the VM."""

    def test_eval(self) -> None:
        results = _run_test_file("eval.test")
        _check_results(results, KNOWN_FAILURES_EVAL, "eval.test")


class TestForNative:
    """Run tmp/tcl9.0.3/tests/for.test through the VM."""

    def test_for(self) -> None:
        results = _run_test_file("for.test")
        _check_results(results, KNOWN_FAILURES_FOR, "for.test")


class TestSourceNative:
    """Run tmp/tcl9.0.3/tests/source.test through the VM."""

    def test_source(self) -> None:
        results = _run_test_file("source.test")
        _check_results(results, KNOWN_FAILURES_SOURCE, "source.test")


class TestIfNative:
    """Run tmp/tcl9.0.3/tests/if.test through the VM."""

    def test_if(self) -> None:
        results = _run_test_file("if.test")
        _check_results(results, KNOWN_FAILURES_IF, "if.test")


class TestWhileNative:
    """Run tmp/tcl9.0.3/tests/while.test through the VM."""

    def test_while(self) -> None:
        results = _run_test_file("while.test")
        _check_results(results, KNOWN_FAILURES_WHILE, "while.test")
