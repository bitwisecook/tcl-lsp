"""Shared compilation artefacts for a single source document.

Built once per diagnostics cycle, consumed by the analyser, optimiser,
shimmer analysis, and compiler checks.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass

from .cfg import CFGFunction, CFGModule, build_cfg_function, prepare_cfg_context
from .connection_scope import ConnectionScope, build_connection_scope
from .core_analyses import FunctionAnalysis, analyse_function
from .execution_intent import FunctionExecutionIntent, build_function_execution_intent
from .interprocedural import (
    InterproceduralAnalysis,
    ProcLocalSummary,
    analyse_interprocedural_ir,
)
from .ir import IRModule
from .lowering import lower_to_ir
from .ssa import SSAFunction, build_ssa


@dataclass(frozen=True, slots=True)
class FunctionUnit:
    """Pre-computed artefacts for a single function."""

    cfg: CFGFunction
    ssa: SSAFunction
    analysis: FunctionAnalysis
    execution_intent: FunctionExecutionIntent


@dataclass(frozen=True, slots=True)
class CompilationUnit:
    """Shared compilation artefacts for a single source document."""

    source: str
    ir_module: IRModule
    cfg_module: CFGModule
    top_level: FunctionUnit
    procedures: dict[str, FunctionUnit]
    interproc: InterproceduralAnalysis
    connection_scope: ConnectionScope | None = None


def ensure_compilation_unit(
    source: str,
    cu: CompilationUnit | None = None,
    *,
    logger: logging.Logger | None = None,
    context: str = "compiler",
    failure_detail: str = "compilation failed; continuing without CompilationUnit",
) -> CompilationUnit | None:
    """Return a usable ``CompilationUnit`` by reusing or compiling.

    This is the canonical adapter for pass entry points that accept
    ``source`` and an optional pre-built ``CompilationUnit``.
    """
    if cu is not None:
        return cu
    try:
        return compile_source(source)
    except Exception:
        if logger is not None:
            logger.debug(
                "%s: %s",
                context,
                failure_detail,
                exc_info=True,
            )
        return None


def _proc_cache_key(
    source: str,
    qname: str,
    start_offset: int,
    end_offset: int,
) -> tuple[str, int] | None:
    """Build a procedure cache key from source offsets."""
    if start_offset < 0 or end_offset < start_offset or end_offset > len(source):
        return None
    return (qname, hash(source[start_offset:end_offset]))


def compile_source(
    source: str,
    *,
    ir_module: IRModule | None = None,
    proc_cache: dict[tuple[str, int], FunctionUnit] | None = None,
    interproc_cache: dict[tuple[str, int], ProcLocalSummary] | None = None,
    prune_interproc_cache: bool = True,
) -> CompilationUnit:
    """Run the full pipeline once and return cached artefacts.

    When *ir_module* is provided, the lowering step is skipped and the
    pre-built IR is used directly.  This is the incremental path: the
    caller has already assembled the ``IRModule`` from cached and
    freshly-lowered chunk IR.

    When *proc_cache* is provided, procedure ``FunctionUnit`` values
    whose source text has not changed are reused instead of rebuilding
    SSA and dataflow analysis from scratch.  The cache key is
    ``(qualified_name, hash(procedure_source_text))``.

    When *interproc_cache* is provided, local interprocedural summaries
    are also reused for unchanged procedures using the same key shape.
    """
    if ir_module is None:
        ir_module = lower_to_ir(source)

    upvar_procs, all_proc_params = prepare_cfg_context(ir_module)
    top_cfg = build_cfg_function(
        "::top",
        ir_module.top_level,
        upvar_procs=upvar_procs,
        proc_params=all_proc_params,
    )
    top_ssa = build_ssa(top_cfg)
    top_analysis = analyse_function(top_cfg, top_ssa)
    top_unit = FunctionUnit(
        cfg=top_cfg,
        ssa=top_ssa,
        analysis=top_analysis,
        execution_intent=build_function_execution_intent(top_cfg),
    )

    proc_cfgs: dict[str, CFGFunction] = {}
    proc_units: dict[str, FunctionUnit] = {}
    for qname, ir_proc in ir_module.procedures.items():
        cache_key = _proc_cache_key(
            source,
            qname,
            ir_proc.range.start.offset,
            ir_proc.range.end.offset,
        )

        # Try the proc cache before rebuilding CFG + SSA + analysis.
        if proc_cache and cache_key is not None:
            cached = proc_cache.get(cache_key)
            if cached is not None:
                proc_units[qname] = cached
                proc_cfgs[qname] = cached.cfg
                continue

        cfg = build_cfg_function(
            qname,
            ir_proc.body,
            upvar_procs=upvar_procs,
            proc_params=all_proc_params,
        )
        proc_cfgs[qname] = cfg
        ssa = build_ssa(cfg)
        proc_params = frozenset(ir_proc.params)
        analysis = analyse_function(cfg, ssa, params=proc_params)
        proc_units[qname] = FunctionUnit(
            cfg=cfg,
            ssa=ssa,
            analysis=analysis,
            execution_intent=build_function_execution_intent(cfg),
        )

    cfg_module = CFGModule(top_level=top_cfg, procedures=proc_cfgs)

    interproc = analyse_interprocedural_ir(
        ir_module,
        source=source,
        proc_local_cache=interproc_cache,
        prune_local_cache=prune_interproc_cache,
        proc_units={qname: (fu.cfg, fu.ssa, fu.analysis) for qname, fu in proc_units.items()},
    )

    when_procs = {qn: fu for qn, fu in proc_units.items() if qn.startswith("::when::")}
    conn_scope = build_connection_scope(when_procs, ir_module) if when_procs else None

    return CompilationUnit(
        source=source,
        ir_module=ir_module,
        cfg_module=cfg_module,
        top_level=top_unit,
        procedures=proc_units,
        interproc=interproc,
        connection_scope=conn_scope,
    )
