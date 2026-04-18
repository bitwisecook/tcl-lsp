"""Per-proc static analysis: which Tcl variables must live in the runtime frame.

A variable is tagged ``FRAME`` if the analysis cannot prove that it is only
read or written through statically resolved positions in the proc body.
Otherwise it stays ``LOCAL`` and the WASM codegen keeps it in a WASM local
slot.

The public entry point is :func:`analyse_var_escape`, which consumes a
:class:`~core.compiler.compilation_unit.CompilationUnit` and produces a map
from qualified proc name to :class:`ProcEscapeSummary`.
"""

from __future__ import annotations

from ._api import TOP_LEVEL_QNAME, analyse_var_escape
from ._interprocedural import solve_interprocedural_escape
from ._types import EscapeTag, ProcEscapeSummary

__all__ = [
    "EscapeTag",
    "ProcEscapeSummary",
    "TOP_LEVEL_QNAME",
    "analyse_var_escape",
    "solve_interprocedural_escape",
]
