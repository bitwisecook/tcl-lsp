"""Tcl 9 tcltest suite runner via WASM compilation.

Uses the real Tcl 9 tcltest package from ``tmp/tcl9.0.3/library/tcltest/``
and runs individual test files from ``tmp/tcl9.0.3/tests/``.  Each test
file is bundled with tcltest.tcl into a single compilation unit, compiled
to WASM, and executed via wasmtime.

Stage structure (mirrors run_tcllib_test.py):

  1. ``test_tcltest9_compiles``  — Tcl 9 tcltest.tcl alone compiles to WASM
  2. ``test_tcltest9_top_runs``  — top-level init executes without trapping
  3. ``test_<name>_compiles``    — tcltest + test-file bundle compiles
  4. ``test_<name>_runs``        — bundle executes and reports Failed == 0

"Init from there" means we use the Tcl 9 tcltest as the init source — its
``package provide tcltest 2.5.10`` wires up the real tcltest machinery rather
than a hand-rolled shim.  Test files whose ``if {"::tcltest" ni ...}`` guard
would skip the namespace import get it via the preamble we inject between
tcltest.tcl and the test file body.

Known accepted limitations
--------------------------
- Tests that require the C extension ``tcl::test`` (testbytestring, etc.)
  are skipped by the test file's own ``testConstraint`` calls — correct.
- Filesystem-heavy tests (makeFile, cd-based) need a preopen'd tmpdir.
- Some Tcl 9-specific commands may trap if not yet implemented; those
  tests will show up as XFAIL until the implementation catches up.
"""

from __future__ import annotations

import re
import tempfile
from pathlib import Path

import pytest

from tests.conftest import ensure_tcl_source
from tests.test_wasm_real_tcl import (
    _compile_tcl,
    _compile_tcl_with_diag,
    _resolve_trap,
    _run_wasm,
)

_EXTERNAL = Path(__file__).resolve().parent

# ---------------------------------------------------------------------------
# Resolve Tcl 9 source paths (lazy — skip if not available)
# ---------------------------------------------------------------------------

def _tcl9_tcltest() -> Path:
    """Return path to Tcl 9 tcltest.tcl, skip if missing."""
    tests_dir = ensure_tcl_source("9.0")
    p = tests_dir.parent / "library" / "tcltest" / "tcltest.tcl"
    if not p.exists():
        pytest.skip(f"Tcl 9 tcltest.tcl not found at {p}")
    return p


def _tcl9_test_file(name: str) -> Path:
    """Return path to a Tcl 9 test file, skip if missing."""
    tests_dir = ensure_tcl_source("9.0")
    p = tests_dir / name
    if not p.exists():
        pytest.skip(f"Tcl 9 test file not found: {p}")
    return p


# ---------------------------------------------------------------------------
# Bundle construction
# ---------------------------------------------------------------------------

# Injected between tcltest.tcl and the test file.
# Responsibilities:
#   1. Import tcltest commands into the global namespace — test files check
#      whether ``::tcltest`` is already a child namespace and skip their own
#      ``namespace import`` block if it is.  We perform the import here so
#      unqualified ``test``, ``testConstraint``, ``cleanupTests``, etc. work.
#   2. Configure a silent output channel (no "Running ..." chatter) so that
#      only the final summary line lands in stdout.
_PREAMBLE = r"""
# ----- run_tcl9_tests.py preamble -----
# Import tcltest commands into the global namespace.  Test files that detect
# ::tcltest already loaded skip their own namespace import block; we fill the
# gap here so unqualified 'test', 'testConstraint', etc. resolve correctly.
namespace import -force ::tcltest::*

# Silence per-test progress chatter; keep the summary line.
::tcltest::configure -verbose {}
"""


