"""Run oo.test and ooNext2.test natively through the VM's tcltest.

Phase 10: verifies the full TclOO test suite (Tcl 9.0.3) through both
non-optimised and optimised VM paths.  Failure sets are tracked per-file
so that regressions are caught immediately while expected failures don't
block CI.
"""

from __future__ import annotations

import io

import pytest

from tests.conftest import ensure_tcl_source
from vm.commands import tcltest_cmds
from vm.commands.test_support_cmds import setup_test_support
from vm.interp import TclInterp

pytestmark = pytest.mark.slow

# ---------------------------------------------------------------------------
# Known failures — oo.test
#
# These are the Tcl test names that are expected to fail in our VM.
# When a VM bug is fixed the test will unexpectedly pass — the set must
# be updated (removing the entry) to keep CI green.
# ---------------------------------------------------------------------------

KNOWN_FAILURES_OO: set[str] = {
    # oo-0.x: Package/version introspection that relies on C-level internals
    "oo-0.2",
    "oo-0.6",
    "oo-0.7",
    "oo-0.9",
    # oo-1.x: Object creation edge cases (interp, namespace, rename)
    "oo-1.10",
    "oo-1.18.3",
    "oo-1.18.4",
    "oo-1.18.5",
    "oo-1.19",
    "oo-1.2",
    "oo-1.20",
    "oo-1.21",
    "oo-1.22",
    "oo-1.25",
    "oo-1.3",
    "oo-1.5.1",
    "oo-1.6",
    "oo-1.7",
    # oo-2.x: Constructor edge cases
    "oo-2.1",
    "oo-2.6",
    "oo-2.7",
    "oo-2.8",
    "oo-2.9",
    # oo-3.x: Destructor edge cases
    "oo-3.1",
    "oo-3.12",
    "oo-3.2",
    "oo-3.3",
    "oo-3.4",
    "oo-3.4a",
    "oo-3.5",
    "oo-3.5a",
    "oo-3.6",
    "oo-3.7",
    "oo-3.8",
    "oo-3.9",
    # oo-4.x: Method dispatch edge cases (errorInfo, uplevel, variable)
    "oo-4.1",
    "oo-4.10",
    "oo-4.2",
    "oo-4.3",
    "oo-4.7",
    "oo-4.8",
    "oo-4.9",
    # oo-5.x: Visibility enforcement edge cases
    "oo-5.2",
    "oo-5.3",
    "oo-5.4",
    "oo-5.5",
    # oo-6.x: oo::define edge cases (namespace, error messages)
    "oo-6.1",
    "oo-6.4",
    "oo-6.7",
    "oo-6.8",
    "oo-6.9",
    "oo-6.10",
    "oo-6.11",
    "oo-6.12",
    "oo-6.13",
    "oo-6.14",
    "oo-6.15",
    "oo-6.16",
    "oo-6.17",
    "oo-6.18",
    "oo-6.19",
    "oo-6.20",
    # oo-7.x: Inheritance / superclass edge cases
    "oo-7.1",
    "oo-7.4",
    "oo-7.5",
    "oo-7.6",
    "oo-7.7",
    "oo-7.9",
    "oo-7.10",
    # oo-8.x: Error messages
    "oo-8.1",
    # oo-10.x: Variable edge cases
    "oo-10.2",
    # oo-11.x: Class methods / class-level dispatch
    "oo-11.1",
    "oo-11.2",
    "oo-11.3",
    "oo-11.4",
    "oo-11.5",
    "oo-11.6.4",
    "oo-11.7",
    "oo-11.8",
    # oo-12.x: Forward method edge cases
    "oo-12.1",
    "oo-12.2",
    "oo-12.3",
    "oo-12.4",
    "oo-12.5",
    "oo-12.6",
    "oo-12.7",
    "oo-12.8",
    # oo-13.x: Filter edge cases
    "oo-13.1",
    "oo-13.2",
    "oo-13.4",
    "oo-13.5",
    "oo-13.6",
    "oo-13.7",
    "oo-13.8",
    "oo-13.9",
    "oo-13.10",
    "oo-13.11",
    # oo-14.x: Mixin edge cases
    "oo-14.1",
    "oo-14.2",
    "oo-14.3",
    "oo-14.4",
    "oo-14.5",
    "oo-14.7",
    "oo-14.9",
    "oo-14.10",
    # oo-15.x: oo::objdefine edge cases
    "oo-15.2",
    "oo-15.3",
    "oo-15.5",
    "oo-15.6",
    "oo-15.7",
    "oo-15.8",
    "oo-15.9",
    "oo-15.10",
    "oo-15.11",
    "oo-15.12",
    "oo-15.13.2",
    "oo-15.14",
    "oo-15.15",
    # oo-16.x: info object/class edge cases
    "oo-16.4",
    "oo-16.6",
    "oo-16.7",
    "oo-16.8",
    "oo-16.10",
    "oo-16.11",
    "oo-16.12",
    "oo-16.13",
    "oo-16.14",
    "oo-16.15",
    "oo-16.16",
    "oo-16.17",
    "oo-16.18",
    "oo-16.18.1",
}

