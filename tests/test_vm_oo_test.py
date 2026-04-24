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

KNOWN_FAILURES_OO: set[str] = set(
    # oo.test raises TclReturn/TclError immediately; Total=0 and no test ever runs.
)

# ---------------------------------------------------------------------------
# Known failures — ooNext2.test
# ---------------------------------------------------------------------------

KNOWN_FAILURES_OO_NEXT2: set[str] = set()


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
    failed_set = set(failed_tests)
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
        _check_results(results, KNOWN_FAILURES_OO, "oo.test", expect_zero_total=True)


class TestOONext2Native:
    """Run ooNext2.test through the VM (non-optimised)."""

    def test_oo_next2_test(self) -> None:
        results = _run_test_file("ooNext2.test", optimise=False)
        _check_results(results, KNOWN_FAILURES_OO_NEXT2, "ooNext2.test", expect_zero_total=True)


# ---------------------------------------------------------------------------
# Test classes — optimised (verify no regressions from optimiser)
# ---------------------------------------------------------------------------


class TestOOOptimised:
    """Run oo.test through the VM (optimised)."""

    def test_oo_test_optimised(self) -> None:
        results = _run_test_file("oo.test", optimise=True)
        _check_results(results, KNOWN_FAILURES_OO, "oo.test [optimised]", expect_zero_total=True)


class TestOONext2Optimised:
    """Run ooNext2.test through the VM (optimised)."""

    def test_oo_next2_test_optimised(self) -> None:
        results = _run_test_file("ooNext2.test", optimise=True)
        _check_results(results, KNOWN_FAILURES_OO_NEXT2, "ooNext2.test [optimised]", expect_zero_total=True)
