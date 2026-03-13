"""Static source optimiser for Tcl.

Current safe subset -- see individual pass modules for details:
    O100--O125 optimisation passes.
"""

from __future__ import annotations

from ._expr_simplify import demorgan_transform, invert_expression
from ._manager import apply_optimisations, find_optimisations, optimise_source
from ._types import Optimisation

__all__ = [
    "Optimisation",
    "apply_optimisations",
    "demorgan_transform",
    "find_optimisations",
    "invert_expression",
    "optimise_source",
]
