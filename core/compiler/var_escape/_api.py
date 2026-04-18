"""Public entry point for the var-escape analysis."""

from __future__ import annotations

import logging

from ..compilation_unit import CompilationUnit, ensure_compilation_unit
from ..ir import IRModule
from ._propagation import analyse_script
from ._types import ProcEscapeSummary

log = logging.getLogger(__name__)

TOP_LEVEL_QNAME = "::top"


def analyse_var_escape(
    source: str | None = None,
    cu: CompilationUnit | None = None,
    *,
    ir_module: IRModule | None = None,
) -> dict[str, ProcEscapeSummary]:
    """Return per-proc escape summaries, keyed by qualified name.

    The top-level script is keyed as :data:`TOP_LEVEL_QNAME`. Callers
    that only care about proc bodies can filter it out.

    Exactly one of ``source``, ``cu``, or ``ir_module`` must be supplied.
    ``ir_module`` is the cheapest path — the analysis is a pure tree walk
    over already-lowered IR and does not need CFG / SSA.
    """
    if ir_module is None:
        if cu is None:
            if source is None:
                raise ValueError("analyse_var_escape requires source, cu, or ir_module")
            cu = ensure_compilation_unit(source, cu, logger=log, context="var_escape")
        if cu is None:
            return {}
        ir_module = cu.ir_module

    result: dict[str, ProcEscapeSummary] = {}
    result[TOP_LEVEL_QNAME] = analyse_script(ir_module.top_level)
    for qname, proc in ir_module.procedures.items():
        result[qname] = analyse_script(proc.body, params=proc.params)
    return result
