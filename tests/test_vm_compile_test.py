"""Run compilation test files natively through the VM's tcltest.

Covers compile.test and execute.test from the Tcl 9.0.3 test suite
(Phase 14 of the VM test conformance plan).

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

KNOWN_FAILURES_COMPILE: set[str] = {
    # Compiled body parsing (braced body not fully parsed)
    "compile-3.1",  # missing close-brace in compiled script
    "compile-3.3",  # missing close-brace in proc body
    "compile-3.6",  # unexpected code after close-brace
    "compile-3.7",  # unexpected code after close-brace
    "compile-4.1",  # invalid command name "}" (compiled for)
    "compile-5.2",  # invalid command name "}" (compiled while)
    "compile-7.1",  # missing close-bracket
    # Error message format differences
    "compile-10.1",  # error in proc definition
    "compile-11.1",  # wrong error message format
    "compile-11.2",  # wrong error message format
    "compile-11.7",  # wrong error message format
    "compile-13.2",  # errorInfo format differs
    "compile-13.3",  # errorInfo format differs
    "compile-14.1",  # compiled expression error
    "compile-14.2",  # compiled expression error
    # Bytecode-level semantics
    "compile-16.10.0",  # compiled dispatch differences
    "compile-16.11.0",  # compiled dispatch differences
    "compile-16.23.0",  # compiled dispatch differences
    "compile-16.25.0",  # compiled dispatch differences
    "compile-17.2",  # namespace compilation semantics
    # Disassembler (tcl::unsupported::disassemble)
    "compile-18.1",
    "compile-18.2",
    "compile-18.3",
    "compile-18.4",
    "compile-18.5",
    "compile-18.6",
    "compile-18.7",
    "compile-18.8",
    "compile-18.9",
    "compile-18.10",
    "compile-18.11",
    "compile-18.12",
    "compile-18.13",
    "compile-18.14",
    "compile-18.15",
    "compile-18.16",
    "compile-18.17",
    "compile-18.18",
    "compile-18.19",
    "compile-18.21",
    "compile-18.22",
    "compile-18.23",
    "compile-18.24",
    "compile-18.25",
    "compile-18.26",
    "compile-18.27",
    "compile-18.28",
    "compile-18.28.1",
    "compile-18.28.2",
    "compile-18.28.3",
    "compile-18.28.4",
    "compile-18.29",
    "compile-18.30",
    "compile-18.31",
    "compile-18.32",
    "compile-18.33",
    "compile-18.34",
    "compile-18.35",
    "compile-18.36",
    "compile-18.37",
    "compile-18.38",
    "compile-18.39",
    "compile-18.40",
    "compile-18.41",
    "compile-18.42",
    "compile-18.43",
    "compile-18.44",
    "compile-18.45",
    "compile-18.46",
    "compile-18.47",
    "compile-18.48",
    "compile-18.50",
    "compile-18.51",
    "compile-18.52",
    "compile-18.53",
    "compile-18.54",
    "compile-18.55",
    "compile-18.56",
    "compile-18.57",
    "compile-18.58",
    # Coroutine compilation
    "compile-20.1",
    "compile-20.2",
}

KNOWN_FAILURES_EXECUTE: set[str] = {
    # Error message format differences
    "execute-1.2",  # wrong # args error format
    "execute-2.1",  # wrong # args error format
    "execute-2.2",  # wrong # args error format
    # Numeric type handling
    "execute-4.1",  # integer type coercion
    "execute-4.2",  # integer type coercion
    # Expression evaluation edge cases
    "execute-6.2",  # expression evaluation
    "execute-6.3",  # expression evaluation
    "execute-6.4",  # expression evaluation
    "execute-6.5",  # expression evaluation
    "execute-6.6",  # expression evaluation
    "execute-6.7",  # expression evaluation
    "execute-6.8",  # expression evaluation
    "execute-6.9",  # expression evaluation
    "execute-6.11",  # expression evaluation
    "execute-6.13",  # expression evaluation
    "execute-6.14",  # expression evaluation
    "execute-6.15",  # expression evaluation
    "execute-6.16",  # expression evaluation
    "execute-6.17",  # expression evaluation
    "execute-6.18",  # expression evaluation
    # Wide integer / overflow
    "execute-7.8",  # hex overflow handling
    "execute-7.14",  # wide integer overflow
    "execute-7.15",  # wide integer overflow
    "execute-7.16",  # wide integer multiplication overflow
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


class TestCompileNative:
    """Run tmp/tcl9.0.3/tests/compile.test through the VM."""

    def test_compile(self) -> None:
        results = _run_test_file("compile.test")
        _check_results(results, KNOWN_FAILURES_COMPILE, "compile.test")


class TestExecuteNative:
    """Run tmp/tcl9.0.3/tests/execute.test through the VM."""

    def test_execute(self) -> None:
        results = _run_test_file("execute.test")
        _check_results(results, KNOWN_FAILURES_EXECUTE, "execute.test")
