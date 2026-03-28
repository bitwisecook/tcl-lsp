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
    "oo-1.7",
    # oo-2.x: Constructor edge cases (interp)
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
    "oo-3.7",
    "oo-3.8",
    "oo-3.9",
    # oo-4.x: Method dispatch edge cases
    "oo-4.9",
    # oo-6.x: oo::define edge cases
    "oo-6.4",
    "oo-6.7",
    "oo-6.16",
    "oo-6.17",
    "oo-6.18",
    "oo-6.19",
    "oo-6.20",
    # oo-7.x: Inheritance edge cases
    "oo-7.6",
    "oo-7.7",
    "oo-7.9",
    "oo-7.10",
    # oo-11.x: Class methods
    "oo-11.5",
    "oo-11.6.4",
    "oo-11.7",
    "oo-11.8",
    # oo-12.x: Forward method edge cases
    "oo-12.2",
    "oo-12.3",
    "oo-12.7",
    "oo-12.8",
    # oo-13.x: Filter edge cases
    "oo-13.6",
    "oo-13.7",
    "oo-13.9",
    "oo-13.10",
    "oo-13.11",
    # oo-14.x: Mixin edge cases
    "oo-14.1",
    "oo-14.2",
    "oo-14.7",
    # oo-15.x: oo::objdefine edge cases
    "oo-15.6",
    "oo-15.8",
    "oo-15.9",
    "oo-15.10",
    "oo-15.12",
    "oo-15.13.2",
    "oo-15.14",
    "oo-15.15",
    # oo-16.x: info object/class edge cases
    "oo-16.10",
    "oo-16.11",
    "oo-16.14",
    "oo-16.16",
    # oo-17.x: definitionnamespace
    "oo-17.5",
    "oo-17.9",
    "oo-17.11",
    "oo-17.12",
    "oo-17.13",
    "oo-17.14",
    "oo-17.15",
    "oo-17.16",
    # oo-18.x: slot operations
    "oo-18.1",
    "oo-18.2",
    "oo-18.3",
    "oo-18.3a",
    "oo-18.3b",
    "oo-18.4",
    "oo-18.5",
    "oo-18.6",
    "oo-18.7",
    "oo-18.8",
    "oo-18.9",
    "oo-18.10",
    "oo-18.11",
    # oo-19.x: abstract class
    "oo-19.1",
    "oo-19.2",
    "oo-19.4",
    "oo-19.5",
    # oo-20.x: singleton/abstract/tag
    "oo-20.3",
    "oo-20.4",
    "oo-20.5",
    "oo-20.6",
    "oo-20.7",
    "oo-20.9",
    "oo-20.10",
    "oo-20.11",
    "oo-20.13",
    "oo-20.14",
    "oo-20.15",
    # oo-21.x: readableproperties/writableproperties
    "oo-21.2",
    "oo-21.3",
    "oo-21.4",
    # oo-22.x: property
    "oo-22.1",
    "oo-22.2",
    "oo-22.3",
    "oo-22.4",
    "oo-22.5",
    "oo-22.6",
    "oo-22.7",
    "oo-22.8",
    # oo-23.x: configurable
    "oo-23.1",
    # oo-24.x: configurable (advanced)
    "oo-24.1",
    "oo-24.2",
    "oo-24.3",
    # oo-26.x: property access
    "oo-26.2",
    "oo-26.3",
    # oo-27.x: configure
    "oo-27.5",
    "oo-27.6",
    "oo-27.8",
    "oo-27.9",
    "oo-27.13",
    "oo-27.14",
    "oo-27.15",
    "oo-27.16",
    "oo-27.17",
    "oo-27.18",
    "oo-27.19",
    "oo-27.20",
    "oo-27.21",
    "oo-27.22",
    "oo-27.23",
    # oo-28.x: configurable (complex)
    "oo-28.1",
    # oo-29.x
    "oo-29.1",
    # oo-30.x
    "oo-30.1",
    "oo-30.2",
    # oo-32.x
    "oo-32.2",
    "oo-32.3",
    "oo-32.4",
    "oo-32.5",
    "oo-32.6",
    "oo-32.7",
    # oo-33.x
    "oo-33.1",
    "oo-33.2",
    "oo-33.3",
    "oo-33.4",
    "oo-33.5",
    # oo-34.x
    "oo-34.1",
    "oo-34.2",
    "oo-34.3",
    "oo-34.4",
    "oo-34.5",
    "oo-34.6",
    "oo-34.7",
    "oo-34.8",
    "oo-34.9",
    "oo-34.10",
    # oo-35.x
    "oo-35.1",
    "oo-35.2",
    "oo-35.5",
    "oo-35.7.1",
    "oo-35.7.2",
    # oo-36.x
    "oo-36.3",
    "oo-36.4",
    "oo-36.5",
    "oo-36.6",
    "oo-36.7",
    "oo-36.8",
    "oo-36.9",
    "oo-36.10",
    # oo-37.x
    "oo-37.5",
    "oo-37.6",
    # oo-38.x
    "oo-38.1",
    "oo-38.2",
    "oo-38.3",
    "oo-38.4",
    "oo-38.5",
    # oo-39.x
    "oo-39.1",
    "oo-39.2",
    "oo-39.3",
    "oo-39.4",
    "oo-39.5",
    "oo-39.6",
    "oo-39.7",
    "oo-39.8",
    "oo-39.9",
    "oo-39.10",
    "oo-39.11",
    "oo-39.12",
    # oo-40.x
    "oo-40.1",
    "oo-40.2",
    "oo-40.3",
    # oo-41.x
    "oo-41.1",
    "oo-41.2",
    "oo-41.3",
    # oo-42.x
    "oo-42.3",
    "oo-42.4",
    "oo-42.5",
    "oo-42.6",
    "oo-42.7",
    # oo-43.x
    "oo-43.1",
    "oo-43.2",
    "oo-43.3",
    "oo-43.4",
    "oo-43.5",
    "oo-43.6",
    "oo-43.7",
    "oo-43.8",
    "oo-43.9",
    "oo-43.10",
    "oo-43.11",
    "oo-43.12",
    "oo-43.13",
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
