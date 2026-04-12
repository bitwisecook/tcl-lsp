"""End-to-end runner: real upstream tcltest executing tcllib counter tests.

Goal (from the next-plate briefing, Option A part 1):

    Concat tcltest.tcl + counter.tcl + counter.test into one compilation
    unit.  Compile to WASM through wasm_codegen_module (no special
    flags).  Instantiate in wasmtime, capture stdout.  Parse the
    ``Total … Passed … Failed … Skipped`` summary line tcltest emits.
    Assert ``Failed == 0`` on non-``timed`` cases.

We do **not** ship a tcltest shim — the user has explicitly asked for
real upstream tcltest through the normal init pipeline.  This runner is
expected to fail on the first CAT2 primitive that counter.test + real
tcltest hit; that failure is the input to the next implementation
commit.

The runner is staged as four progressively-harder tests so the first
failure tells us exactly which layer broke:

  1. ``test_tcltest_compiles`` — tcltest.tcl alone compiles to WASM
  2. ``test_tcltest_top_runs`` — tcltest's top-level executes without trapping
  3. ``test_counter_bundle_compiles`` — tcltest+counter+counter.test compiles
  4. ``test_counter_bundle_runs_and_passes``
     — bundle executes, summary parses, Failed == 0 on non-timed cases.

The counter.test file sources ``devtools/testutilities.tcl`` (a tcllib
testing harness, not tcltest itself) and uses its helpers
(``testsNeedTcl``, ``testsNeedTcltest``, ``testing { useLocal … }``,
``testsuiteCleanup``).  Because we're concatenating everything into one
compilation unit, the ``source`` line is stripped here and those four
helpers are provided as a tiny preamble — they are plumbing for
locating files on disk, which has no meaning in a single WASM module.
Everything that matters for test semantics (``test``, ``configure``,
``cleanupTests``, matching, constraints, output formatting) comes from
real upstream tcltest.
"""

from __future__ import annotations

import re
from pathlib import Path

import pytest

from tests.test_wasm_real_tcl import (
    _compile_tcl,
    _compile_tcl_with_diag,
    _resolve_trap,
    _run_wasm,
)

_EXTERNAL = Path(__file__).resolve().parent
_TCLTEST = _EXTERNAL / "tcllib" / "tcltest" / "tcltest.tcl"
_COUNTER_TCL = _EXTERNAL / "tcllib" / "counter" / "counter.tcl"
_COUNTER_TEST = _EXTERNAL / "tcllib" / "counter" / "counter.test"


# testutilities.tcl shim — locates files on disk and selects a package
# implementation.  None of that has meaning inside a single compiled
# WASM unit where tcltest and the module-under-test are already linked
# together.  Keep this intentionally minimal: anything that smells like
# test *semantics* must route through real tcltest, not here.
_TESTUTIL_STUB = r"""
# -- minimal stand-in for tcllib/devtools/testutilities.tcl --
# The real file resolves source paths on a filesystem and arranges
# package loading; in a single-unit WASM build those concerns don't
# exist, so these helpers are no-ops that satisfy the test file's API.
proc testsNeedTcl    {args} {}
proc testsNeedTcltest {args} {}
proc testing         {script} { uplevel 1 $script }
proc useLocal        {args} {}
proc useLocalKeep    {args} {}
proc useTcllibC      {args} { return 0 }
proc testsuiteCleanup {} { ::tcltest::cleanupTests }
# The test file does ``namespace import ::tcltest::*`` via the tcltest
# harness itself; nothing to do here.
"""


def _read_counter_test_stripped() -> str:
    """counter.test with the testutilities source line removed.

    The file begins with a multi-line ``source [file join ...
    devtools testutilities.tcl]`` that resolves a path on disk.  In a
    single-unit compile there is no disk; the stub above replaces what
    that file would have provided.
    """
    text = _COUNTER_TEST.read_text()
    # Strip the backslash-continued ``source [file join \ … devtools
    # testutilities.tcl]`` block.  Match ``source`` through the first
    # line that ends with a ``]`` not followed by a backslash.
    text = re.sub(
        r"^source \[file join(?:\s*\\\n[^\n]*)*\s*\]\s*\n",
        "# [source devtools/testutilities.tcl stripped — handled by preamble]\n",
        text,
        count=1,
        flags=re.MULTILINE,
    )
    return text


