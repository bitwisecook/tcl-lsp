"""Code sinking pass (O125) for the optimiser.

Sinks side-effect-free definitions into the deepest decision block
that uses them.  For example::

    set b foo               # [O125] set b foo → sunk into if body
    if {$a} {               if {$a} {
        puts $b        →        set b foo
    }                           puts $b
                            }
"""

from __future__ import annotations

import re

from ...analysis.semantic_model import Range
from ...common.naming import normalise_var_name as _normalise_var_name
from ..expr_ast import vars_in_expr_node
from ..ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRIncr,
    IRReturn,
    IRScript,
    IRSwitch,
    IRTry,
    IRWhile,
)
from ._helpers import _full_command_range
from ._types import Optimisation, PassContext

# ---------------------------------------------------------------------------
# Sinkability check
# ---------------------------------------------------------------------------


def _is_sinkable(stmt) -> bool:
    """Return ``True`` if *stmt* is a side-effect-free assignment that can be sunk."""
    match stmt:
        case IRAssignConst():
            return True
        case IRAssignValue(value=value, value_needs_backsubst=value_needs_backsubst):
            # Safe only when the value has no substitutions.
            return not value_needs_backsubst and "[" not in value and "$" not in value
    return False


# ---------------------------------------------------------------------------
# Variable reference detection
# ---------------------------------------------------------------------------


def _text_references_var(text: str, var_name: str) -> bool:
    """Check whether *text* contains a ``$var_name`` reference."""
    # Quick pre-check to avoid regex overhead.
    if f"${var_name}" not in text:
        return False
    escaped = re.escape(var_name)
    pattern = rf"\$(?:\{{{escaped}\}}|{escaped}(?![A-Za-z0-9_:]))"
    return bool(re.search(pattern, text))


def _text_mentions_var(text: str, var_name: str) -> bool:
    """Conservative check for textual mentions of *var_name*."""
    if _text_references_var(text, var_name):
        return True
    escaped = re.escape(var_name)
    pattern = rf"(?<![A-Za-z0-9_:])(?:\{{{escaped}\}}|{escaped})(?![A-Za-z0-9_:])"
    return bool(re.search(pattern, text))


def _expr_references_var(expr, var_name: str) -> bool:
    """Return ``True`` when *expr* references *var_name*."""
    norm_name = _normalise_var_name(var_name)
    return any(_normalise_var_name(name) == norm_name for name in vars_in_expr_node(expr))


def _script_uses_var(source: str, script: IRScript, var_name: str) -> bool:
    """Return ``True`` when *script* references *var_name*."""
    for stmt in script.statements:
        if _stmt_uses_var(source, stmt, var_name):
            return True
    return False


def _stmt_uses_var(source: str, stmt, var_name: str) -> bool:
    """Return ``True`` if *stmt* (including nested bodies) references *var_name*."""
    norm_name = _normalise_var_name(var_name)
    match stmt:
        case IRAssignConst():
            return False
        case IRAssignExpr(expr=expr):
            return _expr_references_var(expr, var_name)
        case IRAssignValue(value=value):
            return _text_references_var(value, var_name)
        case IRIncr(name=name, amount=amount):
            if _normalise_var_name(name) == norm_name:
                return True
            if amount is not None and _text_references_var(amount, var_name):
                return True
            return False
        case IRCall(
            defs=defs,
            reads=reads,
            reads_own_defs=reads_own_defs,
            args=args,
        ):
            if any(_normalise_var_name(read) == norm_name for read in reads if read):
                return True
            if reads_own_defs and any(_normalise_var_name(d) == norm_name for d in defs if d):
                return True
            if any(_text_references_var(arg, var_name) for arg in args):
                return True
        case IRReturn(value=value):
            return value is not None and _text_references_var(value, var_name)
        case IRIf(clauses=clauses, else_body=else_body):
            for clause in clauses:
                if _expr_references_var(clause.condition, var_name):
                    return True
                if _script_uses_var(source, clause.body, var_name):
                    return True
            if else_body is not None and _script_uses_var(source, else_body, var_name):
                return True
            return False
        case IRSwitch(subject=subject, arms=arms, default_body=default_body):
            if _text_references_var(subject, var_name):
                return True
            for arm in arms:
                if arm.body is not None and _script_uses_var(source, arm.body, var_name):
                    return True
            if default_body is not None and _script_uses_var(source, default_body, var_name):
                return True
            return False
        case IRFor(init=init, condition=condition, next=next_script, body=body):
            if _script_uses_var(source, init, var_name):
                return True
            if _expr_references_var(condition, var_name):
                return True
            if _script_uses_var(source, next_script, var_name):
                return True
            if _script_uses_var(source, body, var_name):
                return True
            return False
        case IRWhile(condition=condition, body=body):
            return _expr_references_var(condition, var_name) or _script_uses_var(
                source,
                body,
                var_name,
            )
        case IRForeach(iterators=iterators, body=body):
            if any(_text_references_var(list_arg, var_name) for _vars, list_arg in iterators):
                return True
            return _script_uses_var(source, body, var_name)
        case IRCatch(body=body):
            return _script_uses_var(source, body, var_name)
        case IRTry(body=body, handlers=handlers, finally_body=finally_body):
            if _script_uses_var(source, body, var_name):
                return True
            for handler in handlers:
                if _script_uses_var(source, handler.body, var_name):
                    return True
            if finally_body is not None and _script_uses_var(source, finally_body, var_name):
                return True
            return False

    stmt_range = getattr(stmt, "range", None)
    if stmt_range is None:
        return False
    full_range = _full_command_range(source, stmt_range) or stmt_range
    text = source[full_range.start.offset : full_range.end.offset + 1]
    return _text_mentions_var(text, var_name)


