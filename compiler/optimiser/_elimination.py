"""Elimination passes (DCE, DSE, ADCE) for the optimiser."""

from __future__ import annotations

from shared.codes import opt

from ..cfg import CFGBranch, CFGReturn
from ..execution_intent import EscapeClass, FunctionExecutionIntent, SideEffectClass
from ..expr_ast import ExprNode, vars_in_expr_node
from ..ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRCall,
    IRIncr,
    IRStatement,
)
from ..parsing.command_segmenter import segment_commands
from ..parsing.lexer import TclLexer
from ..registry.runtime import REGISTRY
from ..side_effects import classify_side_effects
from ..var_refs import VarReferenceScanner
from ._expr_simplify import _expr_has_command_subst
from ._pattern_recognition import _statement_delete_rewrite_range, _statement_rewrite_context
from ._types import Optimisation, PassContext

_RETURN_VAR_SCANNER = VarReferenceScanner()


def _word_has_observable_side_effect(text: str) -> bool:
    """True when *text* (a Tcl word body) contains a command substitution
    that has an observable side effect (writes a variable, prints to
    stdout, mutates global state, runs a dynamic barrier, …).

    Used to gate elimination of unused / dead assignments: ``set v
    [puts X]`` discards the result but the call still prints, so the
    assignment is NOT safe to delete.

    Conservative: any command we can't classify (unknown user proc,
    dynamic dispatch) is treated as having side effects -- deletion is
    only allowed when every embedded command is provably side-effect-
    free.  (D2-O126 closure: pre-fix, O126 deleted any unused-result
    assignment, losing observable behaviour for any command-sub RHS.)
    """
    if "[" not in text:
        return False
    from shared.tokens import TokenType

    try:
        tokens = TclLexer(text).tokenise_all()
    except Exception:
        return True  # unparseable -> conservative
    for tok in tokens:
        if tok.type is not TokenType.CMD:
            continue
        # Parse the embedded command to get name + args.
        try:
            cmds = segment_commands(tok.text)
        except Exception:
            return True
        if len(cmds) != 1 or not cmds[0].texts:
            # Multi-command substitution or empty -- conservative.
            return True
        cmd_name = cmds[0].texts[0]
        cmd_args = tuple(cmds[0].texts[1:])
        se = classify_side_effects(cmd_name, cmd_args)
        if not se.pure:
            return True
        # Recurse into nested substitutions inside the args.
        for arg in cmd_args:
            if _word_has_observable_side_effect(arg):
                return True
    return False


def _expr_has_observable_side_effect(node: ExprNode) -> bool:
    """Expr-tree analogue of :func:`_word_has_observable_side_effect` --
    True if any embedded command substitution in the expression has
    an observable side effect."""
    from ..expr_ast import (
        ExprBinary,
        ExprCall,
        ExprCommand,
        ExprRaw,
        ExprTernary,
        ExprUnary,
    )

    match node:
        case ExprCommand(text=text) | ExprRaw(text=text):
            return _word_has_observable_side_effect(text)
        case ExprBinary(left=left, right=right):
            return _expr_has_observable_side_effect(left) or _expr_has_observable_side_effect(right)
        case ExprUnary(operand=operand):
            return _expr_has_observable_side_effect(operand)
        case ExprTernary(condition=cond, true_branch=tb, false_branch=fb):
            return (
                _expr_has_observable_side_effect(cond)
                or _expr_has_observable_side_effect(tb)
                or _expr_has_observable_side_effect(fb)
            )
        case ExprCall(args=args):
            return any(_expr_has_observable_side_effect(a) for a in args)
        case _:
            return False


def _assignment_safe_to_delete(stmt: IRStatement) -> bool:
    """True when *stmt* is an assignment whose RHS can be discarded
    without losing observable behaviour.  ``IRAssignConst`` is always
    safe (literal RHS); other forms require every embedded command
    substitution to be classified as pure."""
    if isinstance(stmt, IRAssignConst):
        return True
    if isinstance(stmt, IRAssignValue):
        return not _word_has_observable_side_effect(stmt.value)
    if isinstance(stmt, IRAssignExpr):
        return not _expr_has_observable_side_effect(stmt.expr)
    if isinstance(stmt, IRIncr):
        # ``incr v`` reads + writes ``v`` -- the assignment itself IS the
        # observable side effect.  Eliminating it is OK only when ``v``
        # is dead and the optional amount word has no side effects.
        if stmt.amount is None:
            return True
        return not _word_has_observable_side_effect(stmt.amount)
    # Unknown statement form -- conservative.
    return False


