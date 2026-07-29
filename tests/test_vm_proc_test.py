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

"""Run procedure-related test files natively through the VM's tcltest.

Covers proc-old.test, rename.test, unknown.test, proc.test, and
apply.test from the Tcl 9.0.3 test suite (Phases 5a–5e of the VM
test conformance plan).

Reference results (tclsh 9.0):
  proc-old.test  — see KNOWN_FAILURES_PROC_OLD
  rename.test    — see KNOWN_FAILURES_RENAME
  unknown.test   — see KNOWN_FAILURES_UNKNOWN
  proc.test      — see KNOWN_FAILURES_PROC
  apply.test     — 38P/4S/0F

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

KNOWN_FAILURES_PROC_OLD: set[str] = {
    "proc-old-3.7",
    "proc-old-3.9",
    "proc-old-5.11",
    "proc-old-5.16",
    "proc-old-7.11",
    "proc-old-7.12",
    "proc-old-7.13",
    "proc-old-7.14",
    "proc-old-7.6",
}

KNOWN_FAILURES_RENAME: set[str] = {
    "rename-2.1",
    "rename-3.1",
    "rename-3.2",
}

KNOWN_FAILURES_UNKNOWN: set[str] = set()

KNOWN_FAILURES_PROC: set[str] = {
    "proc-1.2",
    "proc-1.3",
    "proc-1.6",
    "proc-2.3",
    "proc-3.3",
    "proc-3.4",
    "proc-3.6",
    "proc-3.7",
}

KNOWN_FAILURES_APPLY: set[str] = {
    "apply-2.1",
    "apply-2.2",
    "apply-2.3",
    "apply-2.4",
    "apply-2.5",
    "apply-3.1",
    "apply-3.2",
    "apply-3.3",
    "apply-3.4",
    "apply-4.1",
    "apply-4.2",
    "apply-4.3",
    "apply-4.4",
    "apply-4.5",
    "apply-5.1",
    "apply-6.2",
    "apply-6.3",
    "apply-7.2",
    "apply-7.3",
    "apply-7.4",
    "apply-7.6",
    "apply-7.7",
    "apply-7.8",
    "apply-8.10",
    "apply-8.2",
    "apply-8.3",
    "apply-8.9",
}


# Test runner


def _run_test_file(
    test_file: str,
    *,
    pre_script: str | None = None,
) -> dict[str, object]:
    """Source a .test file through the VM and return results.

    Returns a dict with keys: Total, Passed, Skipped, Failed,
    failed_tests (list of test names), and output (captured stdout).

    *pre_script* is optional Tcl code evaluated before the test file
    (useful for registering commands or setting variables so that
    feature-guard checks at the top of a .test file pass).
    """
    interp = TclInterp(source_init=False)
    setup_test_support(interp)

    # Reset tcltest state for a clean run
    tcltest_cmds._reset_state()

    # Capture output so test failures are visible in pytest output
    buf = io.StringIO()
    interp.channels["stdout"] = buf

    if pre_script is not None:
        interp.eval(pre_script)

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


class TestProcOldNative:
    """Run tmp/tcl9.0.3/tests/proc-old.test through the VM."""

    def test_proc_old(self) -> None:
        results = _run_test_file("proc-old.test")
        _check_results(results, KNOWN_FAILURES_PROC_OLD, "proc-old.test")


class TestRenameNative:
    """Run tmp/tcl9.0.3/tests/rename.test through the VM."""

    def test_rename(self) -> None:
        results = _run_test_file("rename.test")
        _check_results(results, KNOWN_FAILURES_RENAME, "rename.test")


class TestUnknownNative:
    """Run tmp/tcl9.0.3/tests/unknown.test through the VM."""

    def test_unknown(self) -> None:
        results = _run_test_file("unknown.test")
        _check_results(results, KNOWN_FAILURES_UNKNOWN, "unknown.test")


class TestProcNative:
    """Run tmp/tcl9.0.3/tests/proc.test through the VM."""

    def test_proc(self) -> None:
        results = _run_test_file("proc.test")
        _check_results(results, KNOWN_FAILURES_PROC, "proc.test")


class TestApplyNative:
    """Run tmp/tcl9.0.3/tests/apply.test through the VM."""

    def test_apply(self) -> None:
        # apply.test guards with ``[info commands ::apply]``; the VM
        # registers the command without the ``::`` prefix, so we
        # create a namespace-qualified alias to let the guard pass.
        results = _run_test_file(
            "apply.test",
            pre_script="interp alias {} ::apply {} apply",
        )
        _check_results(results, KNOWN_FAILURES_APPLY, "apply.test")