def _bundle(test_file_path: Path) -> str:
    """Concatenate Tcl 9 tcltest + preamble + test file into one script."""
    tcltest_src = _tcl9_tcltest().read_text(encoding="utf-8")
    test_src = test_file_path.read_text(encoding="utf-8")

    return "\n".join([
        "# ===== Tcl 9 tcltest (tmp/tcl9.0.3/library/tcltest/tcltest.tcl) =====",
        tcltest_src,
        "# ===== run_tcl9_tests preamble =====",
        _PREAMBLE,
        f"# ===== {test_file_path.name} =====",
        test_src,
        "",
    ])


def _parse_summary(stdout: str) -> tuple[int, int, int, int] | None:
    """Extract (total, passed, skipped, failed) from a tcltest summary line.

    tcltest cleanupTests emits (with tabs):
        <filename>:\tTotal\t<N>\tPassed\t<N>\tSkipped\t<N>\tFailed\t<N>

    The regex uses \\s+ to match any whitespace (spaces or tabs) and \\d* to
    allow blank Passed/Skipped counts (treated as 0).
    """
    m = re.search(
        r"Total\s+(\d+)\s+Passed\s+(\d*)\s+Skipped\s+(\d*)\s+Failed\s+(\d*)",
        stdout,
    )
    if m is None:
        return None
    def _int(s: str) -> int:
        return int(s) if s else 0
    return (_int(m.group(1)), _int(m.group(2)), _int(m.group(3)), _int(m.group(4)))


def _run_bundle(bundle_src: str, label: str) -> tuple[str, str]:
    """Compile and run a bundle; return (stdout, stderr).

    Raises AssertionError on compile or runtime failure, with a
    human-readable diagnostic that includes the trap site if available.
    """
    try:
        wasm, diag = _compile_tcl_with_diag(bundle_src, label)
    except Exception as exc:
        pytest.fail(f"{label}: compilation failed: {exc}")

    with tempfile.TemporaryDirectory(prefix="tcl9test-") as host_tmp:
        try:
            result = _run_wasm(
                wasm,
                capture_stdout=True,
                capture_stderr=True,
                preopen_tmpdir=host_tmp,
            )
        except Exception as trap:
            pytest.fail(_resolve_trap(trap, getattr(trap, "tcl_stderr", ""), diag))

    # result is (retval, stdout, stderr)
    stdout = result[1] if len(result) >= 2 else ""
    stderr = result[2] if len(result) >= 3 else ""
    return stdout, stderr


# ---------------------------------------------------------------------------
# Stage 1 & 2: tcltest.tcl alone
# ---------------------------------------------------------------------------

class TestTcltest9Init:
    """Stages 1–2: Tcl 9 tcltest.tcl compiles and its top-level runs."""

    def test_tcltest9_compiles(self):
        """Tcl 9 tcltest.tcl compiles to WASM without errors."""
        src = _tcl9_tcltest().read_text(encoding="utf-8")
        try:
            _compile_tcl(src)
        except Exception as exc:
            pytest.fail(f"Tcl 9 tcltest.tcl failed to compile: {exc}")

    def test_tcltest9_top_runs(self):
        """Tcl 9 tcltest.tcl top-level init executes without trapping."""
        src = _tcl9_tcltest().read_text(encoding="utf-8")
        try:
            wasm, diag = _compile_tcl_with_diag(src, "tcl9_tcltest.tcl")
        except Exception as exc:
            pytest.skip(f"Tcl 9 tcltest.tcl does not yet compile: {exc}")

        with tempfile.TemporaryDirectory(prefix="tcl9test-init-") as host_tmp:
            try:
                result = _run_wasm(wasm, capture_stderr=True, preopen_tmpdir=host_tmp)
            except Exception as trap:
                pytest.fail(_resolve_trap(trap, getattr(trap, "tcl_stderr", ""), diag))

        val = result[0]
        stderr_text = result[2] if len(result) >= 3 else ""
        assert val == 0, f"::top returned {val}; stderr:\n{stderr_text}"


# ---------------------------------------------------------------------------
# Stage 3 & 4: individual Tcl 9 test files
# ---------------------------------------------------------------------------

