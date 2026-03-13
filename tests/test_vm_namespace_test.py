"""Run namespace-related test files natively through the VM's tcltest.

Covers namespace.test and namespace-old.test from the Tcl 9.0.3 test
suite (Phase 5d of the VM test conformance plan).

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

KNOWN_FAILURES_NAMESPACE: set[str] = {
    # namespace eval / namespace qualifiers
    "namespace-3.1",  # namespace eval scoping
    "namespace-6.1",  # namespace export
    "namespace-6.2",  # namespace export
    "namespace-6.3",  # namespace export
    "namespace-6.4",  # namespace export
    # namespace import
    "namespace-7.1",  # namespace import
    "namespace-7.4",  # namespace import -force
    "namespace-7.6",  # namespace import pattern
    "namespace-7.7",  # namespace import pattern
    "namespace-7.9",  # namespace import error
    # namespace forget
    "namespace-8.1",  # namespace forget
    "namespace-8.3",  # namespace forget
    "namespace-8.4",  # namespace forget
    "namespace-8.5",  # namespace forget
    "namespace-8.6",  # namespace forget
    "namespace-8.7",  # namespace forget
    # namespace which
    "namespace-9.1",  # namespace which
    "namespace-9.3",  # namespace which -command
    "namespace-9.4",  # namespace which -variable
    "namespace-9.5",  # namespace which -variable
    "namespace-9.7",  # namespace which error
    "namespace-9.8",  # namespace which error
    "namespace-9.9",  # namespace which error
    # namespace code
    "namespace-10.1",  # namespace code
    "namespace-10.2",  # namespace code
    "namespace-10.3",  # namespace code
    "namespace-10.4",  # namespace code
    "namespace-10.8",  # namespace code
    # namespace inscope
    "namespace-11.1",  # namespace inscope
    "namespace-11.2",  # namespace inscope
    "namespace-11.3",  # namespace inscope
    # namespace origin
    "namespace-12.1",  # namespace origin
    # namespace parent
    "namespace-13.1",  # namespace parent
    "namespace-13.2",  # namespace parent
    # namespace children
    "namespace-14.1",  # namespace children
    "namespace-14.2",  # namespace children
    "namespace-14.3",  # namespace children
    "namespace-14.4",  # namespace children
    "namespace-14.5",  # namespace children pattern
    "namespace-14.6",  # namespace children pattern
    "namespace-14.7",  # namespace children error
    "namespace-14.8",  # namespace children error
    "namespace-14.11",  # namespace children
    "namespace-14.12",  # namespace children
    "namespace-14.13",  # namespace children
    # namespace delete cleanup
    "namespace-15.4",  # namespace delete variable cleanup
    # variable resolution
    "namespace-16.3",  # variable resolution across namespaces
    "namespace-16.11",  # variable resolution
    # namespace ensemble
    "namespace-18.1",  # namespace ensemble
    "namespace-18.2",  # namespace ensemble
    # namespace upvar
    "namespace-19.3",  # namespace upvar
    "namespace-19.4",  # namespace upvar
    # namespace unknown
    "namespace-20.2",  # namespace unknown handler
    "namespace-20.3",  # namespace unknown handler
    # apply
    "namespace-21.4",  # apply in namespace
    "namespace-21.5",  # apply in namespace
    "namespace-21.7",  # apply in namespace
    # namespace qualifiers/tail
    "namespace-22.1",  # namespace qualifiers
    "namespace-22.2",  # namespace qualifiers
    "namespace-22.3",  # namespace qualifiers
    "namespace-22.7",  # namespace tail
    # namespace exists
    "namespace-23.1",  # namespace exists
    # namespace current in procs
    "namespace-25.1",  # namespace current
    "namespace-25.2",  # namespace current
    "namespace-25.6",  # namespace current
    "namespace-25.7",  # namespace current
    "namespace-25.8",  # namespace current
    "namespace-25.9",  # namespace current
    # Command resolution
    "namespace-26.3",  # command resolution
    "namespace-26.4",  # command resolution
    "namespace-26.5",  # command resolution
    "namespace-26.6",  # command resolution
    "namespace-26.7",  # command resolution
    # Namespace deletion effects
    "namespace-27.1",  # namespace deletion
    "namespace-27.2",  # namespace deletion
    "namespace-27.3",  # namespace deletion
    # Nested namespace
    "namespace-28.1",  # nested namespace eval
    "namespace-28.3",  # nested namespace eval
    # Error in namespace eval
    "namespace-29.1",  # error in namespace eval
    "namespace-29.2",  # error in namespace eval
    "namespace-29.3",  # error in namespace eval
    "namespace-29.6",  # error in namespace eval
    # Namespace ensemble subcommands
    "namespace-30.2",  # ensemble subcommand
    "namespace-30.5",  # ensemble subcommand
    "namespace-31.1",  # ensemble map
    "namespace-31.4",  # ensemble map
    "namespace-32.2",  # ensemble configure
    "namespace-32.6",  # ensemble configure
    "namespace-32.7",  # ensemble configure
    "namespace-32.8",  # ensemble configure
    "namespace-33.2",  # ensemble prefixes
    # Ensemble unknown handler
    "namespace-34.2",  # ensemble unknown
    "namespace-34.3",  # ensemble unknown
    "namespace-34.4",  # ensemble unknown
    "namespace-34.5",  # ensemble unknown
    "namespace-34.6",  # ensemble unknown
    "namespace-34.7",  # ensemble unknown
    # Ensemble parameters
    "namespace-35.2",  # ensemble parameters
    # Variable resolution order
    "namespace-39.3",  # variable resolution
    # Tailcall in namespace
    "namespace-41.1",  # tailcall
    "namespace-41.2",  # tailcall
    "namespace-41.3",  # tailcall
    # Namespace ensemble create/config
    "namespace-42.1",  # ensemble create
    "namespace-42.2",  # ensemble create
    "namespace-42.3",  # ensemble create
    "namespace-42.4",  # ensemble create
    "namespace-42.5",  # ensemble create
    "namespace-42.8",  # ensemble create
    "namespace-42.9",  # ensemble create
    "namespace-42.10",  # ensemble create
    "namespace-42.11",  # ensemble create
    # Ensemble exists
    "namespace-43.1",  # ensemble exists
    "namespace-43.2",  # ensemble exists
    "namespace-43.4",  # ensemble exists
    "namespace-43.7",  # ensemble exists
    "namespace-43.9",  # ensemble exists
    "namespace-43.12",  # ensemble exists
    "namespace-43.14",  # ensemble exists
    "namespace-43.16",  # ensemble exists
    # Ensemble info
    "namespace-44.2",  # ensemble info
    "namespace-44.3",  # ensemble info
    "namespace-44.4",  # ensemble info
    "namespace-44.5",  # ensemble info
    "namespace-44.6",  # ensemble info
    # Ensemble error handling
    "namespace-45.1",  # ensemble error
    "namespace-45.2",  # ensemble error
    "namespace-46.1",  # ensemble error
    "namespace-46.2",  # ensemble error
    "namespace-46.3",  # ensemble error
    "namespace-46.4",  # ensemble error
    "namespace-46.7",  # ensemble error
    "namespace-46.9",  # ensemble error
    # Ensemble dispatch
    "namespace-47.1",  # ensemble dispatch
    "namespace-47.2",  # ensemble dispatch
    "namespace-47.3",  # ensemble dispatch
    "namespace-47.4",  # ensemble dispatch
    "namespace-47.5",  # ensemble dispatch
    "namespace-47.6",  # ensemble dispatch
    "namespace-47.7",  # ensemble dispatch
    "namespace-47.8",  # ensemble dispatch
    # Ensemble with namespace path
    "namespace-48.1",  # ensemble + path
    "namespace-48.2",  # ensemble + path
    "namespace-48.3",  # ensemble + path
    # Interp alias + namespace
    "namespace-49.1",  # interp alias
    "namespace-49.2",  # interp alias
    # TIP 314 — namespace ensemble compile
    "namespace-50.1",  # ensemble compile
    "namespace-50.2",  # ensemble compile
    "namespace-50.3",  # ensemble compile
    "namespace-50.4",  # ensemble compile
    "namespace-50.5",  # ensemble compile
    "namespace-50.6",  # ensemble compile
    "namespace-50.7",  # ensemble compile
    "namespace-50.8",  # ensemble compile
    "namespace-50.9",  # ensemble compile
    # TIP 400 — namespace ensemble rewrite
    "namespace-51.2",  # ensemble rewrite
    "namespace-51.3",  # ensemble rewrite
    "namespace-51.4",  # ensemble rewrite
    "namespace-51.5",  # ensemble rewrite
    "namespace-51.6",  # ensemble rewrite
    "namespace-51.7",  # ensemble rewrite
    "namespace-51.8",  # ensemble rewrite
    "namespace-51.9",  # ensemble rewrite
    "namespace-51.10",  # ensemble rewrite
    "namespace-51.11",  # ensemble rewrite
    "namespace-51.12",  # ensemble rewrite
    "namespace-51.13",  # ensemble rewrite
    "namespace-51.14",  # ensemble rewrite
    "namespace-51.16",  # ensemble rewrite
    "namespace-51.17",  # ensemble rewrite
    "namespace-51.18",  # ensemble rewrite
    # Ensemble deprecation
    "namespace-52.1",  # ensemble deprecation
    "namespace-52.4",  # ensemble deprecation
    "namespace-52.5",  # ensemble deprecation
    "namespace-52.6",  # ensemble deprecation
    "namespace-52.7",  # ensemble deprecation
    "namespace-52.8",  # ensemble deprecation
    "namespace-52.9",  # ensemble deprecation
    "namespace-52.12",  # ensemble deprecation
    # Namespace ensemble with args
    "namespace-53.1",  # ensemble args
    "namespace-53.2",  # ensemble args
    "namespace-53.3",  # ensemble args
    "namespace-53.4",  # ensemble args
    "namespace-53.5",  # ensemble args
    "namespace-53.6",  # ensemble args
    "namespace-53.7",  # ensemble args
    "namespace-53.8",  # ensemble args
    "namespace-53.9",  # ensemble args
    "namespace-53.10",  # ensemble args
    "namespace-53.11",  # ensemble args
    # Ensemble compilation
    "namespace-55.1",  # ensemble compile
    "namespace-55.2",  # ensemble compile
    # Namespace upvar + trace
    "namespace-56.1",  # upvar + trace
    "namespace-56.2",  # upvar + trace
    "namespace-56.3",  # upvar + trace
    "namespace-56.6",  # upvar + trace
    # Namespace path resolution
    "namespace-57.0",  # path resolution
}

KNOWN_FAILURES_NAMESPACE_OLD: set[str] = {
    # Namespace creation / eval
    "namespace-old-1.3",  # namespace eval scoping
    "namespace-old-1.4",  # namespace eval scoping
    "namespace-old-1.6",  # namespace eval scoping
    "namespace-old-1.7",  # namespace eval scoping
    "namespace-old-1.10",  # namespace eval scoping
    # Namespace deletion
    "namespace-old-2.2",  # namespace delete
    "namespace-old-2.8",  # namespace delete
    "namespace-old-2.9",  # namespace delete
    # Namespace export
    "namespace-old-3.2",  # namespace export
    # Namespace import
    "namespace-old-4.3",  # namespace import
    "namespace-old-4.4",  # namespace import
    # Variable resolution
    "namespace-old-5.9",  # variable resolution
    "namespace-old-5.10",  # variable resolution
    "namespace-old-5.11",  # variable resolution
    "namespace-old-5.16",  # variable resolution
    "namespace-old-5.18",  # variable resolution
    "namespace-old-5.19",  # variable resolution
    # Namespace children / parent / qualifiers
    "namespace-old-6.7",  # namespace children
    "namespace-old-6.8",  # namespace children
    "namespace-old-6.9",  # namespace children
    "namespace-old-6.10",  # namespace parent
    "namespace-old-6.11",  # namespace parent
    "namespace-old-6.16",  # namespace qualifiers
    "namespace-old-6.17",  # namespace qualifiers
    "namespace-old-6.18",  # namespace tail
    "namespace-old-6.19",  # namespace tail
    # Command resolution / import patterns
    "namespace-old-7.3",  # command resolution
    "namespace-old-7.4",  # command resolution
    "namespace-old-7.5",  # command resolution
    "namespace-old-7.6",  # command resolution
    "namespace-old-7.7",  # command resolution
    "namespace-old-7.10",  # command resolution
    "namespace-old-7.11",  # command resolution
    "namespace-old-7.12",  # command resolution
    # Namespace code / inscope
    "namespace-old-8.1",  # namespace code / inscope
    # Namespace origin
    "namespace-old-9.2",  # namespace origin
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


class TestNamespaceNative:
    """Run tmp/tcl9.0.3/tests/namespace.test through the VM."""

    def test_namespace(self) -> None:
        results = _run_test_file("namespace.test")
        _check_results(results, KNOWN_FAILURES_NAMESPACE, "namespace.test")


class TestNamespaceOldNative:
    """Run tmp/tcl9.0.3/tests/namespace-old.test through the VM."""

    def test_namespace_old(self) -> None:
        results = _run_test_file("namespace-old.test")
        _check_results(results, KNOWN_FAILURES_NAMESPACE_OLD, "namespace-old.test")
