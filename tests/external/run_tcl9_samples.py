"""Compile-and-run the ``samples/tcl9_smoke/`` corpus.

Each `.tcl` file under ``samples/tcl9_smoke/`` is a self-contained
primitive-level program with a sibling `.expected` file containing
its expected stdout.  We compile each to WASM with the same
pipeline the main test harness uses, run it under wasmtime, and
assert the captured stdout matches byte-for-byte.

Purpose: a tcltest-independent smoke corpus.  Because the samples
do not use ``tcltest``, a regression caught here points directly
at the primitive the commit broke, without the noise of the
tcltest harness's own compile / dispatch path.  Complements the
tcltest-based ``run_tcl9_tests.py`` which groups by upstream test
file rather than by primitive.

When the expected output changes (Tcl 9 semantic update), regenerate
with ``scripts/tcl9_samples_refgen.py`` from a host that has
``tclsh9.0`` installed.  See ``samples/tcl9_smoke/README.md`` for
the workflow.
"""

from __future__ import annotations

import tempfile
from pathlib import Path

import pytest

from tests.test_wasm_real_tcl import _compile_tcl_with_diag, _run_wasm

_REPO_ROOT = Path(__file__).resolve().parents[2]
_SAMPLES_DIR = _REPO_ROOT / "samples" / "tcl9_smoke"


def _collect_samples() -> list[Path]:
    """Return every ``*.tcl`` under samples/tcl9_smoke/ that has a
    sibling ``.expected`` file.  Alphabetical order so failures land
    grouped by primitive."""
    if not _SAMPLES_DIR.is_dir():
        return []
    out: list[Path] = []
    for tcl in sorted(_SAMPLES_DIR.rglob("*.tcl")):
        expected = tcl.with_suffix(".expected")
        if expected.is_file():
            out.append(tcl)
    return out


_SAMPLES = _collect_samples()


@pytest.mark.parametrize(
    "sample",
    _SAMPLES,
    ids=[str(p.relative_to(_SAMPLES_DIR)) for p in _SAMPLES],
)
def test_sample_matches_expected(sample: Path) -> None:
    """Compile *sample* to WASM, run it, compare stdout to ``.expected``."""
    source = sample.read_text(encoding="utf-8")
    expected = sample.with_suffix(".expected").read_text(encoding="utf-8")

    wasm, _ = _compile_tcl_with_diag(source, filename=sample.name)
    with tempfile.TemporaryDirectory(prefix="tcl9-sample-") as host_tmp:
        result = _run_wasm(
            wasm,
            capture_stdout=True,
            capture_stderr=True,
            preopen_tmpdir=host_tmp,
        )
    stdout = result[1] if len(result) >= 2 else ""
    assert stdout == expected, (
        f"{sample.relative_to(_SAMPLES_DIR)} stdout mismatch\n"
        f"--- expected\n{expected!r}\n"
        f"--- actual\n{stdout!r}\n"
    )