# Keep ``REGISTRY`` importable from this module (avoids re-imports below).
_ = REGISTRY

# O-code registrations for codes primarily emitted from this module
opt(code="O107", description="Eliminate unreachable dead code.", opt_category="dce")
opt(code="O108", description="Eliminate transitively dead code.", opt_category="dce")
opt(code="O109", description="Eliminate dead stores.", opt_category="dce")
opt(
    code="O126",
    description=(
        "Remove unused variable assignments — eliminate `set` statements for variables "
        "that are never read."
    ),
    opt_category="dce",
)


def _is_adce_removable_statement(
    stmt,
    *,
    stmt_key: tuple[str, int] | None = None,
    execution_intent: FunctionExecutionIntent | None = None,
) -> bool:
    match stmt:
        case IRAssignConst():
            return True
        case IRAssignValue(value=value):
            if execution_intent is not None and stmt_key is not None:
                intent = execution_intent.command_substitutions.get(stmt_key)
                if intent is None:
                    return True
                return (
                    intent.side_effect is SideEffectClass.PURE
                    and intent.escape is EscapeClass.NO_ESCAPE
                )
            return "[" not in value
        case IRAssignExpr(expr=expr):
            return not _expr_has_command_subst(expr)
        case IRIncr(amount=amount):
            return amount is None or "[" not in amount
        case _:
            return False


def _return_use_versions(term, exit_versions: dict[str, int]) -> set[tuple[str, int]]:
    if not isinstance(term, CFGReturn):
        return set()
    uses: set[tuple[str, int]] = set()
    if term.value is not None:
        for name in _RETURN_VAR_SCANNER.scan_script(term.value):
            ver = exit_versions.get(name, 0)
            if ver > 0:
                uses.add((name, ver))
    if term.expr is not None:
        for name in vars_in_expr_node(term.expr):
            ver = exit_versions.get(name, 0)
            if ver > 0:
                uses.add((name, ver))
    return uses


def _branch_use_versions(term, exit_versions: dict[str, int]) -> set[tuple[str, int]]:
    if not isinstance(term, CFGBranch):
        return set()
    uses: set[tuple[str, int]] = set()
    for name in vars_in_expr_node(term.condition):
        ver = exit_versions.get(name, 0)
        if ver > 0:
            uses.add((name, ver))
    return uses


def _collect_adce_statement_keys(
    ctx: PassContext,
    cfg,
    ssa,
    analysis,
    execution_intent: FunctionExecutionIntent,
    *,
    baseline_dse_keys: set[tuple[str, int]],
) -> list[tuple[str, int]]:
    executable_blocks = set(cfg.blocks) - set(analysis.unreachable_blocks)

    def_to_stmt: dict[tuple[str, int], tuple[str, int]] = {}
    stmt_defs: dict[tuple[str, int], set[tuple[str, int]]] = {}
    stmt_uses: dict[tuple[str, int], set[tuple[str, int]]] = {}
    removable_stmt_keys: set[tuple[str, int]] = set()
    def_counts: dict[str, int] = {}
    keep_consumer = ("<keep>", -1)
    consumers: dict[tuple[str, int], set[tuple[str, int]]] = {}

    for block_name in executable_blocks:
        block = cfg.blocks.get(block_name)
        ssa_block = ssa.blocks.get(block_name)
        if block is None or ssa_block is None:
            continue

        for phi in ssa_block.phis:
            for pred, incoming_ver in phi.incoming.items():
                if incoming_ver <= 0 or pred not in executable_blocks:
                    continue
                consumers.setdefault((phi.name, incoming_ver), set()).add(keep_consumer)

        for idx, ssa_stmt in enumerate(ssa_block.statements):
            if idx < 0 or idx >= len(block.statements):
                continue
            stmt = block.statements[idx]
            stmt_range = getattr(stmt, "range", None)
            if stmt_range is None:
                continue
            stmt_key = (block_name, idx)
            uses = {(name, ver) for name, ver in ssa_stmt.uses.items() if ver > 0}
            stmt_uses[stmt_key] = uses
            for use_key in uses:
                consumers.setdefault(use_key, set()).add(stmt_key)

            defs = {(name, ver) for name, ver in ssa_stmt.defs.items() if ver > 0}
            stmt_defs[stmt_key] = defs
            for key in defs:
                def_to_stmt[key] = stmt_key
                def_counts[key[0]] = def_counts.get(key[0], 0) + 1

            if _is_adce_removable_statement(
                stmt,
                stmt_key=stmt_key,
                execution_intent=execution_intent,
            ):
                removable_stmt_keys.add(stmt_key)

        for use_key in _branch_use_versions(block.terminator, ssa_block.exit_versions):
            if use_key not in ctx.propagated_branch_uses:
                consumers.setdefault(use_key, set()).add(keep_consumer)
        for use_key in _return_use_versions(block.terminator, ssa_block.exit_versions):
            consumers.setdefault(use_key, set()).add(keep_consumer)

    overwritten_names = {name for name, count in def_counts.items() if count > 1}
    eligible = {
        stmt_key
        for stmt_key in removable_stmt_keys
        if stmt_key not in baseline_dse_keys
        and any(def_key[0] in overwritten_names for def_key in stmt_defs.get(stmt_key, set()))
    }

    removed = set(baseline_dse_keys)
    changed = True
    while changed:
        changed = False
        for stmt_key in sorted(
            eligible - removed,
            key=lambda key: (
                cfg.blocks[key[0]].statements[key[1]].range.start.offset,
                key[0],
                key[1],
            ),
        ):
            defs = stmt_defs.get(stmt_key, set())
            if not defs:
                continue

            has_removed_consumer = False
            can_remove = True
            for def_key in defs:
                def_consumers = consumers.get(def_key, set())
                if not def_consumers:
                    can_remove = False
                    break

                for consumer in def_consumers:
                    if consumer == keep_consumer:
                        can_remove = False
                        break
                    if consumer in removed:
                        has_removed_consumer = True
                        continue
                    can_remove = False
                    break
                if not can_remove:
                    break

            if can_remove and has_removed_consumer:
                removed.add(stmt_key)
                changed = True

    adce_keys = removed - set(baseline_dse_keys)
    return sorted(
        adce_keys,
        key=lambda key: (
            cfg.blocks[key[0]].statements[key[1]].range.start.offset,
            key[0],
            key[1],
        ),
    )


