"""Thin orchestration layer for optimiser passes."""

from __future__ import annotations

import logging
import re

from ...common.naming import normalise_var_name as _NORMALISE
from ..cfg import CFGFunction
from ..compilation_unit import CompilationUnit, ensure_compilation_unit
from ..execution_intent import FunctionExecutionIntent
from ..interprocedural import InterproceduralAnalysis
from ..ir import IRAssignConst, IRBarrier, IRCall, IRModule, IRScript
from . import (
    _branch_folding,
    _code_sinking,
    _elimination,
    _pattern_recognition,
    _structure_elimination,
    _tail_call,
    _unused_procs,
)
from ._helpers import (
    _constants_from_uses,
    _namespace_from_qualified,
    _select_non_overlapping_optimisations,
    _tokens_for_statement,
)
from ._propagation import (
    optimise_constant_var_refs,
    optimise_expr_substitutions,
    optimise_expression_args,
    optimise_return_terminator,
    optimise_static_proc_calls,
    optimise_string_interpolation_var_refs,
)
from ._types import Optimisation, PassContext

log = logging.getLogger(__name__)


def _safe_string_constants(
    constants: dict[str, str],
    block,
    use_idx: int,
) -> dict[str, str]:
    """Return the subset of *constants* safe for string-interpolation propagation.

    String interpolation was previously not propagated, so existing tests rely
    on SCCP-unsound scenarios (upvar aliasing, catch exception paths) being
    harmless.  To avoid regressions we only propagate a constant into a string
    when the defining ``set`` (``IRAssignConst``) is in the **same basic block**
    and there is no intervening ``IRCall`` or ``IRBarrier`` between the
    definition and the use that could mutate the variable through ``upvar``
    or exception side-effects.
    """
    if not constants:
        return constants

    # Scan statements [0 .. use_idx) in this block to find the most recent
    # IRAssignConst for each constant and whether a call/barrier sits between
    # the def and the use.
    last_def_idx: dict[str, int] = {}
    call_or_barrier_indices: list[int] = []

    for si in range(use_idx):
        if si >= len(block.statements):
            break
        s = block.statements[si]
        if isinstance(s, IRAssignConst):
            name = _NORMALISE(s.name)
            if name in constants:
                last_def_idx[name] = si
        if isinstance(s, (IRCall, IRBarrier)):
            call_or_barrier_indices.append(si)

    if not last_def_idx:
        return {}

    safe: dict[str, str] = {}
    for name, def_si in last_def_idx.items():
        # Check that no call/barrier sits between the def and the use.
        blocked = False
        for ci in call_or_barrier_indices:
            if ci > def_si:
                blocked = True
                break
        if not blocked:
            safe[name] = constants[name]
    return safe