def _make_test_class(test_name: str, xfail_run: bool = False):
    """Dynamically build a test class for a Tcl 9 .test file.

    ``test_name`` is the stem of the file (e.g. ``"append"`` for
    ``append.test``).  ``xfail_run`` marks stage-4 as xfail when the file
    is known to hit unimplemented commands.
    """
    filename = f"{test_name}.test"

    class _TestClass:
        def test_compiles(self):
            f"""Tcl 9 {filename} bundle compiles to WASM."""
            test_path = _tcl9_test_file(filename)
            src = _bundle(test_path)
            try:
                _compile_tcl(src)
            except Exception as exc:
                pytest.fail(f"{filename} bundle failed to compile: {exc}")

        def test_runs(self):
            f"""Tcl 9 {filename} bundle executes and reports Failed == 0."""
            test_path = _tcl9_test_file(filename)
            src = _bundle(test_path)
            stdout, stderr = _run_bundle(src, filename)

            summary = _parse_summary(stdout)
            assert summary is not None, (
                f"No tcltest summary line in stdout for {filename}.\n"
                f"stdout:\n{stdout}\nstderr:\n{stderr}"
            )
            total, passed, skipped, failed = summary
            assert failed == 0, (
                f"{filename}: {failed} test(s) FAILED "
                f"(total={total}, passed={passed}, skipped={skipped}).\n"
                f"stdout:\n{stdout}\nstderr:\n{stderr}"
            )

    if xfail_run:
        _TestClass.test_runs = pytest.mark.xfail(  # type: ignore[method-assign]
            reason=f"{filename} hits unimplemented commands; xfail until fixed",
            strict=False,
        )(_TestClass.test_runs)

    _TestClass.__name__ = f"TestTcl9_{test_name.replace('-', '_')}"
    _TestClass.__qualname__ = _TestClass.__name__
    return _TestClass


# ---------------------------------------------------------------------------
# Register test classes for each Tcl 9 test file.
#
# Start with the simplest, self-contained test suites.  Mark those known to
# need unimplemented commands (filesystem I/O, encoding, C extensions) as
# xfail on the run stage so compile-stage failures still surface.
# ---------------------------------------------------------------------------

# Simple string / value commands — most tests use only core built-ins
TestTcl9_append   = _make_test_class("append")
TestTcl9_concat   = _make_test_class("concat")
TestTcl9_list     = _make_test_class("list")
TestTcl9_lsort    = _make_test_class("lsort")
TestTcl9_lsearch  = _make_test_class("lsearch")
TestTcl9_lrange   = _make_test_class("lrange")
TestTcl9_lreplace = _make_test_class("lreplace")
TestTcl9_lindex   = _make_test_class("lindex")
TestTcl9_linsert  = _make_test_class("linsert")
TestTcl9_llength  = _make_test_class("llength")
TestTcl9_dict     = _make_test_class("dict")
TestTcl9_string   = _make_test_class("string")
TestTcl9_apply    = _make_test_class("apply")
TestTcl9_expr     = _make_test_class("expr")
TestTcl9_format   = _make_test_class("format")
TestTcl9_scan     = _make_test_class("scan")
TestTcl9_split    = _make_test_class("split")
TestTcl9_join     = _make_test_class("join")
TestTcl9_incr     = _make_test_class("incr")
TestTcl9_eval     = _make_test_class("eval")
TestTcl9_error    = _make_test_class("error")
TestTcl9_for_     = _make_test_class("for")
TestTcl9_foreach  = _make_test_class("foreach")
TestTcl9_while_   = _make_test_class("while")
TestTcl9_if_      = _make_test_class("if")
TestTcl9_proc_    = _make_test_class("proc")
TestTcl9_return_  = _make_test_class("return")
TestTcl9_upvar    = _make_test_class("upvar")
TestTcl9_uplevel  = _make_test_class("uplevel")
TestTcl9_variable = _make_test_class("variable")
TestTcl9_namespace = _make_test_class("namespace")
