"""Shared proc lookup helpers for symbol-oriented LSP features."""

from __future__ import annotations

from .semantic_model import AnalysisResult, ProcDef


def iter_procs_by_reference(
    analysis: AnalysisResult,
    ref: str,
) -> list[tuple[str, ProcDef]]:
    """Return all procs whose names match *ref* in supported reference forms."""
    matches: list[tuple[str, ProcDef]] = []
    for qname, proc_def in analysis.all_procs.items():
        if proc_def.name == ref or qname == ref or qname == f"::{ref}":
            matches.append((qname, proc_def))
    return matches


def find_proc_by_reference(
    analysis: AnalysisResult,
    ref: str,
) -> tuple[str, ProcDef] | None:
    """Return the first proc matching *ref* in supported reference forms."""
    matches = iter_procs_by_reference(analysis, ref)
    if not matches:
        return None
    return matches[0]
