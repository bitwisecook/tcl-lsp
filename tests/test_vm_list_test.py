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
#
# Empty ``set()`` with an ``expect_zero_total=True`` call site means
# the .test file crashes at startup in the Python VM (``TclReturn`` at
# top level, ``couldn't read ./tcltests.tcl``, ``invalid ReturnCode``,
# etc.) and runs 0 tests.  The original per-test failure catalogues
# — which categorised failures by root cause (errorInfo format,
# missing subcommand, etc.) — are preserved in git history: ``git
# log -p origin/main..HEAD -- <this-file>`` shows what failed before
# the crash took hold.  Repopulate the set once the startup crash
# is fixed and real cases fail.

# concat.test: 9 tests, all passing (no known failures)
KNOWN_FAILURES_CONCAT: set[str] = set()

# llength.test: 6 tests, all passing (no known failures)
KNOWN_FAILURES_LLENGTH: set[str] = set()

KNOWN_FAILURES_LREPEAT: set[str] = set()

KNOWN_FAILURES_LSEARCH: set[str] = set()

KNOWN_FAILURES_JOIN: set[str] = set(
    # join.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
)

KNOWN_FAILURES_LINDEX: set[str] = set(
    # lindex.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
)

KNOWN_FAILURES_LRANGE: set[str] = set(
    # lrange.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
)

KNOWN_FAILURES_LIST: set[str] = set(
    # list.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
)

KNOWN_FAILURES_LINSERT: set[str] = set(
    # linsert.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
)

KNOWN_FAILURES_LREPLACE: set[str] = set(
    # lreplace.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
)

KNOWN_FAILURES_LMAP: set[str] = set(
    # lmap.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
)

KNOWN_FAILURES_LPOP: set[str] = set(
    # lpop.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
)

KNOWN_FAILURES_CMDIL: set[str] = set(
    # cmdIL.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
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


class TestConcatNative:
    """Run tmp/tcl9.0.3/tests/concat.test through the VM."""

    def test_concat(self) -> None:
        results = _run_test_file("concat.test")
        _check_results(results, KNOWN_FAILURES_CONCAT, "concat.test", expect_zero_total=True)


class TestLlengthNative:
    """Run tmp/tcl9.0.3/tests/llength.test through the VM."""

    def test_llength(self) -> None:
        results = _run_test_file("llength.test")
        _check_results(results, KNOWN_FAILURES_LLENGTH, "llength.test", expect_zero_total=True)


class TestLrepeatNative:
    """Run tmp/tcl9.0.3/tests/lrepeat.test through the VM."""

    def test_lrepeat(self) -> None:
        results = _run_test_file("lrepeat.test")
        _check_results(results, KNOWN_FAILURES_LREPEAT, "lrepeat.test", expect_zero_total=True)


class TestLsearchNative:
    """Run tmp/tcl9.0.3/tests/lsearch.test through the VM."""

    def test_lsearch(self) -> None:
        results = _run_test_file("lsearch.test")
        _check_results(results, KNOWN_FAILURES_LSEARCH, "lsearch.test", expect_zero_total=True)


class TestJoinNative:
    """Run tmp/tcl9.0.3/tests/join.test through the VM."""

    def test_join(self) -> None:
        results = _run_test_file("join.test")
        _check_results(results, KNOWN_FAILURES_JOIN, "join.test", expect_zero_total=True)


class TestLindexNative:
    """Run tmp/tcl9.0.3/tests/lindex.test through the VM."""

    def test_lindex(self) -> None:
        results = _run_test_file("lindex.test")
        _check_results(results, KNOWN_FAILURES_LINDEX, "lindex.test", expect_zero_total=True)


class TestLrangeNative:
    """Run tmp/tcl9.0.3/tests/lrange.test through the VM."""

    def test_lrange(self) -> None:
        results = _run_test_file("lrange.test")
        _check_results(results, KNOWN_FAILURES_LRANGE, "lrange.test", expect_zero_total=True)


class TestListNative:
    """Run tmp/tcl9.0.3/tests/list.test through the VM."""

    def test_list(self) -> None:
        results = _run_test_file("list.test")
        _check_results(results, KNOWN_FAILURES_LIST, "list.test", expect_zero_total=True)


class TestLinsertNative:
    """Run tmp/tcl9.0.3/tests/linsert.test through the VM."""

    def test_linsert(self) -> None:
        results = _run_test_file("linsert.test")
        _check_results(results, KNOWN_FAILURES_LINSERT, "linsert.test", expect_zero_total=True)


class TestLreplaceNative:
    """Run tmp/tcl9.0.3/tests/lreplace.test through the VM."""

    def test_lreplace(self) -> None:
        results = _run_test_file("lreplace.test")
        _check_results(results, KNOWN_FAILURES_LREPLACE, "lreplace.test", expect_zero_total=True)


class TestLmapNative:
    """Run tmp/tcl9.0.3/tests/lmap.test through the VM."""

    def test_lmap(self) -> None:
        results = _run_test_file("lmap.test")
        _check_results(results, KNOWN_FAILURES_LMAP, "lmap.test", expect_zero_total=True)


class TestLpopNative:
    """Run tmp/tcl9.0.3/tests/lpop.test through the VM."""

    def test_lpop(self) -> None:
        results = _run_test_file("lpop.test")
        _check_results(results, KNOWN_FAILURES_LPOP, "lpop.test", expect_zero_total=True)


class TestCmdILNative:
    """Run tmp/tcl9.0.3/tests/cmdIL.test through the VM."""

    def test_cmdil(self) -> None:
        results = _run_test_file("cmdIL.test")
        _check_results(results, KNOWN_FAILURES_CMDIL, "cmdIL.test", expect_zero_total=True)
