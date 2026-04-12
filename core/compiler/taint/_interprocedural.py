"""Inter-procedural taint summary solver."""

from __future__ import annotations

from collections import deque

from ..cfg import CFGBranch, CFGFunction, CFGGoto, CFGReturn
from ..compilation_unit import CompilationUnit
from ..interprocedural import resolve_call_target
from ..ir import IRAssignValue, IRCall
from ..ssa import SSAFunction, SSAValueKey
from ..value_shapes import parse_command_substitution
from ._lattice import (
    _BASIS_LATTICES,
    _BASIS_ORDER,
    _UNTAINTED,
    ProcTaintSummary,
    TaintLattice,
    taint_join,
)
from ._propagation import (
    _CallReturnProvider,
    _evaluate_word_taint,
    _word_uses_from_versions,
    taint_propagation,
)
from ._types import _InterprocTaintResult


def _arity_from_params(params: tuple[str, ...]):  # noqa: ANN202
    from ...commands.registry.signatures import Arity

    if params and params[-1] == "args":
        return Arity(len(params) - 1)
    return Arity(len(params), len(params))


def _basis_names_for_taint(taint: TaintLattice) -> tuple[str, ...]:
    if not taint.tainted:
        return ()
    # Derive basis names from _BASIS_ORDER / _BASIS_LATTICES so that adding
    # a new colour+basis in _lattice.py is automatically picked up here.
    names: list[str] = [
        basis
        for basis in _BASIS_ORDER
        if basis != "generic" and bool(_BASIS_LATTICES[basis].colour & taint.colour)
    ]
    if not names:
        names.append("generic")
    return tuple(names)


def _summary_is_equal(a: ProcTaintSummary, b: ProcTaintSummary) -> bool:
    return (
        a.qualified_name == b.qualified_name
        and a.params == b.params
        and a.arity == b.arity
        and a.return_base == b.return_base
        and a.return_by_param_basis == b.return_by_param_basis
    )


def _apply_proc_return_summary(
    summary: ProcTaintSummary,
    arg_taints: tuple[TaintLattice, ...],
) -> TaintLattice:
    if not summary.arity.accepts(len(arg_taints)):
        return _UNTAINTED

    bound: dict[str, TaintLattice] = {}
    if summary.params and summary.params[-1] == "args":
        fixed = len(summary.params) - 1
        for i in range(fixed):
            p = summary.params[i]
            bound[p] = arg_taints[i] if i < len(arg_taints) else _UNTAINTED
        rest = _UNTAINTED
        for i in range(fixed, len(arg_taints)):
            rest = taint_join(rest, arg_taints[i])
        bound["args"] = rest
    else:
        for i, p in enumerate(summary.params):
            bound[p] = arg_taints[i] if i < len(arg_taints) else _UNTAINTED

    out = summary.return_base
    for p, t in bound.items():
        if not t.tainted:
            continue
        for basis in _basis_names_for_taint(t):
            out = taint_join(out, summary.scenario(p, basis))
    return out


def _make_call_return_provider(
    summaries: dict[str, ProcTaintSummary],
) -> _CallReturnProvider:
    known = set(summaries)

    def provider(
        command: str,
        args: tuple[str, ...],
        arg_taints: tuple[TaintLattice, ...],
        caller_qname: str | None,
    ) -> TaintLattice | None:
        caller = caller_qname or "::top"
        qname = resolve_call_target(command, args, caller, known)
        if qname is None:
            return None
        summary = summaries.get(qname)
        if summary is None:
            return None
        return _apply_proc_return_summary(summary, arg_taints)

    return provider


def _compute_executable_edges(
    cfg: CFGFunction,
    executable_blocks: set[str],
) -> set[tuple[str, str]]:
    edges: set[tuple[str, str]] = set()
    for bn in executable_blocks:
        block = cfg.blocks.get(bn)
        if block is None:
            continue
        match block.terminator:
            case CFGGoto(target=target):
                succs = (target,)
            case CFGBranch(true_target=tt, false_target=ft):
                succs = (tt, ft)
            case _:
                succs = ()
        for succ in succs:
            if succ in executable_blocks:
                edges.add((bn, succ))
    return edges


