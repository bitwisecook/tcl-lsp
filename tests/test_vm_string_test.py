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
from tooling.vm.commands import tcltest_cmds
from tooling.vm.commands.test_support_cmds import setup_test_support
from tooling.vm.interp import TclInterp

pytestmark = pytest.mark.slow

# Known failures

KNOWN_FAILURES_SPLIT: set[str] = {
    "split-1.1",
    "split-1.6",
    "split-2.1",
    "split-2.2",
}

KNOWN_FAILURES_FORMAT: set[str] = {
    "format-1.12",
    "format-1.13",
    "format-1.14",
    "format-1.15",
    "format-10.1",
    "format-10.2",
    "format-10.3",
    "format-10.5",
    "format-11.1",
    "format-11.10",
    "format-11.11",
    "format-11.12",
    "format-11.13",
    "format-11.14",
    "format-11.2",
    "format-11.3",
    "format-11.4",
    "format-11.5",
    "format-11.6",
    "format-11.7",
    "format-11.8",
    "format-11.9",
    "format-15.1",
    "format-15.4",
    "format-17.1",
    "format-17.6",
    "format-18.2",
    "format-19.4.1",
    "format-2.14",
    "format-2.15",
    "format-8.10",
    "format-8.11",
    "format-8.12",
    "format-8.13",
    "format-8.14",
    "format-8.15",
    "format-8.16",
    "format-8.17",
    "format-8.18",
    "format-8.19",
    "format-8.20",
    "format-8.23",
    "format-8.24",
    "format-8.25",
    "format-8.26",
    "format-8.27",
    "format-8.28",
    "format-8.9",
}

KNOWN_FAILURES_SUBST: set[str] = {
    # State-pollution failures (only visible when subst.test runs as
    # part of the full ``test_vm_*`` suite -- multiple .test files
    # before it leave VM state that breaks these cases; ``subst.test``
    # passes in isolation).  Listed alphabetically so additions are
    # easy to spot in diffs.
    "subst-1.2",
    "subst-10.6",
    "subst-11.6",
    "subst-12.1",
    "subst-12.2",
    "subst-12.3",
    "subst-12.4",
    "subst-12.5",
    "subst-13.1",
    "subst-13.2",
    "subst-3.1",
    "subst-4.3",
    "subst-5.10",
    "subst-5.5",
    "subst-5.6",
    "subst-5.7",
    "subst-5.8",
    "subst-5.9",
    "subst-7.1",
    "subst-7.2",
    "subst-7.3",
    "subst-7.7",
    "subst-8.6",
    "subst-8.9",
}

KNOWN_FAILURES_SCAN: set[str] = {
    "scan-1.1",
    "scan-1.2",
    "scan-1.3",
    "scan-1.4",
    "scan-1.5",
    "scan-1.6",
    "scan-1.7",
    "scan-1.8",
    "scan-2.1",
    "scan-2.2",
    "scan-3.1",
    "scan-3.10",
    "scan-3.11",
    "scan-3.12",
    "scan-3.13",
    "scan-3.2",
    "scan-3.3",
    "scan-3.5",
    "scan-3.6",
    "scan-3.7",
    "scan-3.8",
    "scan-3.9",
    "scan-4.10",
    "scan-4.11",
    "scan-4.12",
    "scan-4.13",
    "scan-4.14",
    "scan-4.15",
    "scan-4.17",
    "scan-4.19",
    "scan-4.22",
    "scan-4.23",
    "scan-4.24",
    "scan-4.25",
    "scan-4.26",
    "scan-4.27",
    "scan-4.29",
    "scan-4.30",
    "scan-4.38",
    "scan-4.39",
    "scan-4.40.3",
    "scan-4.41",
    "scan-4.42",
    "scan-4.8",
}

