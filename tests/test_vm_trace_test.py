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

"""Run trace and encoding test files natively through the VM's tcltest.

Covers encoding.test from the Tcl 9.0.3 test suite (Phase 15 of the
VM test conformance plan).

trace.test is deferred — it hangs after 3 tests due to trace callback
recursion or blocking operations (vwait) that our VM doesn't handle.

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

# trace.test is deferred — hangs after 3 tests (trace callback
# recursion or vwait).  Not wired into test runner.

KNOWN_FAILURES_ENCODING: set[str] = {
    "encoding-1.3",
    "encoding-10.1",
    "encoding-11.10",
    "encoding-11.11",
    "encoding-11.2",
    "encoding-11.3",
    "encoding-11.4",
    "encoding-11.5",
    "encoding-11.5.1",
    "encoding-11.8",
    "encoding-11.9",
    "encoding-12.1",
    "encoding-12.2",
    "encoding-12.3",
    "encoding-12.4",
    "encoding-12.5",
    "encoding-12.7",
    "encoding-12.8",
    "encoding-13.1",
    "encoding-15.1",
    "encoding-15.10",
    "encoding-15.11",
    "encoding-15.12",
    "encoding-15.13",
    "encoding-15.14",
    "encoding-15.15",
    "encoding-15.17",
    "encoding-15.18",
    "encoding-15.19",
    "encoding-15.20",
    "encoding-15.21",
    "encoding-15.22",
    "encoding-15.23",
    "encoding-15.24",
    "encoding-15.26",
    "encoding-15.28",
    "encoding-15.31",
    "encoding-15.32",
    "encoding-15.33",
    "encoding-15.4",
    "encoding-15.5",
    "encoding-15.6",
    "encoding-15.7",
    "encoding-15.8",
    "encoding-15.9",
    "encoding-16.1",
    "encoding-16.10",
    "encoding-16.11",
    "encoding-16.12",
    "encoding-16.13",
    "encoding-16.14",
    "encoding-16.15",
    "encoding-16.16",
    "encoding-16.17",
    "encoding-16.18",
    "encoding-16.19.strict",
    "encoding-16.19.tcl8",
    "encoding-16.2",
    "encoding-16.20.strict",
    "encoding-16.20.tcl8",
    "encoding-16.21.strict",
    "encoding-16.21.tcl8",
    "encoding-16.22",
    "encoding-16.23",
    "encoding-16.24",
    "encoding-16.25.strict",
    "encoding-16.25.tcl8",
    "encoding-16.3",
    "encoding-16.4",
    "encoding-16.5",
    "encoding-16.6",
    "encoding-16.7",
    "encoding-16.8",
    "encoding-16.9",
    "encoding-17.1",
    "encoding-17.10",
    "encoding-17.11",
    "encoding-17.12",
    "encoding-17.2",
    "encoding-17.3",
    "encoding-17.4",
    "encoding-17.5",
    "encoding-17.6",
    "encoding-17.7",
    "encoding-17.8",
    "encoding-17.9",
    "encoding-18.1",
    "encoding-18.2",
    "encoding-18.3",
    "encoding-18.4",
    "encoding-18.5",
    "encoding-18.6",
    "encoding-19.3",
    "encoding-19.4",
    "encoding-19.5",
    "encoding-19.6",
    "encoding-2.1",
    "encoding-28.0",
    "encoding-3.1",
    "encoding-3.2",
    "encoding-3.3",
    "encoding-5.1",
    "encoding-7.1",
    "encoding-7.2",
    "encoding-8.1",
    "encoding-9.1",
    "encoding-9.2",
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
    # Pre-provide tcltests so that tcltests.tcl (sourced by encoding.test)
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
                    print(output[start:end].encode("utf-8", "replace").decode("utf-8"))
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


class TestEncodingNative:
    """Run tmp/tcl9.0.3/tests/encoding.test through the VM."""

    def test_encoding(self) -> None:
        results = _run_test_file("encoding.test")
        _check_results(results, KNOWN_FAILURES_ENCODING, "encoding.test")
