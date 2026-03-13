"""Run interp test files natively through the VM's tcltest.

Covers interp.test from the Tcl 9.0.3 test suite
(Phase 16 of the VM test conformance plan).

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

KNOWN_FAILURES_INTERP: set[str] = {
    # interp subcommands: child interp management, eval, alias, etc.
    # The test file reaches 354 tests; 271 fail due to missing child interp,
    # safe interp, hidden command, and interp limit/recursion features.
    "interp-1.10",
    "interp-1.2",
    "interp-1.4",
    "interp-1.5",
    "interp-1.6",
    "interp-1.7",
    "interp-1.8",
    "interp-1.9",
    "interp-2.11",
    "interp-2.12",
    "interp-2.4",
    "interp-2.7",
    "interp-3.1",
    "interp-3.4",
    "interp-3.5",
    "interp-3.6",
    "interp-3.7",
    "interp-3.8",
    "interp-4.2",
    "interp-4.3",
    "interp-4.5",
    "interp-4.7",
    "interp-4.8",
    "interp-5.1",
    "interp-6.5",
    "interp-7.5",
    "interp-7.6",
    "interp-8.3",
    "interp-9.3",
    "interp-9.4",
    "interp-10.4",
    "interp-10.5",
    "interp-11.1",
    "interp-11.2",
    "interp-11.3",
    "interp-11.4",
    "interp-11.5",
    "interp-11.6",
    "interp-11.7",
    "interp-12.3",
    "interp-12.4",
    "interp-13.2",
    "interp-13.3",
    "interp-13.4",
    "interp-14.1",
    "interp-14.10",
    "interp-14.2",
    "interp-14.3",
    "interp-14.4",
    "interp-14.5",
    "interp-14.6",
    "interp-14.7",
    "interp-14.8",
    "interp-14.9",
    "interp-15.1",
    "interp-15.2",
    "interp-15.3",
    "interp-15.4",
    "interp-15.5",
    "interp-15.6",
    "interp-15.7",
    "interp-15.8",
    "interp-16.2",
    "interp-16.4",
    "interp-16.5",
    "interp-17.1",
    "interp-17.2",
    "interp-17.3",
    "interp-17.4",
    "interp-17.5",
    "interp-17.6",
    "interp-18.10",
    "interp-18.7",
    "interp-18.8",
    "interp-18.9",
    "interp-19.2",
    "interp-19.6",
    "interp-19.7",
    "interp-19.8",
    "interp-20.11",
    "interp-20.12",
    "interp-20.13",
    "interp-20.14",
    "interp-20.15",
    "interp-20.16",
    "interp-20.17",
    "interp-20.18",
    "interp-20.19",
    "interp-20.2",
    "interp-20.20",
    "interp-20.21",
    "interp-20.22",
    "interp-20.23",
    "interp-20.24",
    "interp-20.25",
    "interp-20.26",
    "interp-20.27",
    "interp-20.28",
    "interp-20.29",
    "interp-20.3",
    "interp-20.30",
    "interp-20.31",
    "interp-20.32",
    "interp-20.33",
    "interp-20.34",
    "interp-20.35",
    "interp-20.36",
    "interp-20.37",
    "interp-20.38",
    "interp-20.39",
    "interp-20.4",
    "interp-20.40",
    "interp-20.41",
    "interp-20.42",
    "interp-20.43",
    "interp-20.44",
    "interp-20.45",
    "interp-20.48",
    "interp-20.49",
    "interp-20.5",
    "interp-20.50.1",
    "interp-20.6",
    "interp-20.7",
    "interp-20.8",
    "interp-20.9",
    "interp-21.1",
    "interp-21.2",
    "interp-21.3",
    "interp-21.4",
    "interp-21.5",
    "interp-21.6",
    "interp-21.7",
    "interp-21.8",
    "interp-21.9",
    "interp-22.1",
    "interp-22.2",
    "interp-22.3",
    "interp-22.4",
    "interp-22.5",
    "interp-22.6",
    "interp-22.7",
    "interp-22.8",
    "interp-22.9",
    "interp-23.1",
    "interp-24.10",
    "interp-24.11",
    "interp-24.12",
    "interp-24.3",
    "interp-24.4",
    "interp-24.9",
    "interp-25.2",
    "interp-26.1",
    "interp-26.2",
    "interp-26.3",
    "interp-26.5",
    "interp-26.7",
    "interp-26.9",
    "interp-27.1",
    "interp-27.2",
    "interp-27.3",
    "interp-27.4",
    "interp-29.1.1",
    "interp-29.1.10",
    "interp-29.1.11",
    "interp-29.1.12",
    "interp-29.1.2",
    "interp-29.1.3",
    "interp-29.1.4",
    "interp-29.1.5",
    "interp-29.1.6",
    "interp-29.1.7",
    "interp-29.1.8",
    "interp-29.1.9",
    "interp-29.2.1",
    "interp-29.2.2",
    "interp-29.2.3",
    "interp-29.2.4",
    "interp-29.2.5",
    "interp-29.2.6",
    "interp-29.2.7",
    "interp-29.2.8",
    "interp-29.3.1",
    "interp-29.3.10a",
    "interp-29.3.10b",
    "interp-29.3.11a",
    "interp-29.3.11b",
    "interp-29.3.12a",
    "interp-29.3.12b",
    "interp-29.3.2",
    "interp-29.3.3",
    "interp-29.3.4",
    "interp-29.3.5",
    "interp-29.3.6",
    "interp-29.3.7a",
    "interp-29.3.7b",
    "interp-29.3.7c",
    "interp-29.3.8a",
    "interp-29.3.8b",
    "interp-29.3.9a",
    "interp-29.3.9b",
    "interp-29.4.1",
    "interp-29.4.2",
    "interp-29.5.1",
    "interp-29.5.2",
    "interp-29.5.3",
    "interp-29.5.4",
    "interp-29.6.1",
    "interp-29.6.10",
    "interp-29.6.2",
    "interp-29.6.3",
    "interp-29.6.4",
    "interp-29.6.5",
    "interp-29.6.6",
    "interp-29.6.7",
    "interp-29.6.8",
    "interp-29.6.9",
    "interp-30.1",
    "interp-33.1",
    "interp-34.1",
    "interp-34.10",
    "interp-34.12",
    "interp-34.13",
    "interp-34.14",
    "interp-34.2",
    "interp-34.3",
    "interp-34.3.1",
    "interp-34.4",
    "interp-34.5",
    "interp-34.6",
    "interp-34.7",
    "interp-34.8",
    "interp-34.9",
    "interp-35.1",
    "interp-35.10",
    "interp-35.11",
    "interp-35.12",
    "interp-35.13",
    "interp-35.14",
    "interp-35.15",
    "interp-35.16",
    "interp-35.17",
    "interp-35.18",
    "interp-35.19",
    "interp-35.2",
    "interp-35.20",
    "interp-35.21",
    "interp-35.22",
    "interp-35.23",
    "interp-35.24",
    "interp-35.3",
    "interp-35.4",
    "interp-35.5",
    "interp-35.6",
    "interp-35.7",
    "interp-35.8",
    "interp-35.9",
    "interp-36.2",
    "interp-36.3",
    "interp-36.4",
    "interp-36.5",
    "interp-36.6",
    "interp-36.7",
    "interp-37.1",
    "interp-38.1",
    "interp-38.2",
    "interp-38.3",
    "interp-38.4",
    "interp-38.5",
    "interp-38.6",
    "interp-38.7",
    "interp-38.8",
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


class TestInterpNative:
    """Run tmp/tcl9.0.3/tests/interp.test through the VM."""

    def test_interp(self) -> None:
        results = _run_test_file("interp.test")
        _check_results(results, KNOWN_FAILURES_INTERP, "interp.test")
