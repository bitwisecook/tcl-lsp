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

from ..commands.registry.runtime import (
    SIGNATURES,
    ArgRole,
    BodyKind,
    arg_indices_for_role,
    body_kind_for_command,
    scope_alias_commands,
)
from ..common.naming import normalise_var_name as _normalise_var_name
from .cfg import (
    CFGBranch,
    CFGFunction,
    CFGGoto,
    CFGReturn,
    CFGTerminator,
    _defs_from_ir_script,
)
from .expr_ast import ExprNode, vars_in_expr_node
from .ir import (
    CommandTokens,
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRCatch,
    IRExprEval,
    IRFor,
    IRForeach,
    IRIf,
    IRIncr,
    IRReturn,
    IRScript,
    IRStatement,
    IRSwitch,
    IRTry,
    IRWhile,
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
    # Registry-driven barrier defs:
    #
    # * ``ArgRole.VAR_WRITE`` — the named arg may later be mutated by a
    #   callback the command registers (``trace add variable name ...``
    #   wires ``name`` to the trace body).  Scope-alias commands
    #   (``global``, ``variable``, ``upvar``) are excluded: their
    #   variable bindings are tracked separately via ``creates_scope_alias``
    #   /``_collect_upvar_targets`` and their single-position ``VAR_WRITE``
    #   role would only mark arg 0 of vararg forms like
    #   ``global ?name ...?`` anyway.  Gated on ``SIGNATURES`` so a role
    #   hint loaded from a previously active dialect (e.g. EDA's
    #   ``foreach_in_collection`` after the user switches back to plain
    #   Tcl) doesn't leak into SSA's def-tracking via the cross-dialect
    #   ``_ROLE_HINTS`` fallback.
    # * ``ArgRole.LOOP_VAR_LIST`` — split into individual iteration
    #   variables (``dict for {k v} ...``).
    if isinstance(stmt, IRBarrier):
        defs: list[str] = []
        if stmt.command in SIGNATURES and stmt.command not in scope_alias_commands():
            for idx in arg_indices_for_role(stmt.command, list(stmt.args), ArgRole.VAR_WRITE):
                if 0 <= idx < len(stmt.args):
                    name = _normalise_var_name(stmt.args[idx])
                    if name:
                        defs.append(name)
        for idx in arg_indices_for_role(stmt.command, list(stmt.args), ArgRole.LOOP_VAR_LIST):
            if 0 <= idx < len(stmt.args):
                defs.extend(stmt.args[idx].split())
        if defs:
            return tuple(defs)
    return ()


def _vars_in_expr(expr: ExprNode) -> set[str]:
    return vars_in_expr_node(expr)


_VAR_REF_SCANNER = VarReferenceScanner(
    VarScanOptions(
        include_var_read_roles=True,
        recurse_cmd_substitutions=True,
    )
)

_DEEP_VAR_REF_SCANNER = VarReferenceScanner(
    VarScanOptions(
        include_var_read_roles=True,
        recurse_cmd_substitutions=True,
        recurse_into_script_roles=True,
    )
)


def _vars_in_word(text: str) -> frozenset[str]:
    return _VAR_REF_SCANNER.scan_word(text)


def _vars_in_script(source: str) -> frozenset[str]:
    return _VAR_REF_SCANNER.scan_script(source)


def _vars_in_body_script(source: str) -> frozenset[str]:
    """Scan a body script and recurse into nested braced bodies.

    Used for body-taking barriers (``dict for``/``dict map``) whose body
    isn't lowered into the CFG — variable references inside nested
    ``if``/``while``/``foreach`` braces would otherwise be missed.
    """
    return _DEEP_VAR_REF_SCANNER.scan_script(source)


def _is_braced_arg(tokens: CommandTokens | None, arg_index: int) -> bool:
    """Return True when argument *arg_index* is a braced literal (STR token).

    When token info is unavailable, conservatively returns True so that
    the body is still excluded (the common case is braced scripts).
    """
    if tokens is None:
        return True
    # tokens.argv includes the command name at index 0; args are 1-based.
    tok_index = arg_index + 1
    if tok_index >= len(tokens.argv):
        return True
    from ..parsing.tokens import TokenType

    return tokens.argv[tok_index].type is TokenType.STR


def _structural_body_indices(
    command: str,
    args: tuple[str, ...],
    tokens: CommandTokens | None = None,
) -> set[int]:
    """Return BODY arg indices that should be excluded from local statement uses.

    We only exclude handler-style bodies that are lowered/analysed separately
    — the registry marks these via :class:`BodyKind.STRUCTURAL` on the source
    spec (``proc``, ``when``, ``tcltest::test``, …).  Dynamic-evaluation
    commands like ``eval`` keep their args as ordinary dataflow inputs (for
    taint and read-before-set tracking) because their body kind is
    :class:`BodyKind.INLINE`.

    To avoid dropping real top-level reads when the body is passed via
    substitution (e.g. ``-body $script``), we only exclude arguments that
    are literal/braced script words.  Non-literal body arguments are still
    scanned for substitutions.
    """
    if body_kind_for_command(command, list(args)) is not BodyKind.STRUCTURAL:
        return set()
    candidate_indices = arg_indices_for_role(command, list(args), ArgRole.BODY)
    return {
        idx for idx in candidate_indices if 0 <= idx < len(args) and _is_braced_arg(tokens, idx)
    }


def _switch_reads(stmt: IRSwitch) -> set[str]:
    """Collect every variable read by a non-lowered ``switch``.

    ``-glob`` and fallthrough ``-exact`` switches are kept as an
    ``IRSwitch`` statement rather than lowered into CFG branches/blocks
    (their shared-body topology can't be expressed as structured control
    flow).  The subject word and the arm/default bodies therefore never
    contribute reads to the enclosing function's dataflow, so we gather
    them here — otherwise a parameter read only as a switch subject or
    only inside an arm body would be reported unused (W214) and similar
    false positives would fire (issue #471).
    """
    reads: set[str] = set()
    reads |= _vars_in_word(stmt.subject)
    for arm in stmt.arms:
        reads |= _vars_in_word(arm.pattern)
        if arm.body is not None:
            reads |= _free_reads_in_ir_script(arm.body)
    if stmt.default_body is not None:
        reads |= _free_reads_in_ir_script(stmt.default_body)
    return reads


def _free_reads_in_ir_script(script: IRScript) -> set[str]:
    """Reads of variables a collapsed body consumes from the *outer* scope.

    An un-lowered switch arm contributes its reads to the single enclosing
    ``IRSwitch`` statement, which has no way to represent the arm's own
    defs.  Subtracting the body's defs keeps arm-local temporaries
    (``set tmp 1; puts $tmp``) from being seen as outer reads — otherwise
    they would surface as false read-before-set (W210).  Genuine outer
    reads (a parameter used as ``return $text``) survive because they are
    never assigned inside the arm.
    """
    return _reads_in_ir_script(script) - set(_defs_from_ir_script(script))


def _reads_in_ir_script(script: IRScript) -> set[str]:
    """Recursively collect variable reads from an un-lowered IR script."""
    reads: set[str] = set()
    for stmt in script.statements:
        reads |= _reads_in_ir_stmt(stmt)
    return reads


def _reads_in_ir_stmt(stmt: IRStatement) -> set[str]:
    """Variable reads of a single statement, recursing into nested bodies.

    Leaf reads come from :func:`_uses` (which already resolves a nested
    ``IRSwitch`` via :func:`_switch_reads`); structured statements (``if``,
    loops, ``catch``/``try``) are not lowered when they live inside an
    un-lowered switch arm, so their conditions and sub-scripts must be
    walked here.
    """
    reads = set(_uses(stmt))
    match stmt:
        case IRIf(clauses=clauses, else_body=else_body):
            for clause in clauses:
                reads |= _vars_in_expr(clause.condition)
                reads |= _reads_in_ir_script(clause.body)
            if else_body is not None:
                reads |= _reads_in_ir_script(else_body)
        case IRWhile(condition=condition, body=body):
            reads |= _vars_in_expr(condition)
            reads |= _reads_in_ir_script(body)
        case IRFor(init=init, condition=condition, next=next_clause, body=body):
            reads |= _reads_in_ir_script(init)
            reads |= _vars_in_expr(condition)
            reads |= _reads_in_ir_script(next_clause)
            reads |= _reads_in_ir_script(body)
        case IRForeach(iterators=iterators, body=body):
            for _var_names, list_arg in iterators:
                reads |= _vars_in_word(list_arg)
            reads |= _reads_in_ir_script(body)
        case IRCatch(body=body):
            reads |= _reads_in_ir_script(body)
        case IRTry(body=body, handlers=handlers, finally_body=finally_body):
            reads |= _reads_in_ir_script(body)
            for handler in handlers:
                reads |= _reads_in_ir_script(handler.body)
            if finally_body is not None:
                reads |= _reads_in_ir_script(finally_body)
    return reads


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
            command=command,
            args=args,
            defs=call_defs,
            reads=call_reads,
            reads_own_defs=rod,
            tokens=call_tokens,
        ):
            vars_found |= _vars_in_word(command)
            body_indices = _structural_body_indices(command, args, call_tokens)
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
        case IRSwitch():
            vars_found |= _switch_reads(stmt)
        case IRReturn(value=value, expr=expr):
            if value is not None:
                vars_found |= _vars_in_word(value)
            if expr is not None:
                vars_found |= _vars_in_expr(expr)
        case IRBarrier(command=command, args=args, tokens=barrier_tokens):
            vars_found |= _vars_in_word(command)
            body_indices = _structural_body_indices(command, args, barrier_tokens)
            # Inline-body indices on a barrier carry a script that never
            # enters the CFG (e.g. ``dict for``/``dict map`` lowered as
            # ``::tcl::dict::for``/``::tcl::dict::map``).  We must
            # discover variable references inside such bodies here,
            # recursing into nested braced bodies; otherwise references
            # like ``$count`` inside ``dict for {k v} $d {... $count ...}``
            # would be invisible to the enclosing function's analysis
            # (false W214 / W210 — see issues #234, #236).
            inline_body_indices = {
                idx
                for idx in arg_indices_for_role(command, list(args), ArgRole.BODY)
                if 0 <= idx < len(args) and idx not in body_indices
            }
            for idx, arg in enumerate(args):
                if idx in body_indices:
                    continue
                if idx in inline_body_indices:
                    vars_found |= _vars_in_body_script(arg)
                else:
                    vars_found |= _vars_in_word(arg)
            # For ``dict with`` / ``dict update`` the dict-variable argument
            # (``args[1]`` of the full barrier args — ``args[0]`` is the
            # subcommand word) is both read and written: the body sees the
            # keys unpacked into local variables of the same name.  The
            # variable name is a plain string, not a ``$``-substitution, so
            # ``_vars_in_word`` misses it.  The registry's resolver attaches
            # both ``ArgRole.VAR_READ`` and ``ArgRole.VAR_WRITE`` to that
            # position, and ``arg_indices_for_role`` returns indices into
            # the full ``args`` list (subcommand offset already applied), so
            # a plain ``VAR_READ`` query yields the right slot.  Mark the
            # resulting name as ``reads_own_def`` so the final filter
            # doesn't drop it on the grounds that it's also a barrier def
            # (issue #307).
            for idx in arg_indices_for_role(command, list(args), ArgRole.VAR_READ):
                if 0 <= idx < len(args):
                    name = _normalise_var_name(args[idx])
                    if name:
                        vars_found.add(name)
                        reads_own_def.add(name)
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