def _bundle_source() -> str:
    """Concatenate tcltest + counter module + counter test file."""
    parts = [
        "# ===== upstream tcltest (Tcl core 8.6.15 library/tcltest) =====",
        _TCLTEST.read_text(),
        "# ===== testutilities.tcl stub =====",
        _TESTUTIL_STUB,
        "# ===== counter.tcl (tcllib modules/counter) =====",
        _COUNTER_TCL.read_text(),
        "# ===== counter.test =====",
        _read_counter_test_stripped(),
    ]
    return "\n".join(parts) + "\n"


def _require_files():
    for p in (_TCLTEST, _COUNTER_TCL, _COUNTER_TEST):
        if not p.exists():
            pytest.skip(f"missing vendored file: {p}")


class TestTcltestCompiles:
    """Stage 1: does tcltest.tcl alone reach the code generator?"""

    def test_tcltest_compiles(self):
        _require_files()
        try:
            _compile_tcl(_TCLTEST.read_text())
        except Exception as e:
            pytest.fail(f"tcltest.tcl failed to compile: {e}")


class TestTcltestTopRuns:
    """Stage 2: does tcltest's top-level (namespace + procs) instantiate?"""

    def test_tcltest_top_runs(self):
        _require_files()
        try:
            wasm, diag = _compile_tcl_with_diag(_TCLTEST.read_text(), "tcltest.tcl")
        except Exception as e:
            pytest.skip(f"tcltest.tcl does not yet compile: {e}")
        try:
            result = _run_wasm(wasm, capture_stderr=True)
        except Exception as trap:
            # _run_wasm attaches captured stderr to the trap as
            # ``tcl_stderr`` so we can surface the ``tcl trap:
            # site=<id> …`` prefix and resolve it via the diag map.
            pytest.fail(_resolve_trap(trap, getattr(trap, "tcl_stderr", ""), diag))
        val = result[0]
        stderr_text = result[2] if len(result) == 3 else ""
        assert val == 0, f"::top returned {val}; stderr:\n{stderr_text}"


class TestCounterBundle:
    """Stages 3 & 4: whole-bundle compile and run."""

    def test_counter_bundle_compiles(self):
        _require_files()
        try:
            _compile_tcl(_bundle_source())
        except Exception as e:
            pytest.fail(f"bundled tcltest+counter+counter.test failed to compile: {e}")

    def test_counter_bundle_runs_and_passes(self):
        _require_files()
        try:
            wasm, diag = _compile_tcl_with_diag(_bundle_source(), "counter_bundle.tcl")
        except Exception as e:
            pytest.skip(f"bundle does not yet compile: {e}")
        try:
            result = _run_wasm(wasm, capture_stdout=True, capture_stderr=True)
        except Exception as trap:
            pytest.fail(_resolve_trap(trap, getattr(trap, "tcl_stderr", ""), diag))
        val, stdout = result[0], result[1]
        # tcltest's cleanupTests emits a summary like:
        #   all.tcl:        Total    9  Passed    9  Skipped    0  Failed    0
        # counter.test isn't run via all.tcl here, so we match the
        # generic "Total … Passed … Failed … Skipped" shape.
        m = re.search(
            r"Total\s+(\d+)\s+Passed\s+(\d+)\s+Skipped\s+(\d+)\s+Failed\s+(\d+)",
            stdout,
        )
        assert m is not None, f"no tcltest summary line in stdout (exit={val}).\nstdout:\n{stdout}"
        total, passed, skipped, failed = (int(x) for x in m.groups())
        # ``timed`` is the constraint on counter-timehist; we don't
        # enable it, so tcltest will skip that case.  Everything else
        # must pass.
        assert failed == 0, (
            f"tcltest reports {failed} failures "
            f"(total={total}, passed={passed}, skipped={skipped}).\n"
            f"stdout:\n{stdout}"
        )
