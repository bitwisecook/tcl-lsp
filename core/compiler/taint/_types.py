"""Warning dataclasses for taint analysis diagnostics."""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

from ...analysis.semantic_model import Range

if TYPE_CHECKING:
    from ..ssa import SSAValueKey
    from ._lattice import ProcTaintSummary, TaintLattice


@dataclass(frozen=True, slots=True)
class TaintWarning:
    """Tainted data flowing into a dangerous sink."""

    range: Range
    variable: str
    sink_command: str
    code: str  # T100
    message: str


@dataclass(frozen=True, slots=True)
class _InterprocTaintResult:
    """Result of inter-procedural taint analysis."""

    top_taints: dict[SSAValueKey, TaintLattice]
    proc_taints: dict[str, dict[SSAValueKey, TaintLattice]]
    summaries: dict[str, ProcTaintSummary]
