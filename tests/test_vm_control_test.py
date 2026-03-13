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
from vm.commands import tcltest_cmds
from vm.commands.test_support_cmds import setup_test_support
from vm.interp import TclInterp

pytestmark = pytest.mark.slow

# Known failures
#
# Each set lists Tcl test names that are expected to fail in our VM.
# When a VM bug is fixed the test will unexpectedly pass — the set
# must be updated (removing the entry) to keep CI green.

KNOWN_FAILURES_FOR_OLD: set[str] = {
    # for wrong-args: compiled for dispatches args differently
    "for-old-1.7",  # "invalid command name" instead of wrong # args
    # Brace-quoting in for body not handled
    "for-old-1.8",  # invalid command name "}" — braced body not parsed
}

KNOWN_FAILURES_WHILE_OLD: set[str] = {
    # Expression evaluation: quoted expression not stripped
    "while-old-1.5",  # 'expected boolean value but got ""0 < 3"'
    # while wrong-args: compiled while dispatches differently
    "while-old-4.3",  # "invalid command name" instead of wrong # args
}

KNOWN_FAILURES_IF_OLD: set[str] = {
    # Unbraced expression: if treats arg as expression, not command
    "if-old-2.5",  # syntax error: "set a 2" treated as expression
    "if-old-2.8",  # syntax error: "set a 4" treated as expression
    # Error message format differences
    "if-old-4.1",  # wrong # args format differs from Tcl
    "if-old-4.3",  # missing argument name in error message
    "if-old-4.4",  # missing argument name in error message
    # Parsing: elseif not recognised as keyword in some positions
    "if-old-4.7",  # returns "0 {}" instead of elseif error
    "if-old-4.8",  # syntax error instead of "invalid command name"
    "if-old-4.10",  # syntax error instead of "invalid command name"
}

KNOWN_FAILURES_FOREACH: set[str] = {
    # List parsing: braced element followed by non-space
    "foreach-1.12",  # "invalid command name" vs list parsing error
    # errorInfo: missing "(setting foreach loop variable)" frame
    "foreach-1.14",
    # Brace-quoting in foreach value list
    "foreach-2.8",  # "invalid command name }" — braced body not parsed
    # lsort-based output vs raw iteration
    "foreach-3.1",  # dict ordering differs from expected sorted output
    # break/continue: extra iterations or missing error
    "foreach-5.4",  # continue count: 4 instead of 1
    "foreach-5.5",  # missing "wrong # args" for continue
    "foreach-6.3",  # break count: 3 instead of 1
    "foreach-6.4",  # missing "wrong # args" for break
    # dict for: key "line" not known
    "foreach-9.2",
}

KNOWN_FAILURES_SWITCH: set[str] = {
    # Duplicate match mode detection
    "switch-3.15",
    "switch-3.16",
    "switch-3.17",
    "switch-3.18",
    # errorInfo: missing arm context frame
    "switch-4.1",  # missing '("a" arm line 1)' frame
    "switch-4.5",  # missing '("default" arm line 1)' frame
    # Error message format
    "switch-4.2",  # wrong # args text differs
}

KNOWN_FAILURES_APPEND: set[str] = {
    # append to undefined var should error when strict
    "append-3.3",  # no error for reading undefined var
    # Unicode / surrogate handling
    "append-3.5",  # surrogate encoding difference
    "append-3.6",  # surrogate encoding difference
    # lappend list quoting: brace escaping
    "append-4.7",
    "append-4.13",
    "append-4.14",  # extra space in result
    "append-4.15",  # backslash-space vs braced space
    "append-4.16",  # extra space in result
    "append-4.18",  # empty vs "{}"
    # lappend: unmatched brace/quote validation
    "append-4.9",
    "append-4.10",
    "append-4.11",
    "append-4.12",
    "append-4.21",
    "append-4.22",
    "append-10.2",
    "append-10.4",
    # String length: append doubles length
    "append-5.1",  # "length mismatch: should have been 300, was 600"
    # Trace handling
    "append-7.1",  # trace on undefined var
    "append-7.5",  # trace count
    # lappend result quoting
    "append-9.0",  # "{new value}" vs "new value"
    "append-9.1",  # "{new value}" vs "new value"
}

KNOWN_FAILURES_EVAL: set[str] = {
    # errorInfo: missing "eval" body context frame
    "eval-2.5",  # missing '("eval" body line 3)' frame in stack trace
}