def optimise_elimination_passes(
    ctx: PassContext,
    cfg,
    ssa,
    analysis,
    execution_intent: FunctionExecutionIntent,
    *,
    is_top_level: bool = False,
) -> None:
    source = ctx.source
    range_by_stmt, next_start_by_stmt = _statement_rewrite_context(source, cfg)

    executable_blocks = set(cfg.blocks) - set(analysis.unreachable_blocks)
    removable_def_versions: dict[str, set[int]] = {}
    for block_name in executable_blocks:
        block = cfg.blocks.get(block_name)
        ssa_block = ssa.blocks.get(block_name)
        if block is None or ssa_block is None:
            continue
        for idx, ssa_stmt in enumerate(ssa_block.statements):
            if idx < 0 or idx >= len(block.statements):
                continue
            stmt = block.statements[idx]
            if not _is_adce_removable_statement(
                stmt,
                stmt_key=(block_name, idx),
                execution_intent=execution_intent,
            ):
                continue
            for name, ver in ssa_stmt.defs.items():
                if ver <= 0:
                    continue
                removable_def_versions.setdefault(name, set()).add(ver)

    # Call-by-name suppression (mirrors W211/W220 in
    # ``analyser/_diag_var_lifecycle``).  A literal-name arg passed to a
    # user proc whose param carries ``VAR_READ`` / ``VAR_WRITE`` is an
    # indirect read of the caller-local of that name.  Deleting the
    # preceding ``set`` would feed an uninitialised name to the callee —
    # a hard correctness bug if the quick-fix were applied.
    from compiler.proc_arg_traits import (
        build_proc_index_from_summaries,
        collect_call_by_name_reads,
    )

    proc_index = build_proc_index_from_summaries(ctx.interproc.procedures)
    call_by_name = collect_call_by_name_reads(cfg, proc_index)

    dead_entries: list[tuple[int, tuple[str, int]]] = []
    for dead in analysis.dead_stores:
        if dead.variable in ctx.cross_event_vars:
            continue
        if dead.variable in call_by_name:
            continue
        later_versions = removable_def_versions.get(dead.variable, set())
        if not any(ver > dead.version for ver in later_versions):
            continue
        key = (dead.block, dead.statement_index)
        # D2-O109 closure: gate elimination on RHS purity (same reasoning
        # as O126).  ``set x a; set y [append x b]; ...`` -- the first
        # ``set x a`` is observably dead at the SSA level, but if we
        # deleted ``set x a`` *and* the second store happened to write
        # x as a side effect, the printed value would change.  Stay
        # safe: only delete when the assignment's RHS is provably
        # side-effect-free.  (The O100 / O109 stale-fact problem the
        # reviewer flagged shares the same root cause -- cmd-sub writes
        # not tracked in SSA -- but the purity gate fixes the
        # observable-behaviour-loss part of it.)
        block_stmts = cfg.blocks.get(dead.block)
        if block_stmts is None:
            continue
        if dead.statement_index < 0 or dead.statement_index >= len(block_stmts.statements):
            continue
        ir_stmt = block_stmts.statements[dead.statement_index]
        if not _assignment_safe_to_delete(ir_stmt):
            continue
        stmt_range = range_by_stmt.get(key)
        if stmt_range is None:
            continue
        dead_entries.append((stmt_range.start.offset, key))

    for _, key in sorted(dead_entries):
        stmt_range = range_by_stmt[key]
        delete_range = _statement_delete_rewrite_range(
            source,
            stmt_range,
            next_start_by_stmt.get(key),
        )
        ctx.optimisations.append(
            Optimisation(
                code="O109",
                message="Eliminate dead store",
                range=delete_range,
                replacement="",
            )
        )

    baseline_dse_keys = {key for _, key in dead_entries}

    # O126: Remove unused variable assignments.
    # Unlike O109 (dead store — overwritten before read), this targets
    # variables that are *never* read anywhere in the function.
    # Skip at top-level: the last command's result is the script return
    # value, and top-level variables may be consumed by other contexts
    # (upvar, info exists, etc.).
    # Also skip variables that are upvar/global/variable aliases —
    # writes to them are visible in other scopes even though the local
    # analysis sees no local reads.
    from compiler.registry.runtime import scope_alias_commands

    scope_aliases: set[str] = set()
    _alias_cmds = scope_alias_commands()
    for block in cfg.blocks.values():
        for stmt in block.statements:
            if isinstance(stmt, IRCall) and stmt.command in _alias_cmds:
                scope_aliases.update(stmt.defs)

    unused_entries: list[tuple[int, tuple[str, int]]] = []
    if not is_top_level:
        for unused in analysis.unused_variables:
            if unused.variable in ctx.cross_event_vars:
                continue
            if unused.variable in scope_aliases:
                continue
            # Call-by-name suppression (see dead-store section above).
            if unused.variable in call_by_name:
                continue
            key = (unused.block, unused.statement_index)
            if key in baseline_dse_keys:
                continue
            # D2-O126 closure: gate elimination on RHS purity.  The
            # variable is provably unused, but the assignment's RHS may
            # have observable side effects (``set unused [puts X]``
            # prints) or mutate state (``set unused [set x 1]``).
            # ``_assignment_safe_to_delete`` returns True only when
            # every embedded command substitution is classified ``pure``
            # by ``compiler.side_effects.classify_side_effects``.
            block_stmts = cfg.blocks.get(unused.block)
            if block_stmts is None:
                continue
            if unused.statement_index < 0 or unused.statement_index >= len(block_stmts.statements):
                continue
            ir_stmt = block_stmts.statements[unused.statement_index]
            if not _assignment_safe_to_delete(ir_stmt):
                continue
            stmt_range = range_by_stmt.get(key)
            if stmt_range is None:
                continue
            unused_entries.append((stmt_range.start.offset, key))

    for _, key in sorted(unused_entries):
        stmt_range = range_by_stmt[key]
        delete_range = _statement_delete_rewrite_range(
            source,
            stmt_range,
            next_start_by_stmt.get(key),
        )
        ctx.optimisations.append(
            Optimisation(
                code="O126",
                message="Remove unused variable assignment",
                range=delete_range,
                replacement="",
            )
        )

    unreachable_entries: list[tuple[int, tuple[str, int]]] = []
    for block_name in analysis.unreachable_blocks:
        block = cfg.blocks.get(block_name)
        if block is None:
            continue
        for idx, _stmt in enumerate(block.statements):
            key = (block_name, idx)
            stmt_range = range_by_stmt.get(key)
            if stmt_range is None:
                continue
            unreachable_entries.append((stmt_range.start.offset, key))

    for _, key in sorted(unreachable_entries):
        stmt_range = range_by_stmt[key]
        delete_range = _statement_delete_rewrite_range(
            source,
            stmt_range,
            next_start_by_stmt.get(key),
        )
        ctx.optimisations.append(
            Optimisation(
                code="O107",
                message="Eliminate unreachable dead code",
                range=delete_range,
                replacement="",
            )
        )

    for key in _collect_adce_statement_keys(
        ctx,
        cfg,
        ssa,
        analysis,
        execution_intent,
        baseline_dse_keys=baseline_dse_keys,
    ):
        stmt_range = range_by_stmt.get(key)
        if stmt_range is None:
            continue
        delete_range = _statement_delete_rewrite_range(
            source,
            stmt_range,
            next_start_by_stmt.get(key),
        )
        ctx.optimisations.append(
            Optimisation(
                code="O108",
                message="Eliminate transitively dead code",
                range=delete_range,
                replacement="",
            )
        )

    # Post-propagation DSE: eliminate definitions whose only consumers
    # were branch conditions that got constant-propagated.
    if ctx.propagated_branch_uses:
        all_removed = baseline_dse_keys | {
            key
            for key in _collect_adce_statement_keys(
                ctx,
                cfg,
                ssa,
                analysis,
                execution_intent,
                baseline_dse_keys=baseline_dse_keys,
            )
        }
        _eliminate_propagated_constants(
            ctx,
            source,
            cfg,
            ssa,
            analysis,
            execution_intent,
            range_by_stmt,
            next_start_by_stmt,
            all_removed,
        )