# ---------------------------------------------------------------------------
# Known failures — ooNext2.test
# ---------------------------------------------------------------------------

KNOWN_FAILURES_OO_NEXT2: set[str] = {
    # oo-call-3.*: Error cases
    "oo-call-3.1",
    "oo-call-3.3",
    "oo-call-3.4",
    # oo-nextto-*: nextto edge cases (error messages, interp)
    "oo-nextto-1.3",
    "oo-nextto-1.4",
    "oo-nextto-2.2",
    "oo-nextto-2.3",
    "oo-nextto-2.4",
    "oo-nextto-2.5",
    "oo-nextto-2.6",
    "oo-nextto-2.7",
    # next-tailcall-*: tailcall interaction (not implemented)
    "next-tailcall-constructor-1",
    "next-tailcall-destructor-1",
    "next-tailcall-filter-1",
    "next-tailcall-forward-1",
    "next-tailcall-mixin-1",
    "next-tailcall-objmixin-1",
    "next-tailcall-simple-1",
    "next-tailcall-simple-2",
    "next-tailcall-simple-3",
    "next-tailcall-simple-4",
    "next-tailcall-superclass-1",
    "next-tailcall-superclass-2",
}


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _run_test_file(test_file: str, *, optimise: bool = False) -> dict[str, object]:
    """Source a .test file through the VM and return results.

    Returns a dict with keys: Total, Passed, Skipped, Failed,
    failed_tests (list of test names), and output (captured stdout).
    """
    interp = TclInterp(source_init=False, optimise=optimise)
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
    failed_tests = results["failed_tests"]
    assert isinstance(failed_tests, list)
    failed_set = set(failed_tests)
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


# ---------------------------------------------------------------------------
# Test classes — non-optimised
# ---------------------------------------------------------------------------


class TestOONative:
    """Run oo.test through the VM (non-optimised)."""

    def test_oo_test(self) -> None:
        results = _run_test_file("oo.test", optimise=False)
        _check_results(results, KNOWN_FAILURES_OO, "oo.test")


class TestOONext2Native:
    """Run ooNext2.test through the VM (non-optimised)."""

    def test_oo_next2_test(self) -> None:
        results = _run_test_file("ooNext2.test", optimise=False)
        _check_results(results, KNOWN_FAILURES_OO_NEXT2, "ooNext2.test")


# ---------------------------------------------------------------------------
# Test classes — optimised (verify no regressions from optimiser)
# ---------------------------------------------------------------------------


class TestOOOptimised:
    """Run oo.test through the VM (optimised)."""

    def test_oo_test_optimised(self) -> None:
        results = _run_test_file("oo.test", optimise=True)
        _check_results(results, KNOWN_FAILURES_OO, "oo.test [optimised]")


class TestOONext2Optimised:
    """Run ooNext2.test through the VM (optimised)."""

    def test_oo_next2_test_optimised(self) -> None:
        results = _run_test_file("ooNext2.test", optimise=True)
        _check_results(results, KNOWN_FAILURES_OO_NEXT2, "ooNext2.test [optimised]")
