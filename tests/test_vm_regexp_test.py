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

"""Run regexp-related test files natively through the VM's tcltest.

Covers regexp.test and regexpComp.test from the Tcl 9.0.3
test suite (Phase 13 of the VM test conformance plan).

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

KNOWN_FAILURES_REGEXP: set[str] = {
    "regexp-11.4",
    "regexp-11.5",
    "regexp-11.6",
    "regexp-11.8",
    "regexp-15.10",
    "regexp-15.6",
    "regexp-15.9",
    "regexp-16.20",
    "regexp-16.21",
    "regexp-16.22",
    "regexp-16.4",
    "regexp-16.7",
    "regexp-16.8",
    "regexp-17.7",
    "regexp-18.10",
    "regexp-18.12",
    "regexp-18.7",
    "regexp-18.8",
    "regexp-18.9",
    "regexp-2.10",
    "regexp-2.9",
    "regexp-20.2",
    "regexp-22.4",
    "regexp-22.5",
    "regexp-23.2",
    "regexp-23.3",
    "regexp-24.1",
    "regexp-24.10",
    "regexp-24.2",
    "regexp-24.3",
    "regexp-24.4",
    "regexp-24.5",
    "regexp-24.6",
    "regexp-24.7",
    "regexp-24.8",
    "regexp-24.9",
    "regexp-25.1",
    "regexp-26.10",
    "regexp-26.11",
    "regexp-26.12",
    "regexp-26.13",
    "regexp-26.8",
    "regexp-26.9",
    "regexp-27.8",
    "regexp-6.10",
    "regexp-6.4",
    "regexp-6.5",
    "regexp-6.9",
}

KNOWN_FAILURES_REGEXPCOMP: set[str] = {
    "regexpComp-11.4",
    "regexpComp-11.5",
    "regexpComp-11.6",
    "regexpComp-11.8",
    "regexpComp-15.6",
    "regexpComp-16.4",
    "regexpComp-17.7",
    "regexpComp-18.10",
    "regexpComp-18.12",
    "regexpComp-18.7",
    "regexpComp-18.8",
    "regexpComp-18.9",
    "regexpComp-20.2",
    "regexpComp-21.10",
    "regexpComp-21.11",
    "regexpComp-21.6",
    "regexpComp-21.7",
    "regexpComp-6.4",
    "regexpComp-6.5",
    "regexpComp-6.9",
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
    interp.script_file = str(path)
    # Pre-provide tcltests so that tcltests.tcl (sourced by regexp.test)
    # returns early instead of conflicting with our test support commands.
    interp.eval("package provide tcltests 1.0")
    script = path.read_text()
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


class TestRegexpNative:
    """Run tmp/tcl9.0.3/tests/regexp.test through the VM."""

    def test_regexp(self) -> None:
        results = _run_test_file("regexp.test")
        _check_results(results, KNOWN_FAILURES_REGEXP, "regexp.test")


class TestRegexpCompNative:
    """Run tmp/tcl9.0.3/tests/regexpComp.test through the VM."""

    def test_regexpcomp(self) -> None:
        results = _run_test_file("regexpComp.test")
        _check_results(results, KNOWN_FAILURES_REGEXPCOMP, "regexpComp.test")
