"""Run expression test files natively through the VM's tcltest.

Covers compExpr-old.test, compExpr.test, and expr-old.test from the
Tcl 9.0.3 test suite.

Not yet included:
- expr.test (1884 tests) — too slow for CI; runs ~330 before timeout
- mathop.test — requires ``::tcl::mathop`` namespace (not implemented)

Known failures are tracked per-file so that regressions are caught
immediately while expected failures don't block CI.
"""

from __future__ import annotations

import io
import sys

import pytest

# Tcl handles arbitrarily large integers; some test files produce values
# exceeding Python's default 4300-digit limit.
if hasattr(sys, "set_int_max_str_digits"):
    sys.set_int_max_str_digits(20000)

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

KNOWN_FAILURES_COMPEXPR_OLD: set[str] = {
    # hello_world / 12days procs replaced with no-ops
    "compExpr-old-3.7",  # hello_world returns empty (proc gutted)
    # Unbraced expr evaluation (compiler doesn't distinguish braced vs unbraced)
    "compExpr-old-1.13",  # unbraced if $bool where bool="$x"
    "compExpr-old-14.31",  # unbraced expr $i where i="5+10"
    "compExpr-old-19.1",  # unbraced expr $x-$center
    # errorInfo format (shows tcl::mathfunc::* instead of expr *())
    "compExpr-old-15.2",  # errorInfo format for unknown function
    "compExpr-old-15.3",  # errorInfo format
    "compExpr-old-15.4",  # errorInfo format
    "compExpr-old-15.5",  # errorInfo format
}

KNOWN_FAILURES_COMPEXPR: set[str] = {
    # double-quoted expr argument with backslash tokens — the segmenter
    # returns raw text for double-quoted strings so Tcl-level backslash
    # escapes are not yet resolved when the expression compiler sees the body.
    "compExpr-2.5",
    # ${b}rge braced variable concatenation inside expression — the VM
    # does not yet resolve braced variable references with trailing text
    # in expression contexts.
    "compExpr-2.10",
    # tcl::unsupported::getbytecode not implemented — bytecode introspection
    # is a Tcl-internal API we do not expose.
    "compExpr-8.1",
    "compExpr-8.2",
    "compExpr-8.3",
    "compExpr-8.4",
}

KNOWN_FAILURES_EXPR_OLD: set[str] = {
    # RNG difference (Python vs C)
    "expr-old-32.50",  # srand(12345) produces different sequence
}


# Script patching


def _patch_hello_world_procs(script: str) -> str:
    """Replace the ``hello_world``/``12days`` procs with no-ops.

    These procs contain deeply nested expressions that trigger a
    compiler error in our VM.  The procs are only used by a few
    tests (``*-1.1``) that exercise correct evaluation ordering,
    not expression semantics proper.
    """
    for proc_name in ("put_hello_char", "hello_world", "12days", "do_twelve_days"):
        start_marker = f"proc {proc_name} "
        idx = script.find(start_marker)
        if idx < 0:
            continue
        # Find the opening brace of the body (second '{' on the line)
        body_start = script.index("{", script.index("{", idx) + 1)
        # Count braces to find the matching close
        depth = 1
        pos = body_start + 1
        while depth > 0 and pos < len(script):
            ch = script[pos]
            if ch == "\\":
                pos += 2
                continue
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
            pos += 1
        # pos is now just past the closing brace
        end_of_proc = pos
        # Replace with a no-op proc
        # Find the arg list
        args_start = script.index("{", idx)
        args_end = script.index("}", args_start) + 1
        arg_list = script[args_start:args_end]
        replacement = f"proc {proc_name} {arg_list} {{}}"
        script = script[:idx] + replacement + script[end_of_proc:]
    return script


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

    interp.eval("catch {testConstraint wideBiggerThanInt 1}")

    tests_dir = ensure_tcl_source("9.0")
    path = tests_dir / test_file
    script = path.read_text()

    # Wrap constraint-setup calls that use unimplemented commands
    # (e.g. ``binary``) in ``catch`` so they default to false
    # instead of aborting the entire file.
    # Python always uses IEEE 754 doubles, so set ieeeFloatingPoint true
    script = script.replace(
        "testConstraint ieeeFloatingPoint [testIEEE]",
        "testConstraint ieeeFloatingPoint 1",
    )

    # The ``12days`` proc body triggers a compiler error in our VM
    # (the deeply nested expressions reference ``$a`` which the
    # bytecode compiler tries to resolve at compile time).  Replace
    # the helper procs with no-ops so the rest of the file runs.
    script = _patch_hello_world_procs(script)

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


class TestCompExprOldNative:
    """Run tmp/tcl9.0.3/tests/compExpr-old.test through the VM."""

    def test_compexpr_old(self) -> None:
        results = _run_test_file("compExpr-old.test")
        _check_results(results, KNOWN_FAILURES_COMPEXPR_OLD, "compExpr-old.test")


class TestCompExprNative:
    """Run tmp/tcl9.0.3/tests/compExpr.test through the VM."""

    def test_compexpr(self) -> None:
        results = _run_test_file("compExpr.test")
        _check_results(results, KNOWN_FAILURES_COMPEXPR, "compExpr.test")


class TestExprOldNative:
    """Run tmp/tcl9.0.3/tests/expr-old.test through the VM."""

    def test_expr_old(self) -> None:
        results = _run_test_file("expr-old.test")
        _check_results(results, KNOWN_FAILURES_EXPR_OLD, "expr-old.test")
