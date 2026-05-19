"""Shared compilation artefacts for a single source document.

Built once per diagnostics cycle, consumed by the analyser, optimiser,
shimmer analysis, and compiler checks.
"""

from __future__ import annotations

import logging
import time
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
from .ir import IRBarrier, IRCall, IRModule, IRStatement
from .lowering import lower_to_ir
from .ssa import SSAFunction, build_ssa

_oo_metaclass_cache: frozenset[str] | None = None


def _oo_metaclasses() -> frozenset[str]:
    global _oo_metaclass_cache
    if _oo_metaclass_cache is None:
        from compiler.registry import REGISTRY

        _oo_metaclass_cache = REGISTRY.check_trait_commands("is_oo_metaclass")
    return _oo_metaclass_cache


def _extract_class_names(ir_module: IRModule) -> frozenset[str]:
    """Extract user-defined TclOO class names from IR statements.

    Scans ``oo::class create ClassName`` (and similar metaclass) patterns
    in the top-level script and procedure bodies.
    """
    names: set[str] = set()

    def _scan(stmts: tuple[IRStatement, ...]) -> None:
        for stmt in stmts:
            cmd: str = ""
            args: tuple[str, ...] = ()
            if isinstance(stmt, IRCall):
                cmd, args = stmt.command, stmt.args
            elif isinstance(stmt, IRBarrier):
                cmd, args = stmt.command, stmt.args
            if (
                cmd in _oo_metaclasses()
                and len(args) >= 2
                and args[0] in ("create", "createWithNamespace")
            ):
                class_name = args[1]
                names.add(f"::{class_name}" if not class_name.startswith("::") else class_name)

    _scan(ir_module.top_level.statements)
    for proc in ir_module.procedures.values():
        _scan(proc.body.statements)
    return frozenset(names)


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
    known_classes: frozenset[str] = frozenset(),
    deep_param_traits: bool = False,
) -> CompilationUnit | None:
    """Return a usable ``CompilationUnit`` by reusing or compiling.

    This is the canonical adapter for pass entry points that accept
    ``source`` and an optional pre-built ``CompilationUnit``.

    Set *deep_param_traits* for offline analytics paths (``tcl
    callgraph``, the compiler explorer, the MCP server) to opt into
    the deeper ``infer_param_traits_deep`` pass that descends into
    nested script bodies.  The LSP synchronous path leaves this off
    so per-keystroke analysis stays bounded.
    """
    if cu is not None:
        return cu
    try:
        return compile_source(
            source, known_classes=known_classes, deep_param_traits=deep_param_traits
        )
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
    stub_fingerprint: int = 0,
) -> tuple[str, int] | None:
    """Build a procedure cache key from source offsets.

    The fingerprint covers the active stub overlay because cached
    summaries depend on how role-aware lookups resolve ``ArgRole.BODY``
    / ``ArgRole.EXPR`` for the commands the proc invokes — adding,
    removing, or changing a stub must invalidate cached summaries
    even when the proc body text is unchanged.
    """
    if start_offset < 0 or end_offset < start_offset or end_offset > len(source):
        return None
    return (qname, hash((source[start_offset:end_offset], stub_fingerprint)))


def compute_stub_fingerprint(source: str) -> int:
    """Return the stub-overlay fingerprint for *source*.

    Exposed so cache builders outside :func:`compile_source` (e.g. the
    LSP workspace's ``_build_proc_cache``) can produce keys that match
    those :func:`compile_source` will look up.  Returns ``0`` when the
    source declares no stubs — the empty-overlay case stays compatible
    with callers that omit the fingerprint argument.
    """
    from analyser.stub_comments import scan_source_for_stubs
    from compiler.registry.runtime import signatures_from_stubs

    cmd_stubs, _ = scan_source_for_stubs(source)
    if not cmd_stubs:
        return 0
    overlay = signatures_from_stubs(cmd_stubs)
    return hash(tuple(sorted(overlay.keys()))) ^ hash(
        tuple(sorted(repr(v) for v in overlay.values()))
    )


