"""Public entry point for the var-escape analysis."""

from __future__ import annotations

import logging

from ..compilation_unit import CompilationUnit, ensure_compilation_unit
from ..ir import IRModule
from ..ssa import SSAValueKey
from ._cfg_propagation import CfgEscapeResult, analyse_cfg_function
from ._interprocedural import solve_interprocedural_escape
from ._propagation import analyse_script
from ._types import EscapeTag, ProcEscapeSummary

log = logging.getLogger(__name__)

TOP_LEVEL_QNAME = "::top"


def _cfg_result_to_summary(result: CfgEscapeResult) -> ProcEscapeSummary:
    """Convert the CFG+SSA pass result to a per-name summary.

    The per-SSA-version tags are collapsed by "FRAME wins" — a var
    is ``FRAME`` if any of its SSA versions was marked ``FRAME``.
    Per-version detail is preserved in the returned
    ``ProcEscapeSummary.ssa_tags`` field for consumers that can
    use it.
    """
    frame_needed = result.dynamic_barrier or any(
        tag is EscapeTag.FRAME for tag in result.name_tags.values()
    )
    return ProcEscapeSummary(
        tags=dict(result.name_tags),
        dynamic_barrier=result.dynamic_barrier,
        frame_needed=frame_needed,
        upvar_source_names=frozenset(result.upvar_source_names),
        unbounded_upvar_source=result.unbounded_upvar_source,
        direct_callees=frozenset(result.direct_callees),
        has_fallback=result.has_fallback,
        ssa_tags=dict(result.ssa_tags),
    )


def analyse_var_escape(
    source: str | None = None,
    cu: CompilationUnit | None = None,
    *,
    ir_module: IRModule | None = None,
    interprocedural: bool = True,
) -> dict[str, ProcEscapeSummary]:
    """Return per-proc escape summaries, keyed by qualified name.

    The top-level script is keyed as :data:`TOP_LEVEL_QNAME`. Callers
    that only care about proc bodies can filter it out.

    Exactly one of ``source``, ``cu``, or ``ir_module`` must be
    supplied. When a ``CompilationUnit`` is available the analysis
    uses the flow-sensitive per-SSA-version propagation over CFG +
    SSA; otherwise it falls back to the tree walk over the IR module.
    Both paths converge on the same per-name result since codegen
    collapses SSA versions by "FRAME wins".

    When ``interprocedural`` is True (the default) callee-induced
    escapes are folded into each caller's summary. Set it False to
    inspect the raw per-proc result (useful for tests).
    """
    result: dict[str, ProcEscapeSummary] = {}

    if cu is None and source is not None:
        cu = ensure_compilation_unit(source, cu, logger=log, context="var_escape")

    if cu is not None:
        # Flow-sensitive CFG + SSA path — each proc has a pre-built
        # CFGFunction and SSAFunction in the compilation unit.
        top_result = analyse_cfg_function(cu.top_level.cfg, cu.top_level.ssa)
        result[TOP_LEVEL_QNAME] = _cfg_result_to_summary(top_result)
        for qname, fu in cu.procedures.items():
            ir_proc = cu.ir_module.procedures.get(qname)
            params = ir_proc.params if ir_proc is not None else ()
            proc_result = analyse_cfg_function(fu.cfg, fu.ssa, params=params)
            result[qname] = _cfg_result_to_summary(proc_result)
    else:
        if ir_module is None:
            raise ValueError("analyse_var_escape requires source, cu, or ir_module")
        # Fallback: IR-only tree walk.  The tree walk produces the
        # same per-name tags; it just can't populate ``ssa_tags``.
        result[TOP_LEVEL_QNAME] = analyse_script(ir_module.top_level)
        for qname, proc in ir_module.procedures.items():
            result[qname] = analyse_script(proc.body, params=proc.params)

    if interprocedural:
        result = solve_interprocedural_escape(result)
    return result


__all__ = [
    "SSAValueKey",
    "TOP_LEVEL_QNAME",
    "analyse_var_escape",
]
