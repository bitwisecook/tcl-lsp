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
from vm.commands import tcltest_cmds
from vm.commands.test_support_cmds import setup_test_support
from vm.interp import TclInterp

# Known failures
#
# Each set lists Tcl test names that are expected to fail in our VM.
# When a VM bug is fixed the test will unexpectedly pass — the set
# must be updated (removing the entry) to keep CI green.

# trace.test is deferred — hangs after 3 tests (trace callback
# recursion or vwait).  Not wired into test runner.

KNOWN_FAILURES_ENCODING: set[str] = {
    # encoding convertfrom / convertto not implemented
    "encoding-1.3",
    "encoding-2.1",
    "encoding-3.1",
    "encoding-3.2",
    "encoding-3.3",
    "encoding-5.1",
    "encoding-7.1",
    "encoding-7.2",
    "encoding-8.1",
    "encoding-9.1",
    "encoding-9.2",
    "encoding-10.1",
    # encoding system / encoding names
    "encoding-11.2",
    "encoding-11.3",
    "encoding-11.4",
    "encoding-11.5",
    "encoding-11.5.1",
    "encoding-11.8",
    "encoding-11.9",
    "encoding-11.10",
    "encoding-11.11",
    # encoding profiles
    "encoding-12.1",
    "encoding-12.2",
    "encoding-12.3",
    "encoding-12.4",
    "encoding-12.5",
    "encoding-12.7",
    "encoding-12.8",
    "encoding-13.1",
    # UTF-8 encoding/decoding
    "encoding-15.1",
    "encoding-15.4",
    "encoding-15.5",
    "encoding-15.6",
    "encoding-15.7",
    "encoding-15.8",
    "encoding-15.9",
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
    # UTF-16 encoding/decoding
    "encoding-16.1",
    "encoding-16.2",
    "encoding-16.3",
    "encoding-16.4",
    "encoding-16.5",
    "encoding-16.6",
    "encoding-16.7",
    "encoding-16.8",
    "encoding-16.9",
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
    "encoding-16.20.strict",
    "encoding-16.20.tcl8",
    "encoding-16.21.strict",
    "encoding-16.21.tcl8",
    "encoding-16.22",
    "encoding-16.23",
    "encoding-16.24",
    "encoding-16.25.strict",
    "encoding-16.25.tcl8",
    # UTF-32 encoding/decoding
    "encoding-17.1",
    "encoding-17.2",
    "encoding-17.3",
    "encoding-17.4",
    "encoding-17.5",
    "encoding-17.6",
    "encoding-17.7",
    "encoding-17.8",
    "encoding-17.9",
    "encoding-17.10",
    "encoding-17.11",
    "encoding-17.12",
    # ISO / other encodings
    "encoding-18.1",
    "encoding-18.2",
    "encoding-18.3",
    "encoding-18.4",
    "encoding-18.5",
    "encoding-18.6",
    # encoding dirs
    "encoding-19.3",
    "encoding-19.4",
    "encoding-19.5",
    "encoding-19.6",
    # misc encoding edge cases
    "encoding-28.0",
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
