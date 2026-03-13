"""Elimination passes (DCE, DSE, ADCE) for the optimiser."""

from __future__ import annotations

from ...common.naming import (
    normalise_var_name as _normalise_var_name,
)
from ...parsing.lexer import TclLexer
from ...parsing.tokens import TokenType
from ..cfg import CFGBranch, CFGReturn
from ..execution_intent import EscapeClass, FunctionExecutionIntent, SideEffectClass
from ..expr_ast import vars_in_expr_node
from ..ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRCall,
    IRIncr,
)
from ._expr_simplify import _expr_has_command_subst
from ._pattern_recognition import _statement_delete_rewrite_range, _statement_rewrite_context
from ._types import Optimisation, PassContext


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
    if not isinstance(term, CFGReturn) or term.value is None:
        return set()
    uses: set[tuple[str, int]] = set()
    lexer = TclLexer(term.value)
    while True:
        tok = lexer.get_token()
        if tok is None:
            break
        if tok.type is not TokenType.VAR:
            continue
        name = _normalise_var_name(tok.text)
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

    dead_entries: list[tuple[int, tuple[str, int]]] = []
    for dead in analysis.dead_stores:
        if dead.variable in ctx.cross_event_vars:
            continue
        later_versions = removable_def_versions.get(dead.variable, set())
        if not any(ver > dead.version for ver in later_versions):
            continue
        key = (dead.block, dead.statement_index)
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
    scope_aliases: set[str] = set()
    _ALIAS_CMDS = frozenset(("upvar", "namespace upvar", "global", "variable"))
    for block in cfg.blocks.values():
        for stmt in block.statements:
            if isinstance(stmt, IRCall) and stmt.command in _ALIAS_CMDS:
                scope_aliases.update(stmt.defs)

    unused_entries: list[tuple[int, tuple[str, int]]] = []
    if not is_top_level:
        for unused in analysis.unused_variables:
            if unused.variable in ctx.cross_event_vars:
                continue
            if unused.variable in scope_aliases:
                continue
            key = (unused.block, unused.statement_index)
            if key in baseline_dse_keys:
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
