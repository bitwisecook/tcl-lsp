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

KNOWN_FAILURES_INCR: set[str] = {
    "incr-1.28",
    "incr-1.30",
    "incr-2.28",
    "incr-2.30",
    "incr-2.31",
    "incr-2.32",
    "incr-2.33",
}

KNOWN_FAILURES_SET_OLD: set[str] = {
    "set-old-8.38.2",
    "set-old-8.38.3",
    "set-old-8.38.5",
    "set-old-8.38.6",
    "set-old-8.38.7",
    "set-old-8.49",
    "set-old-8.56",
    "set-old-9.10",
}

KNOWN_FAILURES_UPVAR: set[str] = {
    "upvar-10.1",
    "upvar-8.8",
    "upvar-8.9",
    "upvar-NS-1.3",
    "upvar-NS-1.4",
    "upvar-NS-1.9",
    "upvar-NS-3.1",
    "upvar-NS-3.2",
    "upvar-NS-3.3",
}

KNOWN_FAILURES_UPLEVEL: set[str] = {
    "uplevel-6.1",
    "uplevel-8.0",
}

KNOWN_FAILURES_SET: set[str] = {
    "set-1.15",
    "set-1.26",
    "set-2.1",
    "set-2.4",
    "set-4.1",
    "set-4.4",
}

KNOWN_FAILURES_INCR_OLD: set[str] = {
    "incr-old-2.10",
    "incr-old-2.4",
    "incr-old-2.5",
    "incr-old-2.6",
}

KNOWN_FAILURES_VAR: set[str] = {
    "var-29.2",
    "var-29.3",
    "var-29.4",
    "var-29.5",
    "var-29.6",
    "var-29.7",
    "var-30.1",
    "var-30.10",
    "var-30.11",
    "var-30.12",
    "var-30.13",
    "var-30.2",
    "var-30.3",
    "var-30.4",
    "var-30.5",
    "var-30.6",
    "var-30.7",
    "var-30.8",
    "var-30.9",
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