class _CompilerOptimiser:
    def __init__(self) -> None:
        self._interproc = InterproceduralAnalysis(procedures={})
        self._proc_cfgs: dict[str, tuple[CFGFunction, tuple[str, ...]]] = {}
        self._ir_module: IRModule | None = None

    def run(
        self,
        source: str,
        cu: CompilationUnit | None = None,
    ) -> list[Optimisation]:
        cu = ensure_compilation_unit(source, cu, logger=log, context="optimiser")
        if cu is None:
            return []
        self._ir_module = cu.ir_module

        self._interproc = cu.interproc
        for qname, fu in cu.procedures.items():
            ir_proc = cu.ir_module.procedures.get(qname)
            if ir_proc:
                self._proc_cfgs[qname] = (fu.cfg, ir_proc.params)

        ctx = PassContext(
            source=source,
            optimisations=[],
            interproc=self._interproc,
            proc_cfgs=self._proc_cfgs,
            propagated_branch_uses=set(),
            cross_event_vars=frozenset(),
            next_group=0,
            propagated_use_groups={},
            propagated_expr_stmts=set(),
            ir_module=self._ir_module,
        )

        self._process_function(
            ctx,
            cu.top_level.cfg,
            cu.top_level.ssa,
            cu.top_level.analysis,
            execution_intent=cu.top_level.execution_intent,
            ir_script=cu.ir_module.top_level,
            namespace="::",
            is_top_level=True,
        )
        conn = cu.connection_scope
        for qname, fu in cu.procedures.items():
            if conn is not None and qname.startswith("::when::"):
                ctx.cross_event_vars = conn.cross_event_defs | conn.cross_event_imports
            else:
                ctx.cross_event_vars = frozenset()
            ir_proc = cu.ir_module.procedures.get(qname)
            self._process_function(
                ctx,
                fu.cfg,
                fu.ssa,
                fu.analysis,
                execution_intent=fu.execution_intent,
                ir_script=ir_proc.body if ir_proc else None,
                namespace=_namespace_from_qualified(qname),
            )
        # Tail-call detection (cross-procedure, runs once).
        _tail_call.optimise_tail_calls(ctx)
        # Module-level passes
        _unused_procs.optimise_unused_procs(ctx)

        return ctx.optimisations

    def _process_function(
        self,
        ctx: PassContext,
        cfg,
        ssa,
        analysis,
        execution_intent: FunctionExecutionIntent,
        *,
        ir_script: IRScript | None = None,
        namespace: str = "::",
        is_top_level: bool = False,
    ) -> None:
        source = ctx.source
        ctx.propagated_branch_uses = set()
        ctx.propagated_expr_stmts = set()

        # Pattern recognition (pre-loop)
        _pattern_recognition.optimise_string_build_chains(ctx, cfg, ssa)
        _pattern_recognition.optimise_incr_idioms(ctx, cfg, ssa)
        _pattern_recognition.optimise_multi_set_packing(ctx, cfg, ssa)

        kill_sites: list[tuple[int, str]] = []
        for block in cfg.blocks.values():
            for stmt in block.statements:
                if not isinstance(stmt, IRCall):
                    continue
                stmt_range = getattr(stmt, "range", None)
                if stmt_range is None:
                    continue
                for name in stmt.defs:
                    if name:
                        kill_sites.append((stmt_range.start.offset, name))
        kill_sites.sort()

        # Statement loop (propagation)
        for block_name, block in cfg.blocks.items():
            ssa_block = ssa.blocks.get(block_name)
            if ssa_block is None:
                continue

            for idx, ssa_stmt in enumerate(ssa_block.statements):
                if idx < 0 or idx >= len(block.statements):
                    continue
                stmt = block.statements[idx]
                stmt_range = getattr(stmt, "range", None)
                if stmt_range is None:
                    continue

                parsed = _tokens_for_statement(stmt, source)
                if parsed is None:
                    continue
                argv_texts, argv_tokens, argv_single = parsed
                if not argv_texts:
                    continue

                cmd_name = argv_texts[0]
                args = argv_texts[1:]
                arg_tokens = argv_tokens[1:]
                arg_single = argv_single[1:]
                constants = _constants_from_uses(ssa_stmt.uses, analysis.values)
                stmt_start = stmt_range.start.offset
                if constants:
                    for kill_offset, var_name in kill_sites:
                        if kill_offset >= stmt_start:
                            break
                        constants.pop(var_name, None)

                n_before = len(ctx.optimisations)
                optimise_expression_args(
                    ctx,
                    cmd_name,
                    args,
                    arg_tokens,
                    arg_single,
                    constants,
                    namespace=namespace,
                    ssa_uses=ssa_stmt.uses,
                    types=analysis.types,
                )
                optimise_expr_substitutions(
                    ctx,
                    arg_tokens,
                    arg_single,
                    constants,
                    ssa_uses=ssa_stmt.uses,
                    types=analysis.types,
                )
                optimise_static_proc_calls(
                    ctx, arg_tokens, arg_single, constants, namespace=namespace
                )
                optimise_constant_var_refs(ctx, arg_tokens, arg_single, constants)
                # Track expression uses consumed by constant propagation/folding.
                if len(ctx.optimisations) > n_before:
                    propagation_codes = frozenset(("O100", "O101", "O102", "O103"))
                    new_opts = ctx.optimisations[n_before:]
                    if any(opt.code in propagation_codes for opt in new_opts):
                        group_id = ctx.alloc_group()
                        # Stamp group on the new optimisations.
                        for i in range(n_before, len(ctx.optimisations)):
                            opt = ctx.optimisations[i]
                            if opt.code in propagation_codes:
                                ctx.optimisations[i] = Optimisation(
                                    code=opt.code,
                                    message=opt.message,
                                    range=opt.range,
                                    replacement=opt.replacement,
                                    group=group_id,
                                )
                        ctx.propagated_expr_stmts.add((block_name, idx))
                        # Only mark variables whose $ref was actually replaced
                        # in the new optimisation.  Comparing original text with
                        # the replacement: a $name present in the original span
                        # but absent from the replacement was consumed.
                        for i in range(n_before, len(ctx.optimisations)):
                            opt = ctx.optimisations[i]
                            if opt.code not in propagation_codes:
                                continue
                            orig_span = source[opt.range.start.offset : opt.range.end.offset + 1]
                            for name in constants:
                                ver = ssa_stmt.uses.get(name, 0)
                                if ver <= 0:
                                    continue
                                ref = f"${name}"
                                if ref in orig_span and ref not in opt.replacement:
                                    ctx.propagated_branch_uses.add((name, ver))
                                    ctx.propagated_use_groups[(name, ver)] = group_id

                # Propagate constants into $var refs inside multi-token words
                # (string interpolation), e.g. puts "$x is the value".
                # Run AFTER the main group tracking to avoid marking
                # non-propagated vars as dead.  Uses a conservative subset of
                # constants: only those whose defining assignment is in this
                # same block with no intervening calls or barriers.
                ct = getattr(stmt, "tokens", None)
                if ct is not None and ct.all_tokens and constants:
                    safe_constants = _safe_string_constants(
                        constants,
                        block,
                        idx,
                    )
                    if safe_constants:
                        n_str = len(ctx.optimisations)
                        optimise_string_interpolation_var_refs(
                            ctx, arg_tokens, arg_single, safe_constants, ct.all_tokens
                        )
                        # Track only the actually-propagated vars for DSE.
                        if len(ctx.optimisations) > n_str:
                            str_group = ctx.alloc_group()
                            for i in range(n_str, len(ctx.optimisations)):
                                opt = ctx.optimisations[i]
                                ctx.optimisations[i] = Optimisation(
                                    code=opt.code,
                                    message=opt.message,
                                    range=opt.range,
                                    replacement=opt.replacement,
                                    group=str_group,
                                )
                            ctx.propagated_expr_stmts.add((block_name, idx))
                            for name in safe_constants:
                                ver = ssa_stmt.uses.get(name, 0)
                                if ver > 0:
                                    ctx.propagated_branch_uses.add((name, ver))
                                    ctx.propagated_use_groups[(name, ver)] = str_group

            optimise_return_terminator(ctx, source, block, ssa_block, analysis, namespace=namespace)

        # Branch folding
        _branch_folding.optimise_branch_proc_calls(ctx, cfg, ssa, analysis, namespace=namespace)
        _branch_folding.optimise_constant_branches(ctx, cfg, ssa, analysis)

        # Elimination
        _elimination.optimise_elimination_passes(
            ctx,
            cfg,
            ssa,
            analysis,
            execution_intent,
            is_top_level=is_top_level,
        )

        # Code sinking
        _code_sinking.optimise_code_sinking(ctx, ir_script)

        # Structure elimination
        _structure_elimination.optimise_structure_elimination(ctx, ir_script, cfg, ssa, analysis)