KNOWN_FAILURES_FOR: set[str] = {
    # Wrong # args error message format
    "for-1.2",  # compiled for dispatches args differently
    "for-1.4",  # wrong # args text differs
    "for-1.10",  # wrong # args text differs
    # Error message format: "invalid command name" vs wrong # args
    "for-2.1",  # missing "wrong # args" for bad for body
    # Braced body not parsed / execution
    "for-3.1",  # "invalid command name }" — braced body not parsed
    # errorInfo: missing "(\"for\" body line N)" frame
    "for-6.6",  # errorInfo format differs
    "for-6.7",  # errorInfo format differs
    "for-6.9",  # errorInfo format differs
    "for-6.13",  # errorInfo format differs
    "for-6.17",  # errorInfo format differs
    "for-6.18",  # errorInfo format differs
}

KNOWN_FAILURES_IF: set[str] = {
    # Wrong # args error message format
    "if-1.1",  # generic "wrong # args" instead of 'no expression after "if"'
    "if-5.1",  # generic "wrong # args" instead of 'no expression after "if"'
    # Unbraced expression / body not parsed
    "if-1.13",  # "invalid command name" — unbraced body not parsed
    # Expression error: "expected boolean … got a list"
    "if-1.17",  # 'got "0 < 3"' instead of "got a list"
    "if-5.17",  # 'got "0 < 3"' instead of "got a list"
    # errorInfo: missing braces around body in error context
    "if-1.3",  # errorInfo format: missing braces around expression
    "if-2.4",  # errorInfo format: missing braces around expression
    "if-5.3",  # errorInfo format: missing braces in $z expansion
    "if-6.4",  # errorInfo format: missing braces in $z expansion
    # Wrong # args: "no script following then"
    "if-1.9",  # 'no script following expression' vs 'no script following "then"'
    "if-5.9",  # 'no script following expression' vs 'no script following "then"'
    # if-10.x: compiled if tracing / conditional evaluation
    "if-10.1",  # "01" instead of "00"
    "if-10.2",  # "0badelseif" instead of "00"
    "if-10.3",  # "01" instead of "00"
    "if-10.4",  # "1" instead of "0"
    "if-10.5",  # error instead of "0 ok"
    "if-10.6",  # missing trace variable handling
    # Missing validation: extra words after "else" clause
    "if-2.2",  # no error for extra words after else
    "if-3.2",  # no error for extra words after else
    "if-6.2",  # no error for extra words after else
    "if-7.2",  # no error for extra words after else
    # Missing validation: no expression after "elseif"
    "if-2.3",  # no error for missing elseif expression
    "if-6.3",  # no error for missing elseif expression
    # errorInfo: missing "invoked from within" context frame
    "if-5.10",  # missing '("if" ... body line N)' frame
    "if-7.4",  # missing 'invoked from within' frame
}

KNOWN_FAILURES_WHILE: set[str] = {
    # Unbraced expression: missing braces in errorInfo context
    "while-1.2",  # errorInfo format: missing braces around expression
    "while-4.3",  # errorInfo format: missing braces in $z expansion
    # String quoting: body not parsed correctly
    "while-1.10",  # 'missing "' — unbraced body not parsed
    # errorInfo: missing "(\"while\" body line N)" frame
    "while-4.9",  # missing 'invoked from within' frame with body context
}

KNOWN_FAILURES_SOURCE: set[str] = {
    # File I/O: source command not reading files
    "source-2.3",  # errorInfo format for source file errors
    "source-2.6",  # encoding option not supported
    "source-2.7",  # encoding option not supported
    # Error propagation
    "source-3.3",  # errorInfo format
    "source-3.4",  # errorInfo format
    "source-3.5",  # errorInfo format
    # Return code handling
    "source-4.1",  # return code from sourced file
    # File handling
    "source-6.2",  # non-existent file error format
    # Line tracking
    "source-7.3",  # info frame/source line tracking
    "source-7.4",  # info frame/source line tracking
    "source-7.6",  # info frame/source line tracking
    # $::errorInfo propagation
    "source-8.1",  # errorInfo on source failure
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
) -> None:
    """Assert that failures are exactly the known set.

    - Unexpected failures (not in known set) -> test fails
    - Unexpected passes (in known set but passed) -> test fails
      (forces cleanup of the known-failure set when bugs are fixed)
    """
    failed_set = set(results["failed_tests"])  # type: ignore[arg-type]
    total = results["Total"]
    passed = results["Passed"]
    skipped = results["Skipped"]

    # Print summary
    print(
        f"\n{test_file}: {total} total, {passed} passed, "
        f"{skipped} skipped, {len(failed_set)} failed"
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
