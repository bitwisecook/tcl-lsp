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
from vm.commands import tcltest_cmds
from vm.commands.test_support_cmds import setup_test_support
from vm.interp import TclInterp

# Known failures
#
# Each set lists Tcl test names that are expected to fail in our VM.
# When a VM bug is fixed the test will unexpectedly pass — the set
# must be updated (removing the entry) to keep CI green.

KNOWN_FAILURES_REGEXP: set[str] = {
    # List element quoting (octal escapes for non-ASCII)
    "regexp-2.9",  # fêtebbbbc → {f\352tebbbbc}
    "regexp-2.10",  # non-ASCII in match var
    # Python re error messages differ from Tcl ARE
    "regexp-6.4",  # missing ) → "parentheses () not balanced"
    "regexp-6.5",  # same
    "regexp-6.9",  # quantifier operand invalid
    "regexp-6.10",  # non-quantifiable operand
    # regsub wrong # args error message format
    "regexp-11.4",
    "regexp-11.5",
    "regexp-11.6",
    "regexp-11.8",
    # -all edge cases (zero-length anchor semantics)
    "regexp-15.6",  # -start with ^$ on empty string
    "regexp-15.9",  # -start double option
    "regexp-15.10",  # -start double option
    # -start with -all and \A anchor (Tcl ARE-specific)
    "regexp-16.4",  # \A re-anchored per -start position
    "regexp-16.7",  # -start with -all
    "regexp-16.8",  # -start with -all
    "regexp-16.20",  # -start double option
    "regexp-16.21",  # -start with -all and nested match
    "regexp-16.22",  # -start with -all and nested match
    # regsub list quoting
    "regexp-17.7",
    # regsub -inline not a valid regsub flag
    "regexp-18.7",  # should error on -inline with match vars
    "regexp-18.8",
    "regexp-18.9",
    "regexp-18.10",
    "regexp-18.12",
    # Tcl ARE lookbehind/lookahead differences
    "regexp-20.2",
    # CompileRegexp cache / internal state
    "regexp-22.4",
    "regexp-22.5",
    # -all -inline -indices with multi-line zero-length matches
    "regexp-23.2",
    "regexp-23.3",
    # evalInProc codegen bug (proc body from variable)
    "regexp-24.1",
    "regexp-24.2",
    "regexp-24.3",
    "regexp-24.4",
    "regexp-24.5",
    "regexp-24.6",
    "regexp-24.7",
    "regexp-24.8",
    "regexp-24.9",
    "regexp-24.10",
    # . vs \n with -line (list quoting of \n in match result)
    "regexp-25.1",
    # -all -inline subgroup handling edge cases
    "regexp-26.8",
    "regexp-26.9",
    "regexp-26.10",
    "regexp-26.11",
    "regexp-26.12",
    "regexp-26.13",
    # regsub -command edge case
    "regexp-27.8",
}

# regexpComp.test: 118 out of 142 tests fail because almost all
# use evalInProc { ... } which hits a pre-existing codegen bug
# (proc defined with variable body: `proc testProc {} $script`
# produces a fallback bytecode path that embeds the variable
# reference literally instead of resolving it).
KNOWN_FAILURES_REGEXPCOMP: set[str] = {
    # evalInProc codegen bug (proc body from variable)
    "regexpComp-1.1",
    "regexpComp-1.2",
    "regexpComp-1.3",
    "regexpComp-1.4",
    "regexpComp-1.5",
    "regexpComp-1.7",
    "regexpComp-1.8",
    "regexpComp-1.9",
    "regexpComp-1.10",
    "regexpComp-1.11",
    "regexpComp-1.12",
    "regexpComp-1.13",
    "regexpComp-1.14",
    "regexpComp-1.15",
    "regexpComp-1.16",
    "regexpComp-2.1",
    "regexpComp-2.2",
    "regexpComp-2.3",
    "regexpComp-2.4",
    "regexpComp-2.5",
    "regexpComp-2.6",
    "regexpComp-2.7",
    "regexpComp-2.8",
    "regexpComp-3.1",
    "regexpComp-3.2",
    "regexpComp-3.3",
    "regexpComp-3.4",
    "regexpComp-3.5",
    "regexpComp-3.6",
    "regexpComp-3.7",
    "regexpComp-4.1",
    "regexpComp-4.2",
    "regexpComp-4.3",
    "regexpComp-4.4",
    "regexpComp-5.1",
    "regexpComp-5.2",
    "regexpComp-5.3",
    "regexpComp-5.4",
    "regexpComp-5.5",
    "regexpComp-6.1",
    "regexpComp-6.2",
    "regexpComp-6.3",
    "regexpComp-6.4",
    "regexpComp-6.5",
    "regexpComp-6.6",
    "regexpComp-6.7",
    "regexpComp-6.8",
    "regexpComp-6.9",
    "regexpComp-7.1",
    "regexpComp-7.2",
    "regexpComp-7.3",
    "regexpComp-7.4",
    "regexpComp-7.5",
    "regexpComp-7.6",
    "regexpComp-7.7",
    "regexpComp-7.8",
    "regexpComp-7.9",
    "regexpComp-7.10",
    "regexpComp-7.11",
    "regexpComp-7.12",
    "regexpComp-7.13",
    "regexpComp-7.14",
    "regexpComp-7.15",
    "regexpComp-7.16",
    "regexpComp-7.17",
    "regexpComp-8.1",
    "regexpComp-8.2",
    "regexpComp-8.3",
    "regexpComp-8.4",
    "regexpComp-8.5",
    "regexpComp-8.6",
    "regexpComp-9.1",
    "regexpComp-9.2",
    "regexpComp-9.3",
    "regexpComp-9.4",
    "regexpComp-9.5",
    "regexpComp-9.6",
    "regexpComp-9.7",
    "regexpComp-10.1",
    "regexpComp-10.2",
    "regexpComp-10.3",
    "regexpComp-10.4",
    "regexpComp-10.5",
    "regexpComp-11.1",
    "regexpComp-11.2",
    "regexpComp-11.3",
    "regexpComp-11.4",
    "regexpComp-11.5",
    "regexpComp-11.6",
    "regexpComp-11.7",
    "regexpComp-11.8",
    "regexpComp-14.1",
    "regexpComp-14.2",
    # Non-evalInProc failures (same categories as regexp.test)
    "regexpComp-15.6",
    "regexpComp-16.4",
    "regexpComp-17.7",
    "regexpComp-18.7",
    "regexpComp-18.8",
    "regexpComp-18.9",
    "regexpComp-18.10",
    "regexpComp-18.11",
    "regexpComp-18.12",
    "regexpComp-19.1",
    "regexpComp-20.1",
    "regexpComp-20.2",
    "regexpComp-21.1",
    "regexpComp-21.2",
    "regexpComp-21.3",
    "regexpComp-21.4",
    "regexpComp-21.5",
    "regexpComp-21.6",
    "regexpComp-21.7",
    "regexpComp-21.8",
    "regexpComp-21.9",
    "regexpComp-21.10",
    "regexpComp-21.11",
    "regexpComp-22.0.1",
    "regexpComp-22.0.2",
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