def _script_defines_var(script: IRScript, var_name: str) -> bool:
    """Return ``True`` if *script* defines (writes) *var_name*."""
    for stmt in script.statements:
        if _stmt_defines_var(stmt, var_name):
            return True
    return False


def _stmt_defines_var(stmt, var_name: str) -> bool:
    """Return ``True`` if *stmt* defines (writes) *var_name*."""
    norm_name = _normalise_var_name(var_name)
    match stmt:
        case IRAssignConst(name=name) | IRAssignValue(name=name) | IRAssignExpr(name=name):
            return _normalise_var_name(name) == norm_name
        case IRIncr(name=name):
            return _normalise_var_name(name) == norm_name
        case IRCall(defs=defs):
            return any(_normalise_var_name(d) == norm_name for d in defs if d)
        case IRIf(clauses=clauses, else_body=else_body):
            for clause in clauses:
                if _script_defines_var(clause.body, var_name):
                    return True
            if else_body is not None and _script_defines_var(else_body, var_name):
                return True
            return False
        case IRSwitch(arms=arms, default_body=default_body):
            for arm in arms:
                if arm.body is not None and _script_defines_var(arm.body, var_name):
                    return True
            if default_body is not None and _script_defines_var(default_body, var_name):
                return True
            return False
        case IRFor(init=init, next=next_script, body=body):
            return (
                _script_defines_var(init, var_name)
                or _script_defines_var(next_script, var_name)
                or _script_defines_var(body, var_name)
            )
        case IRWhile(body=body):
            return _script_defines_var(body, var_name)
        case IRForeach(iterators=iterators, body=body):
            if any(
                _normalise_var_name(iter_var) == norm_name
                for var_list, _list_arg in iterators
                for iter_var in var_list
            ):
                return True
            return _script_defines_var(body, var_name)
        case IRCatch(body=body, result_var=result_var, options_var=options_var):
            if result_var is not None and _normalise_var_name(result_var) == norm_name:
                return True
            if options_var is not None and _normalise_var_name(options_var) == norm_name:
                return True
            return _script_defines_var(body, var_name)
        case IRTry(body=body, handlers=handlers, finally_body=finally_body):
            if _script_defines_var(body, var_name):
                return True
            for handler in handlers:
                if (
                    handler.var_name is not None
                    and _normalise_var_name(handler.var_name) == norm_name
                ):
                    return True
                if (
                    handler.options_var is not None
                    and _normalise_var_name(handler.options_var) == norm_name
                ):
                    return True
                if _script_defines_var(handler.body, var_name):
                    return True
            if finally_body is not None and _script_defines_var(finally_body, var_name):
                return True
            return False
    return False


def _used_in_remaining_stmts(
    source: str,
    stmts: tuple,
    start_idx: int,
    var_name: str,
) -> bool:
    """Check whether *var_name* is referenced in statements from *start_idx* onwards."""
    for j in range(start_idx, len(stmts)):
        if _stmt_uses_var(source, stmts[j], var_name):
            return True
    return False


# ---------------------------------------------------------------------------
# Decision-block helpers
# ---------------------------------------------------------------------------


def _decision_body_info(stmt) -> list[tuple[IRScript, Range]] | None:
    """Return ``(body, body_range)`` pairs for each branch of a decision block."""
    match stmt:
        case IRIf(clauses=clauses, else_body=else_body, else_range=else_range):
            result = [(clause.body, clause.body_range) for clause in clauses]
            if else_body is not None and else_range is not None:
                result.append((else_body, else_range))
            return result
        case IRSwitch(arms=arms, default_body=default_body, default_range=default_range):
            result = []
            for arm in arms:
                if arm.body is not None and arm.body_range is not None and not arm.fallthrough:
                    result.append((arm.body, arm.body_range))
            if default_body is not None and default_range is not None:
                result.append((default_body, default_range))
            return result
    return None


