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

KNOWN_FAILURES_SPLIT: set[str] = {
    # List element quoting in split result
    "split-1.1",  # result quoting of special chars
    "split-1.6",  # result quoting of special chars
    "split-1.8",  # result quoting of special chars
    # Error message format
    "split-2.1",  # wrong # args — errorCode
    "split-2.2",  # wrong # args — errorCode
}

KNOWN_FAILURES_FORMAT: set[str] = {
    # Integer formatting: 64-bit
    "format-1.12",  # %b binary format not fully supported
    "format-1.13",  # %lld wide int
    "format-1.14",  # %lld wide int
    "format-1.15",  # negative 64-bit
    # String formatting: %b binary
    "format-2.14",  # %b binary format
    "format-2.15",  # bad field specifier
    # Wide integer formatting
    "format-8.9",  # wide int overflow
    "format-8.10",  # wide int overflow
    "format-8.11",  # wide int overflow
    "format-8.12",  # wide int overflow
    "format-8.13",  # wide int overflow
    "format-8.14",  # wide int overflow
    "format-8.15",  # wide int overflow
    "format-8.16",  # wide int overflow
    "format-8.17",  # wide int overflow
    "format-8.18",  # wide int overflow
    "format-8.19",  # wide int overflow
    "format-8.20",  # wide int overflow
    "format-8.23",  # wide int overflow
    "format-8.24",  # wide int overflow
    "format-8.25",  # wide int overflow
    "format-8.26",  # wide int overflow
    "format-8.27",  # wide int overflow
    "format-8.28",  # wide int overflow
    # Positional argument (%n$) support
    "format-10.1",  # XPG3 positional args
    "format-10.2",  # XPG3 positional args
    "format-10.3",  # XPG3 positional args
    "format-10.5",  # XPG3 positional args
    # Error detection / edge cases
    "format-11.1",  # error handling
    "format-11.2",  # error handling
    "format-11.3",  # error handling
    "format-11.4",  # error handling
    "format-11.5",  # error handling
    "format-11.6",  # error handling
    "format-11.7",  # error handling
    "format-11.8",  # error handling
    "format-11.9",  # error handling
    "format-11.10",  # error handling
    "format-11.11",  # error handling
    "format-11.12",  # error handling
    "format-11.13",  # error handling
    "format-11.14",  # error handling
    # Unicode formatting
    "format-15.1",  # unicode format
    "format-15.4",  # unicode format
    # $::errorCode propagation
    "format-17.1",  # errorCode
    "format-17.6",  # errorCode
    # Bignum formatting
    "format-18.2",  # bignum
    # %d precision interaction
    "format-19.4.1",  # precision + padding
}

KNOWN_FAILURES_SUBST: set[str] = {
    # Backslash substitution edge cases
    "subst-1.2",  # backslash handling
    # Command substitution: error propagation
    "subst-3.1",  # error in command substitution
    # Option handling: -nobackslashes, -nocommands, -novariables
    "subst-4.3",  # bad option detection
    "subst-5.5",  # -nobackslashes
    "subst-5.6",  # -nobackslashes
    "subst-5.7",  # -nobackslashes
    "subst-5.8",  # -nobackslashes
    "subst-5.9",  # -nobackslashes
    "subst-5.10",  # -nobackslashes
    # -novariables option
    # -nocommands option
    "subst-7.1",  # -nocommands
    "subst-7.2",  # -nocommands
    "subst-7.3",  # -nocommands
    "subst-7.7",  # -nocommands
    # Break/continue/return in subst
    "subst-8.6",  # return in subst
    "subst-8.9",  # exception in subst
    # Error handling in subst
    "subst-10.6",  # error handling
    # Nested substitution
    "subst-11.6",  # nested subst
    # Continuation lines
    "subst-12.1",  # continuation line in subst
    "subst-12.2",  # continuation line in subst
    "subst-12.3",  # continuation line in subst
    "subst-12.4",  # continuation line in subst
    "subst-12.5",  # continuation line in subst
    # Interp / word boundaries
    "subst-13.1",  # interp alias boundary — flaky under parallel execution
    "subst-13.2",  # word boundary in subst
}

