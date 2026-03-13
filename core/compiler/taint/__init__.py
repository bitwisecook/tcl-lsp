"""Taint analysis for untrusted I/O data.

Tracks data provenance through the SSA graph using a colour-aware
lattice.  See individual submodules for detailed documentation.
"""

from __future__ import annotations

from ._api import find_taint_warnings
from ._lattice import ProcTaintSummary, TaintLattice, taint_join
from ._propagation import taint_propagation
from ._types import (
    CollectWithoutReleaseWarning,
    ReleaseWithoutCollectWarning,
    TaintWarning,
)

__all__ = [
    "CollectWithoutReleaseWarning",
    "ProcTaintSummary",
    "ReleaseWithoutCollectWarning",
    "TaintLattice",
    "TaintWarning",
    "find_taint_warnings",
    "taint_join",
    "taint_propagation",
]