def _decision_condition_uses_var(stmt, var_name: str) -> bool:
    """Return ``True`` if any condition of a decision block references *var_name*."""
    match stmt:
        case IRIf(clauses=clauses):
            for clause in clauses:
                if _expr_references_var(clause.condition, var_name):
                    return True
        case IRSwitch(subject=subject):
            if _text_references_var(subject, var_name):
                return True
    return False


# ---------------------------------------------------------------------------
# Deepest-target search
# ---------------------------------------------------------------------------


def _find_deepest_sink_targets(
    source: str,
    body: IRScript,
    body_range: Range,
    var_name: str,
) -> list[Range]:
    """Find the deepest body ranges where *var_name* should be inserted.

    Returns a list of ``body_range`` values for insertion.  An empty list
    means the variable is not used in *body*.
    """
    stmts = body.statements

    # Which statements in this body use the variable?
    using_indices: list[int] = []
    for j, s in enumerate(stmts):
        if _stmt_uses_var(source, s, var_name):
            using_indices.append(j)

    if not using_indices:
        return []  # Variable not used in this body.

    # If only one statement uses the variable, try to sink deeper.
    if len(using_indices) == 1:
        j = using_indices[0]
        # Ensure no prior statement redefines the variable.
        can_pass = True
        for k in range(j):
            if _stmt_defines_var(stmts[k], var_name):
                can_pass = False
                break
        if can_pass:
            inner_stmt = stmts[j]
            deeper = _try_deeper_sink(source, inner_stmt, var_name)
            if deeper:
                return deeper

    # Sink at this level.
    return [body_range]


def _try_deeper_sink(source: str, stmt, var_name: str) -> list[Range]:
    """Try to find deeper sink targets within a decision block."""
    match stmt:
        case IRIf(clauses=clauses, else_body=else_body, else_range=else_range):
            # Variable must not appear in any condition.
            for clause in clauses:
                if _expr_references_var(clause.condition, var_name):
                    return []
            targets: list[Range] = []
            for clause in clauses:
                targets.extend(
                    _find_deepest_sink_targets(
                        source,
                        clause.body,
                        clause.body_range,
                        var_name,
                    )
                )
            if else_body is not None and else_range is not None:
                targets.extend(
                    _find_deepest_sink_targets(
                        source,
                        else_body,
                        else_range,
                        var_name,
                    )
                )
            return targets

        case IRSwitch(
            subject=subject,
            arms=arms,
            default_body=default_body,
            default_range=default_range,
        ):
            if _text_references_var(subject, var_name):
                return []
            targets = []
            for arm in arms:
                if arm.body is not None and arm.body_range is not None and not arm.fallthrough:
                    targets.extend(
                        _find_deepest_sink_targets(
                            source,
                            arm.body,
                            arm.body_range,
                            var_name,
                        )
                    )
            if default_body is not None and default_range is not None:
                targets.extend(
                    _find_deepest_sink_targets(
                        source,
                        default_body,
                        default_range,
                        var_name,
                    )
                )
            return targets

    return []


# ---------------------------------------------------------------------------
# Body-insertion helpers
# ---------------------------------------------------------------------------


def _detect_body_indent(source: str, body_range: Range) -> str:
    """Detect the indentation used for statements inside a brace-delimited body."""
    brace_offset = body_range.start.offset
    pos = brace_offset + 1
    if pos >= len(source):
        return "    "
    # Scan past any whitespace/CR on the same line as {.
    while pos < len(source) and source[pos] in " \t\r":
        pos += 1
    if pos >= len(source) or source[pos] != "\n":
        # Single-line body or content on the { line — use default.
        return "    "
    # Skip newline.
    pos += 1
    indent = ""
    while pos < len(source) and source[pos] in " \t":
        indent += source[pos]
        pos += 1
    return indent or "    "


def _body_insertion_replacement(
    source: str,
    body_range: Range,
    stmt_text: str,
) -> str:
    """Build replacement text for the opening ``{`` of a body.

    Replaces the single ``{`` character with ``{\\n<indent><stmt_text>``
    so the sunk statement appears at the start of the body.
    """
    indent = _detect_body_indent(source, body_range)
    next_offset = body_range.start.offset + 1
    if next_offset < len(source) and source[next_offset] == "\n":
        # Multi-line body: existing newline separates from the next statement.
        return "{\n" + indent + stmt_text
    # Single-line body: add newline + indent after the sunk statement.
    return "{\n" + indent + stmt_text + "\n" + indent