KNOWN_FAILURES_SCAN: set[str] = {
    # Basic scan conversion failures
    "scan-1.1",  # basic integer scan
    "scan-1.2",  # basic integer scan
    "scan-1.3",  # basic integer scan
    "scan-1.4",  # basic integer scan
    "scan-1.5",  # basic integer scan
    "scan-1.6",  # basic string scan
    "scan-1.7",  # basic string scan
    "scan-1.8",  # basic string scan
    # Whitespace / literal matching
    "scan-2.1",  # whitespace handling
    "scan-2.2",  # whitespace handling
    # Character set scanning
    "scan-3.1",  # character set scan
    "scan-3.2",  # character set scan
    "scan-3.3",  # character set scan
    "scan-3.5",  # character set scan
    "scan-3.6",  # character set scan
    "scan-3.7",  # character set scan
    "scan-3.8",  # character set scan
    "scan-3.9",  # character set scan
    "scan-3.10",  # character set scan
    "scan-3.11",  # character set scan
    "scan-3.12",  # character set scan
    "scan-3.13",  # character set scan
    # Format specifier edge cases
    "scan-4.4",  # hex scan
    "scan-4.8",  # octal scan
    "scan-4.10",  # float scan
    "scan-4.11",  # float scan
    "scan-4.12",  # float scan
    "scan-4.13",  # float scan
    "scan-4.14",  # %n position scan
    "scan-4.15",  # %n position scan
    "scan-4.17",  # wide integer scan
    "scan-4.18",  # wide integer scan
    "scan-4.19",  # wide integer scan
    "scan-4.22",  # %u unsigned scan
    "scan-4.23",  # %u unsigned scan
    "scan-4.24",  # %u unsigned scan
    "scan-4.25",  # %u unsigned scan
    "scan-4.26",  # 64-bit scan
    "scan-4.27",  # 64-bit scan
    "scan-4.29",  # %b binary scan
    "scan-4.30",  # %b binary scan
    "scan-4.38",  # wide integer scan
    "scan-4.39",  # wide integer scan
    "scan-4.40",  # wide integer scan
    "scan-4.40.3",  # wide integer scan
    "scan-4.41",  # wide integer scan
    "scan-4.42",  # wide integer scan
}

KNOWN_FAILURES_STRING: set[str] = {
    # Error message format
    "string-1.1.0",  # unknown subcommand error wording
    # string compare edge cases
    "string-2.2.0",  # compare with -length
    "string-2.3.0",  # compare with -length
    "string-2.7.0",  # compare with -nocase
    "string-2.10.0",  # compare error handling
    "string-2.20.0",  # compare with embedded null
    "string-2.21.0",  # compare with embedded null
    "string-2.34.0",  # compare Unicode
    "string-2.35.0",  # compare Unicode
    "string-2.36.0",  # compare Unicode
    "string-2.37.0",  # compare Unicode
    "string-2.38a.0",  # compare Unicode
    "string-2.38b.0",  # compare Unicode
    "string-2.38c.0",  # compare Unicode
    "string-2.38d.0",  # compare Unicode
    "string-2.38e.0",  # compare Unicode
    "string-2.38f.0",  # compare Unicode
    # string equal edge cases
    "string-3.2.0",  # equal with -length
    "string-3.6.0",  # equal with -nocase
    "string-3.10.0",  # equal error handling
    "string-3.11.0",  # equal error handling
    "string-3.15.0",  # equal with embedded null
    "string-3.28.0",  # equal Unicode
    "string-3.29.0",  # equal Unicode
    "string-3.40.0",  # equal Unicode
    "string-3.41.0",  # equal Unicode
    "string-3.42.0",  # equal Unicode
    "string-3.43.0",  # equal Unicode
    "string-3.44.0",  # equal Unicode
    "string-3.45a.0",  # equal Unicode
    "string-3.45b.0",  # equal Unicode
    "string-3.45c.0",  # equal Unicode
    "string-3.45d.0",  # equal Unicode
    "string-3.45e.0",  # equal Unicode
    "string-3.45f.0",  # equal Unicode
    # string first edge cases
    "string-4.3.0",  # first with embedded null
    "string-4.5.0",  # first with Unicode
    "string-4.6.0",  # first with Unicode
    "string-4.8.0",  # first error handling
    "string-4.16.0",  # first with -start
    "string-4.22.0",  # first edge case
    # string index edge cases
    "string-5.4.0",  # index with Unicode
    "string-5.7.0",  # index with end-N
    "string-5.8.0",  # index with end-N
    "string-5.9.0",  # index error handling
    "string-5.13.0",  # index with embedded null
    "string-5.14.0",  # index with large index
    "string-5.15.0",  # index with negative
    "string-5.16.0",  # index edge case
    "string-5.19.0",  # index edge case
    "string-5.20.0",  # index edge case
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


class TestSplitNative:
    """Run tmp/tcl9.0.3/tests/split.test through the VM."""

    def test_split(self) -> None:
        results = _run_test_file("split.test")
        _check_results(results, KNOWN_FAILURES_SPLIT, "split.test")


class TestFormatNative:
    """Run tmp/tcl9.0.3/tests/format.test through the VM."""

    def test_format(self) -> None:
        results = _run_test_file("format.test")
        _check_results(results, KNOWN_FAILURES_FORMAT, "format.test")


class TestSubstNative:
    """Run tmp/tcl9.0.3/tests/subst.test through the VM."""

    def test_subst(self) -> None:
        results = _run_test_file("subst.test")
        _check_results(results, KNOWN_FAILURES_SUBST, "subst.test")


class TestScanNative:
    """Run tmp/tcl9.0.3/tests/scan.test through the VM."""

    def test_scan(self) -> None:
        results = _run_test_file("scan.test")
        _check_results(results, KNOWN_FAILURES_SCAN, "scan.test")


class TestStringNative:
    """Run tmp/tcl9.0.3/tests/string.test through the VM."""

    def test_string(self) -> None:
        results = _run_test_file("string.test")
        _check_results(results, KNOWN_FAILURES_STRING, "string.test")
