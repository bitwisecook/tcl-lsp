"""Static Single-Assignment (SSA) construction over CFG blocks.

SSA is a variable-naming discipline where every variable is assigned
exactly once.  When control flow merges (e.g. after an ``if``), a
synthetic *phi node* is inserted to select the correct version of a
variable depending on which predecessor block was executed.

This module:

1. Computes **dominators** and the **dominance frontier** for each
   CFG block (needed to decide where phi nodes go).
2. Places phi nodes using the iterated-dominance-frontier algorithm.
3. Renames every variable definition and use so that each definition
   produces a unique ``(name, version)`` pair.

The resulting ``SSAFunction`` is consumed by SCCP and liveness in
``core_analyses``.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TypeAlias

from ..commands.registry.runtime import ArgRole, arg_indices_for_role
from ..common.naming import normalise_var_name as _normalise_var_name
from .cfg import CFGBranch, CFGFunction, CFGGoto, CFGReturn, CFGTerminator
from .expr_ast import ExprNode, vars_in_expr_node
from .ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRExprEval,
    IRIncr,
    IRReturn,
    IRStatement,
)
from .var_refs import VarReferenceScanner, VarScanOptions

# Semantic type aliases

SSAVersion: TypeAlias = int
"""SSA version number — each definition of a variable gets a unique version."""

BlockName: TypeAlias = str
"""Identifier for a CFG basic block (e.g. ``'entry'``, ``'if_true_0'``)."""

SSAValueKey: TypeAlias = tuple[str, SSAVersion]
"""Key identifying a specific SSA value: ``(variable_name, version)``."""

# Short names: bn = block name, dc = dominator candidate,
# nb = block popped from worklist, fb = frontier block,
# vn = SSA version number, p = predecessor block,
# v = variable name in comprehensions, s = SSAStatement.


def _successors(term: CFGTerminator | None) -> tuple[str, ...]:
    match term:
        case CFGGoto(target=target):
            return (target,)
        case CFGBranch(true_target=tt, false_target=ft):
            return (tt, ft)
        case CFGReturn() | None:
            return ()
        case _:
            return ()


def _defs(stmt: IRStatement) -> tuple[str, ...]:
    if isinstance(stmt, (IRAssignConst, IRAssignExpr, IRAssignValue, IRIncr)):
        return (_normalise_var_name(stmt.name),)
    if isinstance(stmt, IRCall) and stmt.defs:
        return stmt.defs
    if isinstance(stmt, IRBarrier) and stmt.command == "trace" and len(stmt.args) >= 3:
        if stmt.args[0] == "add" and stmt.args[1] == "variable":
            return (_normalise_var_name(stmt.args[2]),)
    return ()


def _vars_in_expr(expr: ExprNode) -> set[str]:
    return vars_in_expr_node(expr)


_VAR_REF_SCANNER = VarReferenceScanner(
    VarScanOptions(
        include_var_read_roles=True,
        recurse_cmd_substitutions=True,
    )
)


def _vars_in_word(text: str) -> set[str]:
    return _VAR_REF_SCANNER.scan_word(text)


def _vars_in_script(source: str) -> set[str]:
    return _VAR_REF_SCANNER.scan_script(source)


def _structural_body_indices(command: str, args: tuple[str, ...]) -> set[int]:
    """Return BODY arg indices that should be excluded from local statement uses.

    We only exclude handler-style bodies that are lowered/analyzed separately.
    Dynamic evaluation commands like ``eval`` still need their args treated as
    ordinary dataflow inputs (for taint and read-before-set tracking).
    """
    if command in ("when", "proc"):
        return arg_indices_for_role(command, list(args), ArgRole.BODY)
    return set()


def _uses(stmt: IRStatement) -> tuple[str, ...]:
    vars_found: set[str] = set()
    reads_own_def: set[str] = set()

    match stmt:
        case IRExprEval(expr=expr):
            vars_found |= _vars_in_expr(expr)
        case IRAssignExpr(expr=expr):
            vars_found |= _vars_in_expr(expr)
            reads_own_def = vars_found & {_normalise_var_name(stmt.name)}
        case IRAssignValue(value=value):
            vars_found |= _vars_in_word(value)
            reads_own_def = vars_found & {_normalise_var_name(stmt.name)}
        case IRIncr(name=raw_name, amount=amount):
            name = _normalise_var_name(raw_name)
            if name:
                vars_found.add(name)
                reads_own_def.add(name)
            if amount is not None:
                vars_found |= _vars_in_word(amount)
        case IRCall(
            command=command, args=args, defs=call_defs, reads=call_reads, reads_own_defs=rod
        ):
            vars_found |= _vars_in_word(command)
            body_indices = _structural_body_indices(command, args)
            for idx, arg in enumerate(args):
                if idx in body_indices:
                    continue
                vars_found |= _vars_in_word(arg)
            if call_reads:
                for name in call_reads:
                    if name:
                        vars_found.add(name)
            if rod and call_defs:
                for name in call_defs:
                    vars_found.add(name)
                    reads_own_def.add(name)
        case IRReturn(value=value) if value is not None:
            vars_found |= _vars_in_word(value)
        case IRBarrier(command=command, args=args):
            vars_found |= _vars_in_word(command)
            body_indices = _structural_body_indices(command, args)
            for idx, arg in enumerate(args):
                if idx in body_indices:
                    continue
                vars_found |= _vars_in_word(arg)
        case _:
            pass

    defs = set(_defs(stmt))
    return tuple(sorted(v for v in vars_found if v and (v not in defs or v in reads_own_def)))


@dataclass(frozen=True, slots=True)
class SSAPhi:
    """A phi node merging variable versions at a control-flow join.

    ``incoming`` maps each predecessor block name to the variable
    version that flows in from that edge.
    """

    name: str
    version: SSAVersion
    incoming: dict[BlockName, SSAVersion]


@dataclass(frozen=True, slots=True)
class SSAStatement:
    """An IR statement annotated with SSA version numbers.

    ``uses`` maps each variable name read by the statement to the
    SSA version in scope.  ``defs`` maps each variable name written
    to its newly assigned version.
    """

    statement: IRStatement
    uses: dict[str, SSAVersion]
    defs: dict[str, SSAVersion]


@dataclass(frozen=True, slots=True)
class SSABlock:
    """A CFG basic block in SSA form.

    ``entry_versions`` / ``exit_versions`` record which SSA version
    of each variable is live at the start and end of the block.
    """

    name: BlockName
    phis: tuple[SSAPhi, ...]
    statements: tuple[SSAStatement, ...]
    entry_versions: dict[str, SSAVersion]
    exit_versions: dict[str, SSAVersion]


@dataclass(frozen=True, slots=True)
class SSAFunction:
    """Complete SSA representation of one Tcl procedure or top-level script.

    Includes the dominator tree and dominance frontier so that
    downstream passes (SCCP, liveness) do not need to recompute them.
    """

    name: str
    entry: BlockName
    blocks: dict[BlockName, SSABlock]
    idom: dict[BlockName, BlockName | None]
    dominance_frontier: dict[BlockName, tuple[BlockName, ...]]
    dominator_tree: dict[BlockName, tuple[BlockName, ...]]


def _reachable_blocks(cfg: CFGFunction) -> set[str]:
    seen: set[str] = set()
    stack = [cfg.entry]
    while stack:
        bn = stack.pop()
        if bn in seen or bn not in cfg.blocks:
            continue
        seen.add(bn)
        stack.extend(_successors(cfg.blocks[bn].terminator))
    return seen


def _predecessors(cfg: CFGFunction) -> dict[str, set[str]]:
    preds: dict[str, set[str]] = {bn: set() for bn in cfg.blocks}
    for bn, block in cfg.blocks.items():
        for succ in _successors(block.terminator):
            if succ in preds:
                preds[succ].add(bn)
    return preds


def _dominators(
    cfg: CFGFunction, reachable: set[str], preds: dict[str, set[str]]
) -> dict[str, set[str]]:
    dom: dict[str, set[str]] = {}
    for bn in cfg.blocks:
        if bn not in reachable:
            dom[bn] = {bn}
        elif bn == cfg.entry:
            dom[bn] = {bn}
        else:
            dom[bn] = set(reachable)

    changed = True
    while changed:
        changed = False
        for bn in reachable:
            if bn == cfg.entry:
                continue
            bn_preds = [p for p in preds.get(bn, set()) if p in reachable]
            if not bn_preds:
                new_dom = {bn}
            else:
                pred_dom = set(dom[bn_preds[0]])
                for p in bn_preds[1:]:
                    pred_dom &= dom[p]
                new_dom = pred_dom | {bn}
            if new_dom != dom[bn]:
                dom[bn] = new_dom
                changed = True
    return dom


def _immediate_dominators(
    cfg: CFGFunction,
    reachable: set[str],
    dom: dict[str, set[str]],
) -> dict[str, str | None]:
    idom: dict[str, str | None] = {bn: None for bn in cfg.blocks}
    idom[cfg.entry] = None

    for bn in reachable:
        if bn == cfg.entry:
            continue
        strict = dom[bn] - {bn}
        if not strict:
            idom[bn] = None
            continue
        candidate: str | None = None
        for dc in strict:
            dominated_by_other = any(dc in dom[o] for o in strict if o != dc)
            if not dominated_by_other:
                candidate = dc
                break
        idom[bn] = candidate
    return idom


def _dominance_frontier(
    cfg: CFGFunction,
    reachable: set[str],
    preds: dict[str, set[str]],
    idom: dict[str, str | None],
) -> dict[str, set[str]]:
    df: dict[str, set[str]] = {bn: set() for bn in cfg.blocks}
    for bn in reachable:
        bn_preds = [p for p in preds.get(bn, set()) if p in reachable]
        if len(bn_preds) < 2:
            continue
        for p in bn_preds:
            runner = p
            while runner is not None and runner != idom.get(bn):
                df[runner].add(bn)
                runner = idom.get(runner)
    return df


def _dom_tree(idom: dict[str, str | None]) -> dict[str, list[str]]:
    tree: dict[str, list[str]] = {bn: [] for bn in idom}
    for bn, parent in idom.items():
        if parent is not None:
            tree[parent].append(bn)
    for children in tree.values():
        children.sort()
    return tree


def _phi_vars(
    cfg: CFGFunction,
    reachable: set[str],
    df: dict[str, set[str]],
) -> dict[str, set[str]]:
    defsites: dict[str, set[str]] = {}
    for bn in reachable:
        block = cfg.blocks[bn]
        for stmt in block.statements:
            for var in _defs(stmt):
                defsites.setdefault(var, set()).add(bn)

    phi: dict[str, set[str]] = {bn: set() for bn in cfg.blocks}
    for var, sites in defsites.items():
        work = list(sorted(sites))
        has_phi: set[str] = set()
        while work:
            nb = work.pop()
            for fb in df.get(nb, set()):
                if fb not in has_phi:
                    phi[fb].add(var)
                    has_phi.add(fb)
                    if fb not in sites:
                        work.append(fb)
    return phi


def build_ssa(cfg: CFGFunction) -> SSAFunction:
    """Build SSA with dominator-based phi placement and renaming."""
    reachable = _reachable_blocks(cfg)
    preds = _predecessors(cfg)
    dom = _dominators(cfg, reachable, preds)
    idom = _immediate_dominators(cfg, reachable, dom)
    df = _dominance_frontier(cfg, reachable, preds, idom)
    tree = _dom_tree(idom)
    phi_vars = _phi_vars(cfg, reachable, df)

    version_counter: dict[str, int] = {}
    stacks: dict[str, list[int]] = {}

    def top(var: str) -> int:
        st = stacks.get(var, [])
        return st[-1] if st else 0

    def push_new(var: str) -> int:
        vn = version_counter.get(var, 0) + 1
        version_counter[var] = vn
        stacks.setdefault(var, []).append(vn)
        return vn

    phi_versions: dict[str, dict[str, int]] = {bn: {} for bn in cfg.blocks}
    phi_incoming: dict[str, dict[str, dict[str, int]]] = {bn: {} for bn in cfg.blocks}
    entry_versions: dict[str, dict[str, int]] = {bn: {} for bn in cfg.blocks}
    exit_versions: dict[str, dict[str, int]] = {bn: {} for bn in cfg.blocks}
    stmt_infos: dict[str, list[SSAStatement]] = {bn: [] for bn in cfg.blocks}

    def rename(bn: str) -> None:
        pushed_in_block: list[str] = []

        for var in sorted(phi_vars.get(bn, set())):
            ver = push_new(var)
            pushed_in_block.append(var)
            phi_versions[bn][var] = ver
            phi_incoming[bn].setdefault(var, {})

        visible_vars = set(stacks.keys()) | set(phi_versions[bn].keys())
        entry_versions[bn] = {v: top(v) for v in sorted(visible_vars) if top(v) > 0}

        for stmt in cfg.blocks[bn].statements:
            uses_map: dict[str, int] = {}
            for var in _uses(stmt):
                uses_map[var] = top(var)

            defs_map: dict[str, int] = {}
            for var in _defs(stmt):
                ver = push_new(var)
                pushed_in_block.append(var)
                defs_map[var] = ver

            stmt_infos[bn].append(
                SSAStatement(
                    statement=stmt,
                    uses=uses_map,
                    defs=defs_map,
                )
            )

        visible_vars = set(stacks.keys()) | set(phi_versions[bn].keys())
        exit_versions[bn] = {v: top(v) for v in sorted(visible_vars) if top(v) > 0}

        for succ in _successors(cfg.blocks[bn].terminator):
            if succ not in cfg.blocks:
                continue
            for var in sorted(phi_vars.get(succ, set())):
                phi_incoming[succ].setdefault(var, {})
                phi_incoming[succ][var][bn] = top(var)

        for child in tree.get(bn, []):
            rename(child)

        for var in reversed(pushed_in_block):
            stacks[var].pop()
            if not stacks[var]:
                del stacks[var]

    if cfg.entry in cfg.blocks:
        rename(cfg.entry)

    ssa_blocks: dict[str, SSABlock] = {}
    for bn, block in cfg.blocks.items():
        phis: list[SSAPhi] = []
        for var in sorted(phi_vars.get(bn, set())):
            phis.append(
                SSAPhi(
                    name=var,
                    version=phi_versions.get(bn, {}).get(var, 0),
                    incoming=dict(phi_incoming.get(bn, {}).get(var, {})),
                )
            )
        ssa_blocks[bn] = SSABlock(
            name=bn,
            phis=tuple(phis),
            statements=tuple(stmt_infos[bn]),
            entry_versions=dict(entry_versions[bn]),
            exit_versions=dict(exit_versions[bn]),
        )

    return SSAFunction(
        name=cfg.name,
        entry=cfg.entry,
        blocks=ssa_blocks,
        idom=idom,
        dominance_frontier={bn: tuple(sorted(v)) for bn, v in df.items()},
        dominator_tree={bn: tuple(children) for bn, children in tree.items()},
    )
