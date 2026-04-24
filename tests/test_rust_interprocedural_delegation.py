"""Parity test for the Rust-delegated interprocedural summaries.

When ``TCL_LSP_RUST_INTERPROC=1`` is set and the ``tcl_lsp_rust``
wheel is installed, ``analyse_interprocedural_source`` delegates
to the Rust implementation and materialises its output back as
Python ``ProcSummary`` / ``InterproceduralAnalysis`` dataclasses.

This module verifies the core fields survive the round trip and
agree with the Python reference on representative shapes.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import core.compiler.interprocedural as _interproc_module
from core.compiler.interprocedural import analyse_interprocedural_source


def _rust_available() -> bool:
    return _interproc_module._rust_interprocedural_summaries is not None


def _with_rust(source: str):
    saved = os.environ.get("TCL_LSP_RUST_INTERPROC")
    os.environ["TCL_LSP_RUST_INTERPROC"] = "1"
    try:
        return analyse_interprocedural_source(source)
    finally:
        if saved is None:
            os.environ.pop("TCL_LSP_RUST_INTERPROC", None)
        else:
            os.environ["TCL_LSP_RUST_INTERPROC"] = saved


def _with_python(source: str):
    saved = os.environ.pop("TCL_LSP_RUST_INTERPROC", None)
    try:
        return analyse_interprocedural_source(source)
    finally:
        if saved is not None:
            os.environ["TCL_LSP_RUST_INTERPROC"] = saved


@pytest.mark.skipif(not _rust_available(), reason="Rust wheel not installed")
def test_constant_return_proc_summary_matches_python() -> None:
    source = "proc ::answer {} { return 42 }"
    rust = _with_rust(source)
    python = _with_python(source)
    assert "::answer" in rust.procedures
    assert "::answer" in python.procedures
    r = rust.procedures["::answer"]
    p = python.procedures["::answer"]
    # Core fields that should match exactly.
    assert r.qualified_name == p.qualified_name
    assert r.params == p.params
    assert r.returns_constant == p.returns_constant == True  # noqa: E712
    assert r.constant_return == p.constant_return == 42
    assert r.pure == p.pure == True  # noqa: E712
    assert r.can_fold_static_calls == p.can_fold_static_calls == True  # noqa: E712


@pytest.mark.skipif(not _rust_available(), reason="Rust wheel not installed")
def test_impure_proc_not_foldable() -> None:
    """A proc with an ``eval`` barrier must not be foldable."""
    source = "proc ::f {} { eval {} ; return 42 }"
    rust = _with_rust(source)
    r = rust.procedures["::f"]
    assert r.can_fold_static_calls is False


@pytest.mark.skipif(not _rust_available(), reason="Rust wheel not installed")
def test_rust_fallback_to_python_on_failure(monkeypatch) -> None:
    """If the Rust call raises unexpectedly, the Python fallback
    must still produce a valid analysis."""

    def _boom(_src, _dialect):
        raise RuntimeError("synthetic failure")

    monkeypatch.setattr(_interproc_module, "_rust_interprocedural_summaries", _boom, raising=False)
    monkeypatch.setenv("TCL_LSP_RUST_INTERPROC", "1")
    ia = analyse_interprocedural_source("proc ::answer {} { return 1 }")
    assert "::answer" in ia.procedures
    assert ia.procedures["::answer"].returns_constant is True


@pytest.mark.skipif(not _rust_available(), reason="Rust wheel not installed")
def test_gate_is_opt_in_default_stays_python(monkeypatch) -> None:
    """Without the env var, analysis must stay on the Python path.

    Verified via a spy that wraps the Rust binding and counts calls —
    an observable signal that delegation did not occur regardless of
    whether the dataclass fields happen to coincide.
    """
    rust_calls = 0
    original_rust = _interproc_module._rust_interprocedural_summaries
    assert original_rust is not None, "Rust wheel must be installed for this test"

    def _spy(src, dialect):
        nonlocal rust_calls
        rust_calls += 1
        return original_rust(src, dialect)

    monkeypatch.setattr(_interproc_module, "_rust_interprocedural_summaries", _spy, raising=False)
    monkeypatch.delenv("TCL_LSP_RUST_INTERPROC", raising=False)

    ia = analyse_interprocedural_source("proc ::f {x} { if {$x > 0} { return 1 } ; return 0 }")
    assert "::f" in ia.procedures
    assert rust_calls == 0, "Rust binding must not be invoked when gate is off"