def find_optimisations(
    source: str,
    cu: CompilationUnit | None = None,
) -> list[Optimisation]:
    """Find static optimisation opportunities in source order."""
    selected = _select_non_overlapping_optimisations(
        _CompilerOptimiser().run(source, cu=cu),
    )

    # When O112 replaces a large range, inner O101/O112 optimisations get
    # dropped by overlap resolution.  Prevent O109/O126 from removing
    # definitions that are still textually referenced in an O112 replacement.
    var_refs_in_replacements: set[str] = set()
    for opt in selected:
        if opt.code == "O112" and opt.replacement:
            for m in re.finditer(r"\$(\w+)", opt.replacement):
                var_refs_in_replacements.add(m.group(1))
    if var_refs_in_replacements:
        filtered: list[Optimisation] = []
        for opt in selected:
            if opt.code in ("O109", "O126") and opt.replacement == "":
                text = source[opt.range.start.offset : opt.range.end.offset + 1].strip()
                parts = text.split(None, 2)
                if len(parts) >= 2 and parts[0] == "set" and parts[1] in var_refs_in_replacements:
                    continue
            filtered.append(opt)
        selected = filtered

    # Drop orphaned O125 parts: a grouped O125 rewrite needs both the
    # source comment and at least one insertion.  When overlap resolution
    # (e.g. O109 or O112) drops either half, the surviving half is a no-op
    # or misleading — remove incomplete groups.  Filter per group so that
    # one complete group does not mask a broken sibling.
    o125_groups: dict[int | None, dict[str, bool]] = {}
    for opt in selected:
        if opt.code != "O125":
            continue
        state = o125_groups.setdefault(opt.group, {"has_comment": False, "has_insert": False})
        if opt.message.startswith("Sink "):
            state["has_comment"] = True
        elif opt.message.startswith("Insert sunk"):
            state["has_insert"] = True
    bad_o125_groups = {
        gid
        for gid, state in o125_groups.items()
        if not (state["has_comment"] and state["has_insert"])
    }
    if bad_o125_groups:
        selected = [
            opt for opt in selected if not (opt.code == "O125" and opt.group in bad_o125_groups)
        ]

    return selected


def apply_optimisations(source: str, optimisations: list[Optimisation]) -> str:
    """Apply optimisation rewrites to source text.

    Delegates to :func:`core.common.text_edits.apply_edits` after
    converting ``Optimisation`` ranges to ``(offset, length, text)`` tuples.
    """
    from ...common.text_edits import apply_edits

    if not optimisations:
        return source

    edits: list[tuple[int, int, str]] = []
    for opt in optimisations:
        if opt.hint_only:
            continue
        start = opt.range.start.offset
        end_exclusive = opt.range.end.offset + 1
        if start < 0 or end_exclusive > len(source) or start > end_exclusive:
            continue
        edits.append((start, end_exclusive - start, opt.replacement))

    return apply_edits(source, edits)


def optimise_source(source: str) -> tuple[str, list[Optimisation]]:
    """Return optimised source and the optimisation rewrites used."""
    cu = ensure_compilation_unit(source, logger=log, context="optimise_source")
    optimisations = find_optimisations(source, cu=cu)
    return apply_optimisations(source, optimisations), optimisations