def _collect_return_taint(
    cfg: CFGFunction,
    ssa: SSAFunction,
    taints: dict[SSAValueKey, TaintLattice],
    executable_blocks: set[str],
    *,
    call_return_provider: _CallReturnProvider | None = None,
) -> TaintLattice:
    ret = _UNTAINTED
    for bn in executable_blocks:
        block = cfg.blocks.get(bn)
        if block is None:
            continue
        term = block.terminator
        if not isinstance(term, CFGReturn) or term.value is None:
            continue
        ssa_block = ssa.blocks.get(bn)
        if ssa_block is None:
            continue
        uses = _word_uses_from_versions(term.value, ssa_block.exit_versions)
        ret = taint_join(
            ret,
            _evaluate_word_taint(
                term.value,
                uses,
                taints,
                caller_qname=cfg.name,
                call_return_provider=call_return_provider,
            ),
        )
    return ret


def _infer_proc_summary(
    qname: str,
    params: tuple[str, ...],
    cfg: CFGFunction,
    ssa: SSAFunction,
    executable_blocks: set[str],
    executable_edges: set[tuple[str, str]],
    *,
    call_return_provider: _CallReturnProvider | None = None,
) -> ProcTaintSummary:
    base_taints = taint_propagation(
        cfg,
        ssa,
        executable_blocks,
        executable_edges,
        call_return_provider=call_return_provider,
    )
    return_base = _collect_return_taint(
        cfg,
        ssa,
        base_taints,
        executable_blocks,
        call_return_provider=call_return_provider,
    )

    by_param_basis: list[tuple[str, tuple[TaintLattice, ...]]] = []
    for param in params:
        scenario_values: list[TaintLattice] = []
        for basis in _BASIS_ORDER:
            seeded = taint_propagation(
                cfg,
                ssa,
                executable_blocks,
                executable_edges,
                param_taints={param: _BASIS_LATTICES[basis]},
                call_return_provider=call_return_provider,
            )
            scenario_values.append(
                _collect_return_taint(
                    cfg,
                    ssa,
                    seeded,
                    executable_blocks,
                    call_return_provider=call_return_provider,
                )
            )
        by_param_basis.append((param, tuple(scenario_values)))

    return ProcTaintSummary(
        qualified_name=qname,
        params=params,
        arity=_arity_from_params(params),
        return_base=return_base,
        return_by_param_basis=tuple(by_param_basis),
    )


def _resolve_call_flows_for_function(
    cfg: CFGFunction,
    ssa: SSAFunction,
    taints: dict[SSAValueKey, TaintLattice],
    executable_blocks: set[str],
    summaries: dict[str, ProcTaintSummary],
) -> list[tuple[str, tuple[TaintLattice, ...]]]:
    known = set(summaries)
    provider = _make_call_return_provider(summaries)
    flows: list[tuple[str, tuple[TaintLattice, ...]]] = []

    for bn in executable_blocks:
        block = cfg.blocks.get(bn)
        ssa_block = ssa.blocks.get(bn)
        if block is None or ssa_block is None:
            continue
        stmt_count = min(len(block.statements), len(ssa_block.statements))
        for idx in range(stmt_count):
            stmt = block.statements[idx]
            ssa_stmt = ssa_block.statements[idx]

            cmd_name: str | None = None
            cmd_args: tuple[str, ...] = ()
            if isinstance(stmt, IRCall):
                cmd_name = stmt.command
                cmd_args = stmt.args
            elif isinstance(stmt, IRAssignValue):
                parsed = parse_command_substitution(stmt.value)
                if parsed is not None:
                    cmd_name, cmd_args = parsed

            if cmd_name is None:
                continue

            callee = resolve_call_target(cmd_name, cmd_args, cfg.name, known)
            if callee is None:
                continue
            summary = summaries.get(callee)
            if summary is None or not summary.arity.accepts(len(cmd_args)):
                continue

            arg_taints = tuple(
                _evaluate_word_taint(
                    arg,
                    ssa_stmt.uses,
                    taints,
                    caller_qname=cfg.name,
                    call_return_provider=provider,
                )
                for arg in cmd_args
            )
            flows.append((callee, arg_taints))

    return flows


