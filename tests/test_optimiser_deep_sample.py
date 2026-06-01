"""Behavioural-equivalence checks for samples/optimiser/deep_pipeline.tcl.

The sample is engineered to drive the multi-pass optimiser through a deep,
multi-kind cascade (interprocedural folding + constant propagation + expr
folding + nested-expr unwrap + string-build chains + dead-store elimination).
These tests prove three things on real C Tcl 9:

1. the original sample runs and prints its documented output;
2. the optimiser actually does deep, varied work (many passes, several codes,
   including interprocedural O103); and
3. the optimised, formatted, and minified forms each still execute on tclsh9.0
   and produce byte-identical output — i.e. the three tools preserve behaviour.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
from collections import Counter
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.optimiser import optimise_source_multipass
from compiler.registry.runtime import configure_signatures
from tooling.formatter import format_tcl
from tooling.minifier import minify_tcl

_SAMPLE = Path(__file__).resolve().parent.parent / "samples" / "optimiser" / "deep_pipeline.tcl"
_EXPECTED_OUTPUT = "tcl v9.0 score=62 x2=124\n"


def _find_tcl9() -> str | None:
    for cand in (shutil.which("tclsh9.0"), shutil.which("tclsh")):
        if not cand or not Path(cand).exists():
            continue
        try:
            proc = subprocess.run(
                [cand], input="puts [info patchlevel]", capture_output=True, text=True, timeout=5
            )
        except (subprocess.TimeoutExpired, OSError):
            continue
        if proc.stdout.strip().startswith("9."):
            return cand
    return None


_TCLSH9 = _find_tcl9()
pytestmark = pytest.mark.skipif(_TCLSH9 is None, reason="tclsh9.0 not available")


def _run(source: str) -> str:
    assert _TCLSH9 is not None
    proc = subprocess.run(
        [_TCLSH9], input=source, capture_output=True, text=True, timeout=15
    )
    assert proc.returncode == 0, f"tclsh failed: {proc.stderr}"
    return proc.stdout


@pytest.fixture(scope="module")
def sample_source() -> str:
    return _SAMPLE.read_text()


def test_sample_runs_on_tcl9(sample_source: str) -> None:
    assert _run(sample_source) == _EXPECTED_OUTPUT


def test_optimiser_cascade_is_deep_and_varied(sample_source: str) -> None:
    configure_signatures(dialect="tcl9.0")
    _optimised, opts, iterations = optimise_source_multipass(sample_source, max_iterations=15)
    codes = Counter(o.code for o in opts)
    # A genuine multi-pass cascade, not a single sweep.
    assert iterations >= 5, f"expected a deep cascade, got {iterations} iterations"
    # Several *different* optimisation kinds chained together.
    assert len(codes) >= 5, f"expected varied optimisations, got {dict(codes)}"
    # Interprocedural folding of pure proc calls must be part of it.
    assert codes.get("O103", 0) >= 3, f"expected interprocedural folds, got {dict(codes)}"


def test_optimised_form_is_equivalent_on_tcl9(sample_source: str) -> None:
    configure_signatures(dialect="tcl9.0")
    optimised, _opts, _iters = optimise_source_multipass(sample_source, max_iterations=15)
    assert optimised != sample_source, "optimiser made no changes"
    assert _run(optimised) == _EXPECTED_OUTPUT


def test_formatted_form_is_equivalent_on_tcl9(sample_source: str) -> None:
    formatted = format_tcl(sample_source)
    assert _run(formatted) == _EXPECTED_OUTPUT


def test_minified_form_is_equivalent_on_tcl9(sample_source: str) -> None:
    configure_signatures(dialect="tcl9.0")
    minified = minify_tcl(sample_source, dialect="tcl9.0")
    assert isinstance(minified, str)
    assert _run(minified) == _EXPECTED_OUTPUT


def test_optimise_then_minify_is_equivalent_on_tcl9(sample_source: str) -> None:
    # The tools must compose: optimised output is still valid input to the
    # minifier, and the doubly-transformed program still behaves identically.
    configure_signatures(dialect="tcl9.0")
    optimised, _opts, _iters = optimise_source_multipass(sample_source, max_iterations=15)
    minified = minify_tcl(optimised, dialect="tcl9.0")
    assert isinstance(minified, str)
    assert _run(minified) == _EXPECTED_OUTPUT
