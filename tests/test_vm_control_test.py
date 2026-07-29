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

KNOWN_FAILURES_FOR_OLD: set[str] = {
    "for-old-1.7",
    "for-old-1.8",
}

KNOWN_FAILURES_WHILE_OLD: set[str] = {
    "while-old-1.5",
    "while-old-4.3",
}

KNOWN_FAILURES_IF_OLD: set[str] = {
    "if-old-2.5",
    "if-old-2.8",
    "if-old-4.10",
    "if-old-4.3",
    "if-old-4.7",
    "if-old-4.8",
}

KNOWN_FAILURES_FOREACH: set[str] = {
    "foreach-1.12",
    "foreach-1.14",
    "foreach-2.8",
    "foreach-5.4",
    "foreach-5.5",
    "foreach-6.3",
    "foreach-6.4",
    "foreach-9.2",
}

KNOWN_FAILURES_SWITCH: set[str] = {
    "switch-11.1",
    "switch-11.3",
    "switch-11.4",
    "switch-11.5",
    "switch-11.6",
    "switch-12.1",
    "switch-12.3",
    "switch-12.4",
    "switch-12.5",
    "switch-12.6",
    "switch-12.7",
    "switch-12.8",
    "switch-12.9",
    "switch-13.1",
    "switch-13.2",
    "switch-13.3",
    "switch-13.4",
    "switch-13.5",
    "switch-13.6",
    "switch-14.1",
    "switch-14.10",
    "switch-14.13",
    "switch-14.16",
    "switch-14.17",
    "switch-14.2",
    "switch-14.7",
    "switch-15.1",
    "switch-3.15",
    "switch-3.16",
    "switch-3.17",
    "switch-3.18",
    "switch-4.1",
    "switch-4.2",
    "switch-4.5",
    "switch-5.1",
    "switch-6.1",
    "switch-6.2",
    "switch-8.1",
    "switch-9.1",
    "switch-9.10",
    "switch-9.2",
    "switch-9.3",
    "switch-9.4",
    "switch-9.8",
}

KNOWN_FAILURES_APPEND: set[str] = {
    "append-10.2",
    "append-10.4",
    "append-3.3",
    "append-3.5",
    "append-3.6",
    "append-4.10",
    "append-4.11",
    "append-4.12",
    "append-4.13",
    "append-4.14",
    "append-4.15",
    "append-4.16",
    "append-4.21",
    "append-4.22",
    "append-4.7",
    "append-4.9",
    "append-7.1",
    "append-7.5",
}

KNOWN_FAILURES_EVAL: set[str] = {
    "eval-2.5",
}

KNOWN_FAILURES_FOR: set[str] = {
    "for-1.10",
    "for-1.2",
    "for-1.4",
    "for-2.1",
    "for-3.1",
    "for-6.13",
    "for-6.17",
    "for-6.18",
    "for-6.6",
    "for-6.7",
    "for-6.9",
}

KNOWN_FAILURES_IF: set[str] = {
    "if-1.13",
    "if-1.17",
    "if-1.3",
    "if-10.1",
    "if-10.2",
    "if-10.3",
    "if-10.4",
    "if-10.5",
    "if-10.6",
    "if-2.2",
    "if-2.3",
    "if-2.4",
    "if-3.2",
    "if-5.10",
    "if-5.17",
    "if-5.3",
    "if-6.2",
    "if-6.4",
    "if-7.2",
    "if-7.4",
}

KNOWN_FAILURES_WHILE: set[str] = {
    "while-1.10",
    "while-1.2",
    "while-4.3",
    "while-4.9",
}

KNOWN_FAILURES_SOURCE: set[str] = {
    "source-2.3",
    "source-2.6",
    "source-2.7",
    "source-3.3",
    "source-3.4",
    "source-3.5",
    "source-4.1",
    "source-6.2",
    "source-7.3",
    "source-7.4",
    "source-7.6",
    "source-8.1",
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
    *,
    expect_zero_total: bool = False,
) -> None:
    """Assert that failures are exactly the known set.

    - Unexpected failures (not in known set) -> test fails
    - Unexpected passes (in known set but passed) -> test fails
      (forces cleanup of the known-failure set when bugs are fixed)
    """
    failed_tests = results["failed_tests"]
    assert isinstance(failed_tests, list)
    failed_set: set[str] = {str(name) for name in failed_tests}
    total = results["Total"]
    passed = results["Passed"]
    skipped = results["Skipped"]

    # Print summary
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