def compile_source(
    source: str,
    *,
    ir_module: IRModule | None = None,
    proc_cache: dict[tuple[str, int], FunctionUnit] | None = None,
    interproc_cache: dict[tuple[str, int], ProcLocalSummary] | None = None,
    prune_interproc_cache: bool = True,
    known_classes: frozenset[str] = frozenset(),
    deep_param_traits: bool = False,
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
    from analyser.stub_comments import scan_source_for_stubs
    from compiler.registry.runtime import stub_signature_scope

    cmd_stubs, _ = scan_source_for_stubs(source)
    # Fingerprint the stub overlay so cached proc / interproc summaries
    # are invalidated whenever a stub is added, removed, or changed —
    # the proc body text alone is not enough because summaries depend
    # on the role-aware command lookups the overlay drives.
    stub_fingerprint = compute_stub_fingerprint(source)
    with stub_signature_scope(cmd_stubs):
        return _compile_source_inner(
            source,
            ir_module=ir_module,
            proc_cache=proc_cache,
            interproc_cache=interproc_cache,
            prune_interproc_cache=prune_interproc_cache,
            known_classes=known_classes,
            stub_fingerprint=stub_fingerprint,
            deep_param_traits=deep_param_traits,
        )


def _compile_source_inner(
    source: str,
    *,
    ir_module: IRModule | None,
    proc_cache: dict[tuple[str, int], FunctionUnit] | None,
    interproc_cache: dict[tuple[str, int], ProcLocalSummary] | None,
    prune_interproc_cache: bool,
    known_classes: frozenset[str],
    stub_fingerprint: int = 0,
    deep_param_traits: bool = False,
) -> CompilationUnit:
    if ir_module is None:
        ir_module = lower_to_ir(source)

    # P8.2: specialise Option-shape factory calls at every literal-
    # args call site.  Runs before CFG / SSA so the synthesised
    # child procs participate in the rest of the pipeline just like
    # any other IR-lowered proc.  Pass is a no-op when no factories
    # are detected — amortised cost is a single proc walk.
    from .passes.specialise_factories import specialise_factories

    specialise_factories(ir_module)

    # Insert IRInterpBoundary markers before each IRBarrier so the
    # frame-sync at every interpreter-crossing statement lives in IR
    # rather than as scattered ``_emit_frame_sync()`` calls in
    # codegen.  Pure-additive transformation; codegen short-circuits
    # on the new node and dispatches to ``_emit_interp_boundary``.
    # Idempotent — the pass skips insertion when an
    # ``IRInterpBoundary`` already precedes the ``IRBarrier``, so
    # ``compile_source(..., ir_module=cu.ir_module)`` re-runs
    # don't accumulate duplicate boundaries.  Run once here.
    from .passes.interp_boundaries import insert_interp_boundaries

    ir_module = insert_interp_boundaries(ir_module)

    # Extract TclOO class names from the IR so type propagation can
    # recognise ``[ClassName new]`` as returning an OBJECT instance.
    if not known_classes:
        known_classes = _extract_class_names(ir_module)

    upvar_procs, all_proc_params = prepare_cfg_context(ir_module)
    top_cfg = build_cfg_function(
        "::top",
        ir_module.top_level,
        upvar_procs=upvar_procs,
        proc_params=all_proc_params,
    )
    top_ssa = build_ssa(top_cfg)
    top_analysis = analyse_function(top_cfg, top_ssa, known_classes=known_classes)
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
            stub_fingerprint=stub_fingerprint,
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
        analysis = analyse_function(cfg, ssa, params=proc_params, known_classes=known_classes)
        proc_units[qname] = FunctionUnit(
            cfg=cfg,
            ssa=ssa,
            analysis=analysis,
            execution_intent=build_function_execution_intent(cfg),
        )
        time.sleep(0)  # Yield GIL between procedures

    cfg_module = CFGModule(top_level=top_cfg, procedures=proc_cfgs)

    interproc = analyse_interprocedural_ir(
        ir_module,
        source=source,
        proc_local_cache=interproc_cache,
        prune_local_cache=prune_interproc_cache,
        proc_units={qname: (fu.cfg, fu.ssa, fu.analysis) for qname, fu in proc_units.items()},
        stub_fingerprint=stub_fingerprint,
        deep_param_traits=deep_param_traits,
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
