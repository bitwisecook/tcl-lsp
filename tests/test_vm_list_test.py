"""Run list-related test files natively through the VM's tcltest.

Covers concat.test, llength.test, lrepeat.test, lsearch.test, join.test,
lindex.test, lrange.test, list.test, lmap.test, linsert.test,
lreplace.test, lpop.test, and cmdIL.test from the Tcl 9.0.3 test suite
(Phase 5c of the VM test conformance plan).

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

# concat.test: 9 tests, all passing (no known failures)
KNOWN_FAILURES_CONCAT: set[str] = set()

# llength.test: 6 tests, all passing (no known failures)
KNOWN_FAILURES_LLENGTH: set[str] = set()

KNOWN_FAILURES_LREPEAT: set[str] = set()

KNOWN_FAILURES_LSEARCH: set[str] = set()

KNOWN_FAILURES_JOIN: set[str] = {
    # $::errorCode not set to TCL WRONGARGS
    "join-2.1",  # wrong # args — errorCode missing
    "join-2.2",  # wrong # args — errorCode missing
    "join-2.3",  # wrong # args — errorCode missing
}

KNOWN_FAILURES_LINDEX: set[str] = {
    # Nested list indexing
    "lindex-3.9",  # error on nested index
    # lindex with negative/out-of-range
    "lindex-10.1",
    "lindex-10.3",
    "lindex-10.4",
    # Compiled lindex differences
    "lindex-12.8",
    "lindex-12.10",
    "lindex-14.3",
    "lindex-15.3",
    # Error message format
    "lindex-16.4",
    "lindex-16.5",
    "lindex-16.6",
    "lindex-16.7",
    # Integer overflow handling
    "lindex-17.0",
    "lindex-18.0",  # 0+0x10000000000000000 index arithmetic with hex
}

KNOWN_FAILURES_LRANGE: set[str] = {
    # end-based index arithmetic
    "lrange-1.15",
    # Internal object representation tests
    "lrange-4.1",  # tcl::unsupported::representation
    "lrange-4.2",  # tcl::unsupported::representation
    "lrange-4.3",  # tcl::unsupported::representation
    "lrange-4.4",  # tcl::unsupported::representation
    "lrange-1.16",
    # List element quoting
    "lrange-3.3",
    "lrange-3.5",
    "lrange-3.6",
    "lrange-3.7a",
}

KNOWN_FAILURES_LIST: set[str] = {
    # List element quoting/escaping
    "list-1.4",
    "list-1.5",
    "list-1.10",
    "list-1.12",
    "list-1.17",
    "list-1.18",
    "list-1.19",
    "list-1.20",
    "list-1.21",
    "list-1.25",
    "list-1.26",
    "list-1.27",
    "list-1.30",
    # lappend/concat list canonicalisation
    "list-2.7-0",
    "list-2.7-2",
    "list-2.10-0",
    "list-2.10-1",
    "list-2.10-2",
    "list-2.11-0",
    "list-2.11-1",
    "list-2.11-2",
    "list-2.13-1",
    "list-2.13-2",
    "list-2.14-0",
    # Error handling
    "list-3.1",  # error propagation through list command
    # Unicode/special chars
    "list-4.2",
    "list-4.3",
}

KNOWN_FAILURES_LINSERT: set[str] = {
    # list quoting edge cases
    "linsert-1.8",  # brace quoting for special chars
    "linsert-1.15",  # list element quoting (backslash-space)
    "linsert-1.16",  # list element quoting (backslash-brace combo)
    # Edge cases
    "linsert-3.2",  # internal rep / shimmer
}

KNOWN_FAILURES_LREPLACE: set[str] = {
    # list quoting edge cases
    "lreplace-1.25",  # brace quoting in replacement
}

KNOWN_FAILURES_LMAP: set[str] = {
    # List parsing: braced element followed by non-space
    "lmap-1.13",
    "lmap-4.13",
    # Result collection / list quoting
    "lmap-1.15",
    "lmap-2.9",
    "lmap-4.15",
    "lmap-5.9",
    # Coroutine not implemented
    "lmap-8.1",
    "lmap-8.2",
}

KNOWN_FAILURES_LPOP: set[str] = {
    # nested index / deep lpop
    "lpop-1.4",  # nested lpop with lindex-style multi-index
    "lpop-1.4b",  # nested lpop with lindex-style multi-index
    "lpop-1.5",  # nested lpop with lindex-style multi-index
    "lpop-1.6",  # nested lpop with lindex-style multi-index
    "lpop-1.8",  # nested lpop with lindex-style multi-index
}

KNOWN_FAILURES_CMDIL: set[str] = {
    # lsort basic ordering
    "cmdIL-1.1",
    "cmdIL-1.2",
    "cmdIL-1.3",
    "cmdIL-1.5",
    "cmdIL-1.6",
    "cmdIL-1.8",
    "cmdIL-1.9",
    "cmdIL-1.11",
    "cmdIL-1.12",
    "cmdIL-1.13",
    "cmdIL-1.14",
    "cmdIL-1.23",
    "cmdIL-1.24",
    "cmdIL-1.25",
    "cmdIL-1.26",
    "cmdIL-1.27",
    "cmdIL-1.28",
    "cmdIL-1.30",
    "cmdIL-1.31",
    "cmdIL-1.32",
    "cmdIL-1.33",
    "cmdIL-1.34",
    "cmdIL-1.35",
    "cmdIL-1.36",
    "cmdIL-1.37",
    "cmdIL-1.38",
    "cmdIL-1.39",
    "cmdIL-1.40",
    "cmdIL-1.41",
    "cmdIL-1.42",
    "cmdIL-1.43",
    # lsort -command
    "cmdIL-3.1",
    "cmdIL-3.2",
    "cmdIL-3.3",
    "cmdIL-3.4",
    "cmdIL-3.4.1",
    "cmdIL-3.5",
    "cmdIL-3.5.1",
    "cmdIL-3.5.2",
    "cmdIL-3.5.3",
    "cmdIL-3.5.4",
    "cmdIL-3.5.5",
    "cmdIL-3.5.6",
    "cmdIL-3.5.7",
    "cmdIL-3.5.8",
    "cmdIL-3.5.9",
    "cmdIL-3.5.10",
    "cmdIL-3.6",
    "cmdIL-3.8",
    "cmdIL-3.11",
    "cmdIL-3.15",
    "cmdIL-3.17",
    "cmdIL-3.18",
    # lsort -index
    "cmdIL-4.1",
    "cmdIL-4.2",
    "cmdIL-4.4",
    "cmdIL-4.6",
    "cmdIL-4.7",
    "cmdIL-4.8",
    "cmdIL-4.17",
    "cmdIL-4.20",
    "cmdIL-4.26",
    "cmdIL-4.27",
    "cmdIL-4.28",
    "cmdIL-4.29",
    "cmdIL-4.30",
    "cmdIL-4.31",
    "cmdIL-4.32",
    "cmdIL-4.33",
    "cmdIL-4.36",
    "cmdIL-4.37",
    "cmdIL-4.38",
    # lsort error handling
    "cmdIL-5.1",
    "cmdIL-5.2",
    "cmdIL-5.3",
    "cmdIL-5.4",
    "cmdIL-5.5",
    "cmdIL-5.6",
    # lsort -stride
    "cmdIL-6.27",
    # lsort -command / stability
    "cmdIL-8.1",
    "cmdIL-8.2",
    "cmdIL-8.3",
    "cmdIL-8.4",
    "cmdIL-8.5",
    "cmdIL-8.6",
    "cmdIL-8.7",
    "cmdIL-8.8",
    "cmdIL-8.9",
    "cmdIL-8.10",
    "cmdIL-8.11",
    "cmdIL-8.12",
    "cmdIL-8.13",
    "cmdIL-8.14",
    "cmdIL-8.15",
    # info complete
    "info-20.6",
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
    interp.script_file = str(path.resolve())
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


class TestConcatNative:
    """Run tmp/tcl9.0.3/tests/concat.test through the VM."""

    def test_concat(self) -> None:
        results = _run_test_file("concat.test")
        _check_results(results, KNOWN_FAILURES_CONCAT, "concat.test")


class TestLlengthNative:
    """Run tmp/tcl9.0.3/tests/llength.test through the VM."""

    def test_llength(self) -> None:
        results = _run_test_file("llength.test")
        _check_results(results, KNOWN_FAILURES_LLENGTH, "llength.test")


class TestLrepeatNative:
    """Run tmp/tcl9.0.3/tests/lrepeat.test through the VM."""

    def test_lrepeat(self) -> None:
        results = _run_test_file("lrepeat.test")
        _check_results(results, KNOWN_FAILURES_LREPEAT, "lrepeat.test")


class TestLsearchNative:
    """Run tmp/tcl9.0.3/tests/lsearch.test through the VM."""

    def test_lsearch(self) -> None:
        results = _run_test_file("lsearch.test")
        _check_results(results, KNOWN_FAILURES_LSEARCH, "lsearch.test")


class TestJoinNative:
    """Run tmp/tcl9.0.3/tests/join.test through the VM."""

    def test_join(self) -> None:
        results = _run_test_file("join.test")
        _check_results(results, KNOWN_FAILURES_JOIN, "join.test")


class TestLindexNative:
    """Run tmp/tcl9.0.3/tests/lindex.test through the VM."""

    def test_lindex(self) -> None:
        results = _run_test_file("lindex.test")
        _check_results(results, KNOWN_FAILURES_LINDEX, "lindex.test")


class TestLrangeNative:
    """Run tmp/tcl9.0.3/tests/lrange.test through the VM."""

    def test_lrange(self) -> None:
        results = _run_test_file("lrange.test")
        _check_results(results, KNOWN_FAILURES_LRANGE, "lrange.test")


class TestListNative:
    """Run tmp/tcl9.0.3/tests/list.test through the VM."""

    def test_list(self) -> None:
        results = _run_test_file("list.test")
        _check_results(results, KNOWN_FAILURES_LIST, "list.test")


class TestLinsertNative:
    """Run tmp/tcl9.0.3/tests/linsert.test through the VM."""

    def test_linsert(self) -> None:
        results = _run_test_file("linsert.test")
        _check_results(results, KNOWN_FAILURES_LINSERT, "linsert.test")


class TestLreplaceNative:
    """Run tmp/tcl9.0.3/tests/lreplace.test through the VM."""

    def test_lreplace(self) -> None:
        results = _run_test_file("lreplace.test")
        _check_results(results, KNOWN_FAILURES_LREPLACE, "lreplace.test")


class TestLmapNative:
    """Run tmp/tcl9.0.3/tests/lmap.test through the VM."""

    def test_lmap(self) -> None:
        results = _run_test_file("lmap.test")
        _check_results(results, KNOWN_FAILURES_LMAP, "lmap.test")


class TestLpopNative:
    """Run tmp/tcl9.0.3/tests/lpop.test through the VM."""

    def test_lpop(self) -> None:
        results = _run_test_file("lpop.test")
        _check_results(results, KNOWN_FAILURES_LPOP, "lpop.test")


class TestCmdILNative:
    """Run tmp/tcl9.0.3/tests/cmdIL.test through the VM."""

    def test_cmdil(self) -> None:
        results = _run_test_file("cmdIL.test")
        _check_results(results, KNOWN_FAILURES_CMDIL, "cmdIL.test")
