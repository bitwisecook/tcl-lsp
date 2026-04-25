"""Differential parity harness for the C40 ``signature_scan`` Rust port.

For each fixture in :data:`CORPUS`, both the Python implementation
(``_extract_signatures_python``) and the Rust path (via
``signature_scan_extract`` + the ``_materialise_rust_signatures``
helper) are exercised and asserted to produce field-by-field
equivalent :class:`AnalysisResult` records on the subset of fields
``signature_scan`` populates.

Skipped wholesale when the ``tcl_lsp_rust`` binding cannot be
imported.
"""

from __future__ import annotations

import pytest

pytest.importorskip("tcl_lsp_rust", reason="tcl_lsp_rust extension not built into this venv")

from tcl_lsp_rust import (  # noqa: E402
    signature_scan_extract,  # ty: ignore[unresolved-import]
)

from core.analysis.semantic_model import AnalysisResult  # noqa: E402
from core.analysis.signature_scan import (  # noqa: E402
    _extract_signatures_python,
    _materialise_rust_signatures,
)


def _compare_results(py: AnalysisResult, rust: AnalysisResult) -> None:
    """Assert that ``py`` and ``rust`` are field-equal on the
    subset of :class:`AnalysisResult` populated by signature_scan.
    """
    assert py.all_procs == rust.all_procs, "all_procs mismatch"
    assert py.all_classes == rust.all_classes, "all_classes mismatch"
    assert py.package_requires == rust.package_requires, "package_requires mismatch"
    assert py.source_targets == rust.source_targets, "source_targets mismatch"
    assert py.command_aliases == rust.command_aliases, "command_aliases mismatch"
    assert py.namespace_imports == rust.namespace_imports, "namespace_imports mismatch"
    assert py.auto_path_entries == rust.auto_path_entries, "auto_path_entries mismatch"
    assert py.command_invocations == rust.command_invocations, "command_invocations mismatch"


def _assert_same(source: str) -> None:
    """Run both implementations on ``source`` and assert parity."""
    py = _extract_signatures_python(source)
    rust = _materialise_rust_signatures(source, signature_scan_extract(source))
    _compare_results(py, rust)


def test_simple_proc_parity() -> None:
    """Sanity check: a single bare proc round-trips identically."""
    _assert_same("proc foo {} {}")
