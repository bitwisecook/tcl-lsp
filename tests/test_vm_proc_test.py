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
from vm.commands import tcltest_cmds
from vm.commands.test_support_cmds import setup_test_support
from vm.interp import TclInterp

pytestmark = pytest.mark.slow

# Known failures
#
# Each set lists Tcl test names that are expected to fail in our VM.
# When a VM bug is fixed the test will unexpectedly pass — the set
# must be updated (removing the entry) to keep CI green.

KNOWN_FAILURES_PROC_OLD: set[str] = {
    # Trace on array element through global alias
    "proc-old-3.7",
    "proc-old-3.9",
    # Failed proc definition still leaves command accessible
    "proc-old-5.11",
    # $::errorInfo: line number wrong for error inside while body (compiled separately)
    # + unset trace not fired on proc frame cleanup
    "proc-old-5.16",
    # ReturnCode enum doesn't handle arbitrary integer codes (catch + return -code -14)
    "proc-old-7.6",
    # normalizeMsg command not found + $::errorInfo
    "proc-old-7.11",
    "proc-old-7.12",
    "proc-old-7.13",
    "proc-old-7.14",
}

KNOWN_FAILURES_RENAME: set[str] = {
    # Built-in rename not fully removing command from all lookup paths
    "rename-2.1",
    # $::errorCode not set on wrong-args errors
    "rename-3.1",
    "rename-3.2",
}

KNOWN_FAILURES_UNKNOWN: set[str] = {
    # Argument quoting in calls to unknown handler
    "unknown-3.1",
}

KNOWN_FAILURES_PROC: set[str] = {
    # Wrong # args error message format
    "proc-1.1",  # wrong # args format
    "proc-1.2",  # wrong # args format
    "proc-1.3",  # wrong # args format
    "proc-1.6",  # wrong # args format
    "proc-1.9",  # wrong # args format
    # Default argument handling
    "proc-2.3",  # default arg with special chars
    # Argument validation
    "proc-3.3",  # args validation
    "proc-3.4",  # args validation
    "proc-3.6",  # duplicate arg names
    "proc-3.7",  # duplicate arg names
    # Error propagation
    "proc-4.10",  # errorInfo format for proc errors
}

KNOWN_FAILURES_APPLY: set[str] = {
    # Malformed lambda checks
    "apply-2.1",  # error message format for malformed lambda
    "apply-2.2",  # error message format for malformed lambda
    "apply-2.3",  # error message format for malformed lambda
    "apply-2.4",  # error message format for malformed lambda
    "apply-2.5",  # error message format for malformed lambda
    # Argument handling
    "apply-3.1",  # wrong # args in lambda
    "apply-3.2",  # wrong # args in lambda
    "apply-3.3",  # wrong # args in lambda
    "apply-3.4",  # wrong # args in lambda
    # Namespace evaluation
    "apply-4.1",  # namespace resolution in lambda
    "apply-4.2",  # namespace resolution in lambda
    "apply-4.3",  # namespace resolution in lambda
    "apply-4.4",  # namespace resolution in lambda
    "apply-4.5",  # namespace resolution in lambda
    # Error in body
    "apply-5.1",  # errorInfo for error in lambda body
    # Local variable scoping
    "apply-6.2",  # local variable isolation
    "apply-6.3",  # local variable isolation
    # Nested and complex lambdas
    "apply-7.2",  # nested lambda / uplevel
    "apply-7.3",  # nested lambda / uplevel
    "apply-7.4",  # nested lambda / uplevel
    "apply-7.6",  # nested lambda / uplevel
    "apply-7.7",  # nested lambda / uplevel
    "apply-7.8",  # nested lambda / uplevel
    # Lambda in ensemble / advanced
    "apply-8.1",  # advanced lambda usage
    "apply-8.2",  # advanced lambda usage
    "apply-8.3",  # advanced lambda usage
    "apply-8.4",  # advanced lambda usage
    "apply-8.5",  # advanced lambda usage
    "apply-8.6",  # advanced lambda usage
    "apply-8.7",  # advanced lambda usage
    "apply-8.8",  # advanced lambda usage
    "apply-8.9",  # advanced lambda usage
    "apply-8.10",  # advanced lambda usage
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