# ---------------------------------------------------------------------------
# Optimisation emission
# ---------------------------------------------------------------------------


def _emit_sinking_opts(
    ctx: PassContext,
    source: str,
    stmt,
    stmt_range: Range,
    var_name: str,
    targets: list[Range],
    decision_stmt,
) -> None:
    """Emit grouped ``O125`` optimisation objects for a sinking rewrite."""
    group = ctx.alloc_group()

    full_range = _full_command_range(source, stmt_range) or stmt_range
    stmt_text = source[stmt_range.start.offset : stmt_range.end.offset + 1].strip()

    target_name = "if body" if isinstance(decision_stmt, IRIf) else "switch arm"

    # Part 1: replace original statement with a comment.
    comment = f"# [O125] {stmt_text} \u2192 sunk into {target_name}"
    ctx.optimisations.append(
        Optimisation(
            code="O125",
            message=f"Sink {stmt_text} into {target_name}",
            range=full_range,
            replacement=comment,
            group=group,
        )
    )

    # Part 2: insert statement at the start of each target body.
    for body_range in targets:
        brace_range = Range(start=body_range.start, end=body_range.start)
        replacement = _body_insertion_replacement(source, body_range, stmt_text)
        ctx.optimisations.append(
            Optimisation(
                code="O125",
                message=f"Insert sunk {stmt_text}",
                range=brace_range,
                replacement=replacement,
                group=group,
            )
        )


# ---------------------------------------------------------------------------
# Main walk
# ---------------------------------------------------------------------------


def optimise_code_sinking(
    ctx: PassContext,
    ir_script: IRScript | None,
) -> None:
    """O125: Sink definitions into the deepest decision block that uses them."""
    if ir_script is None:
        return
    _walk_for_sinking(ctx, ir_script)


def _walk_for_sinking(ctx: PassContext, ir_script: IRScript) -> None:
    source = ctx.source
    stmts = ir_script.statements

    # Look for sinkable patterns at this level.
    for i in range(len(stmts) - 1):
        stmt = stmts[i]
        next_stmt = stmts[i + 1]

        # Only sink side-effect-free assignments.
        if not isinstance(stmt, (IRAssignConst, IRAssignValue)):
            continue
        if not _is_sinkable(stmt):
            continue

        var_name = stmt.name
        if var_name in ctx.cross_event_vars:
            continue

        stmt_range = stmt.range

        # Skip if another optimisation already covers this statement
        # (e.g. O109 dead-store elimination already deletes it).
        stmt_start = stmt_range.start.offset
        stmt_end = stmt_range.end.offset
        already_covered = any(
            opt.range.start.offset <= stmt_start and opt.range.end.offset >= stmt_end
            for opt in ctx.optimisations
        )
        if already_covered:
            continue

        # Next statement must be a decision block.
        body_info = _decision_body_info(next_stmt)
        if body_info is None:
            continue

        # Variable must NOT appear in any condition.
        if _decision_condition_uses_var(next_stmt, var_name):
            continue

        # Variable must NOT be used after the decision block.
        if _used_in_remaining_stmts(source, stmts, i + 2, var_name):
            continue

        # Find deepest targets (branches that use the variable).
        targets: list[Range] = []
        for body, body_range in body_info:
            targets.extend(_find_deepest_sink_targets(source, body, body_range, var_name))

        if not targets:
            continue

        _emit_sinking_opts(ctx, source, stmt, stmt_range, var_name, targets, next_stmt)

    # Recurse into nested compound statements.
    for stmt in stmts:
        match stmt:
            case IRIf(clauses=clauses, else_body=else_body):
                for clause in clauses:
                    _walk_for_sinking(ctx, clause.body)
                if else_body is not None:
                    _walk_for_sinking(ctx, else_body)
            case IRSwitch(arms=arms, default_body=default_body):
                for arm in arms:
                    if arm.body is not None:
                        _walk_for_sinking(ctx, arm.body)
                if default_body is not None:
                    _walk_for_sinking(ctx, default_body)
            case IRFor(init=init, body=body, next=next_script):
                _walk_for_sinking(ctx, init)
                _walk_for_sinking(ctx, body)
                _walk_for_sinking(ctx, next_script)
            case IRWhile(body=body):
                _walk_for_sinking(ctx, body)
            case IRForeach(body=body):
                _walk_for_sinking(ctx, body)
            case IRCatch(body=body):
                _walk_for_sinking(ctx, body)
            case IRTry(body=body, handlers=handlers, finally_body=finally_body):
                _walk_for_sinking(ctx, body)
                for handler in handlers:
                    _walk_for_sinking(ctx, handler.body)
                if finally_body is not None:
                    _walk_for_sinking(ctx, finally_body)