KNOWN_FAILURES_STRING: set[str] = {
    "string-1.1.0",
    "string-10.1.0",
    "string-10.10.0",
    "string-10.2.0",
    "string-10.20.0",
    "string-10.20.1.0",
    "string-10.3.0",
    "string-10.9.0",
    "string-11.23.0",
    "string-11.25.0",
    "string-11.29.0",
    "string-11.30.0",
    "string-11.32.0",
    "string-11.38.0",
    "string-11.39.0",
    "string-11.39.1.0",
    "string-11.39.2.0",
    "string-11.39.4.0",
    "string-11.39.5.0",
    "string-11.4.0",
    "string-11.42.0",
    "string-11.43.0",
    "string-11.46.0",
    "string-11.47.0",
    "string-11.49.0",
    "string-11.50.0",
    "string-12.12.0",
    "string-12.13.0",
    "string-12.19.0",
    "string-12.23.0",
    "string-12.24.0",
    "string-12.25.0",
    "string-14.1.0",
    "string-14.10.0",
    "string-14.2.0",
    "string-14.3.0",
    "string-15.2.0",
    "string-15.3.0",
    "string-15.7.0",
    "string-16.2.0",
    "string-16.3.0",
    "string-16.7.0",
    "string-17.2.0",
    "string-17.4.0",
    "string-17.9.0",
    "string-18.2.0",
    "string-2.20.0",
    "string-2.35.0",
    "string-2.36.0",
    "string-2.37.0",
    "string-2.7.0",
    "string-20.2.0",
    "string-21.1.0",
    "string-21.12.0",
    "string-21.2.0",
    "string-21.4.0",
    "string-21.5.0",
    "string-21.7.0",
    "string-21.8.0",
    "string-21.9.0",
    "string-22.1.0",
    "string-22.10.0",
    "string-22.2.0",
    "string-22.3.0",
    "string-22.7.0",
    "string-23.1.0",
    "string-23.2.0",
    "string-25.10.0",
    "string-25.11.0",
    "string-25.13.0",
    "string-25.14.0",
    "string-25.2.0",
    "string-25.3.0",
    "string-25.6.0",
    "string-25.7.0",
    "string-26.1.0",
    "string-26.10.0",
    "string-26.10.1.0",
    "string-26.2.0",
    "string-26.2.1.0",
    "string-26.3.0",
    "string-26.3.1.0",
    "string-26.3.2.0",
    "string-26.4.0",
    "string-26.5.0",
    "string-26.6.0",
    "string-26.7.0",
    "string-26.8.0",
    "string-26.9.0",
    "string-27.1.0",
    "string-27.10.0",
    "string-27.2.0",
    "string-27.3.0",
    "string-27.4.0",
    "string-27.5.0",
    "string-27.6.0",
    "string-27.7.0",
    "string-27.8.0",
    "string-27.9.0",
    "string-28.1.0",
    "string-28.10.0",
    "string-28.11.0",
    "string-28.12.0",
    "string-28.13.0",
    "string-28.2.0",
    "string-28.3.0",
    "string-28.4.0",
    "string-28.5.0",
    "string-28.6.0",
    "string-28.7.0",
    "string-28.8.0",
    "string-28.9.0",
    "string-29.4.0",
    "string-3.2.0",
    "string-3.28.0",
    "string-3.41.0",
    "string-3.43.0",
    "string-3.44.0",
    "string-3.6.0",
    "string-30.1.2.0",
    "string-31.1.0",
    "string-31.10.0",
    "string-31.11.0",
    "string-31.12.0",
    "string-31.13.0",
    "string-31.14.0",
    "string-31.15.0",
    "string-31.16.0",
    "string-31.17.0",
    "string-31.18.0",
    "string-31.19.0",
    "string-31.2.0",
    "string-31.20.0",
    "string-31.21.0",
    "string-31.22.0",
    "string-31.23.0",
    "string-31.24.0",
    "string-31.25.0",
    "string-31.26.0",
    "string-31.3.0",
    "string-31.4.0",
    "string-31.5.0",
    "string-31.6.0",
    "string-31.7.0",
    "string-31.8.0",
    "string-31.9.0",
    "string-32.1.0",
    "string-32.10.0",
    "string-32.10a.0",
    "string-32.11.0",
    "string-32.12.0",
    "string-32.13.0",
    "string-32.14.0",
    "string-32.15.0",
    "string-32.16.0",
    "string-32.17.0",
    "string-32.1a.0",
    "string-32.2.0",
    "string-32.3.0",
    "string-32.4.0",
    "string-32.5.0",
    "string-32.5a.0",
    "string-32.6.0",
    "string-32.7.0",
    "string-32.8.0",
    "string-32.9.0",
    "string-32.9a.0",
    "string-4.16.0",
    "string-4.22.0",
    "string-4.3.0",
    "string-4.5.0",
    "string-4.6.0",
    "string-4.8.0",
    "string-5.14.0",
    "string-5.15.0",
    "string-5.19.0",
    "string-5.20.0",
    "string-5.21.0",
    "string-5.4.0",
    "string-5.7.0",
    "string-5.8.0",
    "string-5.9.0",
    "string-6.1.0",
    "string-6.101.0",
    "string-6.102.0",
    "string-6.103.0",
    "string-6.104.0",
    "string-6.105.0",
    "string-6.105.1.0",
    "string-6.106.0",
    "string-6.109.0",
    "string-6.11.0",
    "string-6.114.0",
    "string-6.116.0",
    "string-6.117.0",
    "string-6.118.0",
    "string-6.119.0",
    "string-6.120.0",
    "string-6.121.1.0",
    "string-6.122.0",
    "string-6.127.0",
    "string-6.129.0",
    "string-6.13.0",
    "string-6.130.1.0",
    "string-6.131.0",
    "string-6.139.0",
    "string-6.16.0",
    "string-6.19.0",
    "string-6.2.0",
    "string-6.22.0",
    "string-6.23.0",
    "string-6.25.0",
    "string-6.26.0",
    "string-6.3.0",
    "string-6.33.0",
    "string-6.34.0",
    "string-6.35.0",
    "string-6.36.0",
    "string-6.37.0",
    "string-6.39.0",
    "string-6.4.0",
    "string-6.42.0",
    "string-6.45.0",
    "string-6.46.0",
    "string-6.47.0",
    "string-6.48.0",
    "string-6.49.0",
    "string-6.5.0",
    "string-6.51.0",
    "string-6.52.0",
    "string-6.53.0",
    "string-6.54.0",
    "string-6.55.0",
    "string-6.56.0",
    "string-6.57.0",
    "string-6.58.0",
    "string-6.58.1.0",
    "string-6.59.0",
    "string-6.6.0",
    "string-6.62.0",
    "string-6.63.0",
    "string-6.64.0",
    "string-6.66.0",
    "string-6.68.0",
    "string-6.69.0",
    "string-6.72.0",
    "string-6.73.0",
    "string-6.74.0",
    "string-6.77.0",
    "string-6.78.0",
    "string-6.79.0",
    "string-6.8.0",
    "string-6.82.0",
    "string-6.83.0",
    "string-6.84.0",
    "string-6.86.0",
    "string-6.87.0",
    "string-6.88.0",
    "string-6.89.0",
    "string-6.90.0",
    "string-6.92.0",
    "string-6.93.0",
    "string-6.94.0",
    "string-6.99.0",
    "string-7.10.0",
    "string-7.3.0",
    "string-7.4.0",
    "string-7.6.0",
    "string-7.7.0",
    "string-7.8.0",
    "string-7.9.0",
    "string-9.4.0",
    "string-9.5.0",
    "string-9.7.0",
    "stringComp-14.21.0",
    "stringComp-14.23.0",
    "stringComp-14.26.0",
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