def _eliminate_propagated_constants(
    ctx: PassContext,
    source: str,
    cfg,
    ssa,
    analysis,
    execution_intent: FunctionExecutionIntent,
    range_by_stmt,
    next_start_by_stmt,
    already_removed: set[tuple[str, int]],
) -> None:
    """Eliminate set statements for constants whose branch uses were all propagated."""
    executable_blocks = set(cfg.blocks) - set(analysis.unreachable_blocks)

    def_to_key: dict[tuple[str, int], tuple[str, int]] = {}
    for block_name in executable_blocks:
        block = cfg.blocks.get(block_name)
        ssa_block = ssa.blocks.get(block_name)
        if block is None or ssa_block is None:
            continue
        for idx, ssa_stmt in enumerate(ssa_block.statements):
            if idx >= len(block.statements):
                continue
            stmt = block.statements[idx]
            if not _is_adce_removable_statement(
                stmt,
                stmt_key=(block_name, idx),
                execution_intent=execution_intent,
            ):
                continue
            for name, ver in ssa_stmt.defs.items():
                if name in ctx.cross_event_vars:
                    continue
                if ver > 0 and (name, ver) in ctx.propagated_branch_uses:
                    def_to_key[(name, ver)] = (block_name, idx)

    if not def_to_key:
        return

    for (name, ver), key in sorted(def_to_key.items(), key=lambda kv: kv[1]):
        if key in already_removed:
            continue
        has_live_consumer = False
        for block_name in executable_blocks:
            ssa_block = ssa.blocks.get(block_name)
            if ssa_block is None:
                continue
            # Statement uses.
            for idx, ssa_stmt in enumerate(ssa_block.statements):
                if ssa_stmt.uses.get(name) == ver:
                    if (block_name, idx) not in already_removed and (
                        block_name,
                        idx,
                    ) not in ctx.propagated_expr_stmts:
                        has_live_consumer = True
                        break
            if has_live_consumer:
                break
            # Phi uses.
            for phi in ssa_block.phis:
                if phi.name == name:
                    for pred, incoming_ver in phi.incoming.items():
                        if incoming_ver == ver and pred in executable_blocks:
                            has_live_consumer = True
                            break
                if has_live_consumer:
                    break
            if has_live_consumer:
                break
            # Return uses.
            block = cfg.blocks.get(block_name)
            if block is not None:
                for use_key in _return_use_versions(
                    block.terminator,
                    ssa_block.exit_versions,
                ):
                    if use_key == (name, ver):
                        has_live_consumer = True
                        break
            if has_live_consumer:
                break

        if has_live_consumer:
            continue

        stmt_range = range_by_stmt.get(key)
        if stmt_range is None:
            continue
        delete_range = _statement_delete_rewrite_range(
            source,
            stmt_range,
            next_start_by_stmt.get(key),
        )
        # Inherit group from the propagation that consumed this def.
        dse_group = ctx.propagated_use_groups.get((name, ver))
        ctx.optimisations.append(
            Optimisation(
                code="O109",
                message="Eliminate dead store",
                range=delete_range,
                replacement="",
                group=dse_group,
            )
        )
