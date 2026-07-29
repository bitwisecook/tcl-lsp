# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

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
from tooling.vm.commands import tcltest_cmds
from tooling.vm.commands.test_support_cmds import setup_test_support
from tooling.vm.interp import TclInterp

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

# concat.test crashes at startup in the Python VM (Total=0); the
# caller passes ``expect_zero_total=True``.  The earlier "9 tests,
# all passing" note was accurate before the startup crash took hold.
KNOWN_FAILURES_CONCAT: set[str] = set()

# llength.test — same startup crash as concat.test.
KNOWN_FAILURES_LLENGTH: set[str] = set()

# lrepeat.test — same.
KNOWN_FAILURES_LREPEAT: set[str] = set()

# lsearch.test — same.
KNOWN_FAILURES_LSEARCH: set[str] = {
    "lsearch-10.1",
    "lsearch-10.10",
    "lsearch-10.2",
    "lsearch-10.3",
    "lsearch-10.4",
    "lsearch-10.5",
    "lsearch-10.6",
    "lsearch-10.7",
    "lsearch-10.8",
    "lsearch-10.9",
    "lsearch-12.2",
    "lsearch-14.5",
    "lsearch-14.6",
    "lsearch-14.7",
    "lsearch-14.8",
    "lsearch-15.1",
    "lsearch-17.1",
    "lsearch-17.10",
    "lsearch-17.11",
    "lsearch-17.12",
    "lsearch-17.13",
    "lsearch-17.14",
    "lsearch-17.15",
    "lsearch-17.16",
    "lsearch-17.2",
    "lsearch-17.3",
    "lsearch-17.4",
    "lsearch-17.5",
    "lsearch-17.6",
    "lsearch-17.7",
    "lsearch-17.8",
    "lsearch-17.9",
    "lsearch-18.1",
    "lsearch-18.2",
    "lsearch-18.3",
    "lsearch-18.4",
    "lsearch-18.5",
    "lsearch-19.1",
    "lsearch-19.2",
    "lsearch-19.3",
    "lsearch-19.4",
    "lsearch-19.5",
    "lsearch-19.6",
    "lsearch-19.7",
    "lsearch-19.8",
    "lsearch-2.10",
    "lsearch-2.6",
    "lsearch-20.1",
    "lsearch-20.2",
    "lsearch-20.3",
    "lsearch-22.1",
    "lsearch-22.2",
    "lsearch-22.3",
    "lsearch-22.4",
    "lsearch-22.5",
    "lsearch-23.1",
    "lsearch-23.2",
    "lsearch-23.3",
    "lsearch-23.4",
    "lsearch-23.5",
    "lsearch-24.1",
    "lsearch-24.10",
    "lsearch-24.11",
    "lsearch-24.2",
    "lsearch-24.3",
    "lsearch-24.4",
    "lsearch-24.5",
    "lsearch-24.6",
    "lsearch-24.7",
    "lsearch-24.8",
    "lsearch-24.9",
    "lsearch-25.1",
    "lsearch-25.2",
    "lsearch-25.3",
    "lsearch-25.4",
    "lsearch-25.5",
    "lsearch-25.6",
    "lsearch-26.1",
    "lsearch-26.2",
    "lsearch-26.3",
    "lsearch-26.4",
    "lsearch-26.5",
    "lsearch-26.6",
    "lsearch-27.1",
    "lsearch-27.2",
    "lsearch-27.3",
    "lsearch-27.4",
    "lsearch-27.5",
    "lsearch-27.6",
    "lsearch-27.7",
    "lsearch-27.8",
    "lsearch-28.1",
    "lsearch-28.10",
    "lsearch-28.2",
    "lsearch-28.3",
    "lsearch-28.4",
    "lsearch-28.5",
    "lsearch-28.6",
    "lsearch-28.7",
    "lsearch-28.8",
    "lsearch-28.9",
    "lsearch-3.1",
    "lsearch-3.2",
    "lsearch-3.3",
    "lsearch-3.4",
    "lsearch-3.6",
    "lsearch-3.7",
    "lsearch-8.1",
    "lsearch-8.2",
    "lsearch-8.3",
    "lsearch-8.4",
}

KNOWN_FAILURES_JOIN: set[str] = {
    "join-2.1",
    "join-2.2",
    "join-2.3",
}

KNOWN_FAILURES_LINDEX: set[str] = {
    "lindex-10.1",
    "lindex-10.3",
    "lindex-10.4",
    "lindex-12.10",
    "lindex-12.8",
    "lindex-14.3",
    "lindex-16.4",
    "lindex-16.5",
    "lindex-16.6",
    "lindex-16.7",
    "lindex-17.0",
    "lindex-18.0",
    "lindex-3.9",
}

KNOWN_FAILURES_LRANGE: set[str] = {
    "lrange-3.3",
    "lrange-3.5",
    "lrange-3.6",
    "lrange-3.7a",
    "lrange-4.1",
    "lrange-4.2",
    "lrange-4.3",
    "lrange-4.4",
}

KNOWN_FAILURES_LIST: set[str] = {
    "list-1.27",
    "list-1.30",
    "list-3.1",
    "list-4.3",
}

KNOWN_FAILURES_LINSERT: set[str] = {
    "linsert-3.2",
}

KNOWN_FAILURES_LREPLACE: set[str] = set()

KNOWN_FAILURES_LMAP: set[str] = {
    "lmap-1.13",
    "lmap-1.15",
    "lmap-2.9",
    "lmap-4.13",
    "lmap-4.15",
    "lmap-5.9",
    "lmap-8.1",
    "lmap-8.2",
}

KNOWN_FAILURES_LPOP: set[str] = {
    "lpop-1.4",
    "lpop-1.4b",
    "lpop-1.5",
    "lpop-1.6",
    "lpop-1.8",
}

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
    failed_set: set[str] = {str(name) for name in failed_tests}
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
        _check_results(results, KNOWN_FAILURES_CMDIL, "cmdIL.test", expect_zero_total=True)