def value_use_blocks(ssa: SSAFunction) -> dict[SSAValueKey, set[BlockName]]:
    """Map each SSA value ``(name, version)`` to the blocks that read it.

    A reader is any block whose phi nodes take that version on an incoming
    edge, or whose statements use it.  Forward dataflow worklists
    (SCCP, type / rendered-property / taint propagation) use this to
    re-enqueue exactly the blocks affected when a value's lattice entry
    changes, instead of re-scanning every block on every fixpoint pass.

    Terminator (branch/return) uses are not included — passes that read
    values in a terminator add those dependencies themselves.
    """
    deps: dict[SSAValueKey, set[BlockName]] = {}
    for bn, block in ssa.blocks.items():
        for phi in block.phis:
            for inc_ver in phi.incoming.values():
                if inc_ver > 0:
                    deps.setdefault((phi.name, inc_ver), set()).add(bn)
        for s in block.statements:
            for name, ver in s.uses.items():
                deps.setdefault((name, ver), set()).add(bn)
    return deps


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


def _compute_idom_fast(
    cfg: CFGFunction,
    reachable: set[str],
    preds: dict[str, set[str]],
) -> dict[str, str | None]:
    """Immediate dominators via Cooper-Harvey-Kennedy.

    Computes idom directly over reverse-postorder indices with the
    walk-up ``intersect`` — O(N) memory, no per-block dominator sets.
    The naive set-based fixpoint (:func:`_dominators` /
    :func:`_immediate_dominators`, retained for cross-validation) is
    O(N²) memory / ~O(N³) time and OOMs on a single huge generated proc
    that lowers to tens of thousands of CFG blocks.

    Result shape matches the set-based path exactly: entry → ``None``,
    unreachable → ``None``, otherwise the immediate-dominator block.

    Reference: ``compute_idom_fast`` in ``rust/tcl-compiler/src/ssa.rs``.
    """
    out: dict[str, str | None] = {bn: None for bn in cfg.blocks}
    # Pure reachable RPO (entry == index 0): reverse_postorder lists
    # reachable blocks first in true RPO, so filtering drops only the
    # unreachable tail.
    rpo = [bn for bn in cfg.reverse_postorder() if bn in reachable]
    if not rpo:
        return out
    rpo_index = {bn: i for i, bn in enumerate(rpo)}
    undef = -1
    idom = [undef] * len(rpo)
    idom[0] = 0  # entry dominates itself (sentinel for the walk-up)

    def intersect(a: int, b: int) -> int:
        # Walk both nodes up the idom tree until they meet; a dominator
        # always has a strictly smaller RPO index.
        while a != b:
            while a > b:
                a = idom[a]
            while b > a:
                b = idom[b]
        return a

    changed = True
    while changed:
        changed = False
        # Process in RPO order (skipping the entry) so each block sees
        # already-processed predecessors.
        for i in range(1, len(rpo)):
            new_idom = undef
            for p in preds.get(rpo[i], set()):
                pi = rpo_index.get(p)
                if pi is None or idom[pi] == undef:
                    continue  # unreachable / not yet processed this pass
                new_idom = pi if new_idom == undef else intersect(pi, new_idom)
            if new_idom != undef and idom[i] != new_idom:
                idom[i] = new_idom
                changed = True

    for i, bn in enumerate(rpo):
        out[bn] = None if i == 0 or idom[i] == undef else rpo[idom[i]]
    return out


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
        # The immediate dominator is the strict dominator closest to bn
        # in the dominator tree — equivalently, the one with the largest
        # dominator set (since dominators form a chain from entry to bn).
        # Using max(|dom[dc]|) is O(|strict|) instead of the previous
        # O(|strict|²) nested membership test.
        idom[bn] = max(strict, key=lambda dc: len(dom[dc]))
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
    idom = _compute_idom_fast(cfg, reachable, preds)
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

    # Rename walk over the dominator tree.  Iterative with an explicit
    # stack rather than recursion: a long block chain ⇒ deep dominator
    # tree, which a recursive descent overflows at the common 2 MB
    # worker-thread stack.  Each frame is processed in two phases —
    # ``_ENTER`` does the per-block work (phis, statements, successor phi
    # edges), then ``_CHILDREN`` walks the dominator-tree children and,
    # once exhausted, pops the versions this block pushed.
    _ENTER, _CHILDREN = 0, 1
    work: list[list] = []
    if cfg.entry in cfg.blocks:
        work.append([cfg.entry, 0, [], _ENTER])

    while work:
        frame = work[-1]
        bn = frame[0]
        if frame[3] == _ENTER:
            pushed_in_block: list[str] = frame[2]

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

            frame[3] = _CHILDREN
        else:
            children = tree.get(bn, [])
            if frame[1] < len(children):
                child = children[frame[1]]
                frame[1] += 1
                work.append([child, 0, [], _ENTER])
            else:
                for var in reversed(frame[2]):
                    stacks[var].pop()
                    if not stacks[var]:
                        del stacks[var]
                work.pop()

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