def _solve_interprocedural_taints(cu: CompilationUnit) -> _InterprocTaintResult:
    proc_units = cu.procedures

    proc_exec: dict[str, tuple[set[str], set[tuple[str, str]]]] = {}
    for qname, fu in proc_units.items():
        executable = set(fu.cfg.blocks) - fu.analysis.unreachable_blocks
        proc_exec[qname] = (
            executable,
            _compute_executable_edges(fu.cfg, executable),
        )

    summaries: dict[str, ProcTaintSummary] = {}
    for qname, proc in cu.ir_module.procedures.items():
        summaries[qname] = ProcTaintSummary(
            qualified_name=qname,
            params=proc.params,
            arity=_arity_from_params(proc.params),
            return_base=_UNTAINTED,
            return_by_param_basis=tuple(
                (p, tuple(_UNTAINTED for _ in _BASIS_ORDER)) for p in proc.params
            ),
        )

    changed = True
    while changed:
        changed = False
        provider = _make_call_return_provider(summaries)
        for qname, proc in cu.ir_module.procedures.items():
            fu = proc_units[qname]
            executable, edges = proc_exec[qname]
            inferred = _infer_proc_summary(
                qname,
                proc.params,
                fu.cfg,
                fu.ssa,
                executable,
                edges,
                call_return_provider=provider,
            )
            old = summaries[qname]
            if not _summary_is_equal(old, inferred):
                summaries[qname] = inferred
                changed = True

    provider = _make_call_return_provider(summaries)

    top_exec = set(cu.top_level.cfg.blocks) - cu.top_level.analysis.unreachable_blocks
    top_edges = _compute_executable_edges(cu.top_level.cfg, top_exec)
    top_taints = taint_propagation(
        cu.top_level.cfg,
        cu.top_level.ssa,
        top_exec,
        top_edges,
        call_return_provider=provider,
    )

    entry_taints: dict[str, dict[str, TaintLattice]] = {
        qname: {p: _UNTAINTED for p in proc.params}
        for qname, proc in cu.ir_module.procedures.items()
    }
    proc_taints: dict[str, dict[SSAValueKey, TaintLattice]] = {
        qname: {} for qname in cu.ir_module.procedures
    }

    initial_flows = _resolve_call_flows_for_function(
        cu.top_level.cfg,
        cu.top_level.ssa,
        top_taints,
        top_exec,
        summaries,
    )
    queue: deque[str] = deque(sorted(cu.ir_module.procedures))
    queued: set[str] = set(queue)

    def update_entry(callee: str, args: tuple[TaintLattice, ...]) -> bool:
        summary = summaries.get(callee)
        if summary is None:
            return False
        changed_local = False
        if summary.params and summary.params[-1] == "args":
            fixed = len(summary.params) - 1
            for i in range(fixed):
                param = summary.params[i]
                incoming = args[i] if i < len(args) else _UNTAINTED
                merged = taint_join(entry_taints[callee][param], incoming)
                if merged != entry_taints[callee][param]:
                    entry_taints[callee][param] = merged
                    changed_local = True
            rest = _UNTAINTED
            for i in range(fixed, len(args)):
                rest = taint_join(rest, args[i])
            merged = taint_join(entry_taints[callee]["args"], rest)
            if merged != entry_taints[callee]["args"]:
                entry_taints[callee]["args"] = merged
                changed_local = True
            return changed_local

        for i, param in enumerate(summary.params):
            incoming = args[i] if i < len(args) else _UNTAINTED
            merged = taint_join(entry_taints[callee][param], incoming)
            if merged != entry_taints[callee][param]:
                entry_taints[callee][param] = merged
                changed_local = True
        return changed_local

    for callee, args in initial_flows:
        if update_entry(callee, args) and callee not in queued:
            queue.append(callee)
            queued.add(callee)

    while queue:
        qname = queue.popleft()
        queued.discard(qname)
        fu = proc_units[qname]
        executable, edges = proc_exec[qname]
        taints = taint_propagation(
            fu.cfg,
            fu.ssa,
            executable,
            edges,
            param_taints=entry_taints[qname],
            call_return_provider=provider,
        )
        proc_taints[qname] = taints

        for callee, args in _resolve_call_flows_for_function(
            fu.cfg,
            fu.ssa,
            taints,
            executable,
            summaries,
        ):
            if update_entry(callee, args) and callee not in queued:
                queue.append(callee)
                queued.add(callee)

    return _InterprocTaintResult(
        top_taints=top_taints,
        proc_taints=proc_taints,
        summaries=summaries,
    )
