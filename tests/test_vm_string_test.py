"""Run string-related test files natively through the VM's tcltest.

Covers split.test, format.test, subst.test, scan.test, and string.test
from the Tcl 9.0.3 test suite (Phases 5c–5d of the VM test conformance
plan).

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

KNOWN_FAILURES_SPLIT: set[str] = set(
    # split.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
)

KNOWN_FAILURES_FORMAT: set[str] = set(
    # format.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
)

KNOWN_FAILURES_SUBST: set[str] = set(
    # subst.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
)

KNOWN_FAILURES_SCAN: set[str] = set(
    # scan.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
)

KNOWN_FAILURES_STRING: set[str] = set(
    # string.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
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

    # Wrap constraint-setup calls that use unimplemented commands
    # in ``catch`` so they default to false instead of aborting
    # the entire file.  scan.test defines testIEEE locally.
    script = script.replace(
        "testConstraint ieeeFloatingPoint [testIEEE]",
        "catch {testConstraint ieeeFloatingPoint [testIEEE]}",
    )

    # string.test sources tcltests.tcl via [info script] which
    # resolves to the wrong path inside our VM.  Wrap in catch so
    # the rest of the file still runs (the sourced file only sets
    # constraints that are already false in our environment).
    script = script.replace(
        "source [file join [file dirname [info script]] tcltests.tcl]",
        "catch {source [file join [file dirname [info script]] tcltests.tcl]}",
    )

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
    *,
    expect_zero_total: bool = False,
) -> None:
    """Assert that failures are exactly the known set.

    When ``expect_zero_total`` is False (default), Total must be > 0 —
    a 0-total run means the .test file crashed at startup before any
    tcltest case executed, and that should fail loudly so we notice
    regressions.  Files whose .test script is *known* to crash at
    startup opt in with ``expect_zero_total=True``; flipping that flag
    back to False is the signal we've fixed whatever was crashing."""
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
    if total == 0 and not expect_zero_total:
        pytest.fail(
            f"{test_file} ran 0 tests (Total=0).  The .test file probably "
            f"crashed at startup; fix the root cause, or pass "
            f"``expect_zero_total=True`` if the crash is the expected state."
        )
    if total != 0 and expect_zero_total:
        pytest.fail(
            f"{test_file} now runs {total} tests, but is marked "
            f"``expect_zero_total=True``.  Remove that flag and repopulate "
            f"known_failures based on what actually fails now."
        )
    if expect_zero_total and known_failures:
        pytest.fail(
            f"{test_file}: ``expect_zero_total=True`` requires known_failures "
            f"to be empty (no tests ran, so nothing can be 'known to fail'); "
            f"clear the set.  Found {len(known_failures)} entries."
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


class TestSplitNative:
    """Run tmp/tcl9.0.3/tests/split.test through the VM."""

    def test_split(self) -> None:
        results = _run_test_file("split.test")
        _check_results(results, KNOWN_FAILURES_SPLIT, "split.test", expect_zero_total=True)


class TestFormatNative:
    """Run tmp/tcl9.0.3/tests/format.test through the VM."""

    def test_format(self) -> None:
        results = _run_test_file("format.test")
        _check_results(results, KNOWN_FAILURES_FORMAT, "format.test", expect_zero_total=True)


class TestSubstNative:
    """Run tmp/tcl9.0.3/tests/subst.test through the VM."""

    def test_subst(self) -> None:
        results = _run_test_file("subst.test")
        _check_results(results, KNOWN_FAILURES_SUBST, "subst.test", expect_zero_total=True)


class TestScanNative:
    """Run tmp/tcl9.0.3/tests/scan.test through the VM."""

    def test_scan(self) -> None:
        results = _run_test_file("scan.test")
        _check_results(results, KNOWN_FAILURES_SCAN, "scan.test", expect_zero_total=True)


class TestStringNative:
    """Run tmp/tcl9.0.3/tests/string.test through the VM."""

    def test_string(self) -> None:
        results = _run_test_file("string.test")
        _check_results(results, KNOWN_FAILURES_STRING, "string.test", expect_zero_total=True)
