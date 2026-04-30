"""S5.4 — Global Value Numbering for redundant expression computation.

When the same expression is computed twice within a basic block
with no intervening write to its source variables, the second
computation is redundant — replace it with a copy from the first
result.  Concretely::

    set y [expr {$x + 1}]
    set z [expr {$x + 1}]   # ← redundant; same value as y

becomes::

    set y [expr {$x + 1}]
    set z $y                # ← copy from the prior result

The transformation is purely subtractive at runtime — no new
allocations, no extra retain/release.

**Scope.**  We restrict to consecutive top-level statements in
the same script.  Nested control-flow boundaries reset the
value table because any branch / loop break could invalidate
the recorded source-var values.  Multi-block GVN would need
proper SSA infrastructure that doesn't exist in the IR walker.

**Soundness.**  Each candidate match requires:

* the prior :class:`IRAssignExpr` exposes a ``ExprNode`` whose
  free variable set we can extract;
* none of those source variables has been written between the
  two assignments (any :class:`IRAssignConst` /
  :class:`IRAssignValue` / :class:`IRAssignExpr` / :class:`IRIncr`
  / :class:`IRCall` defs that touch a source var clears the
  table);
* the prior assignment's target hasn't been over-written either
  (a re-bind of ``y`` between the two writes invalidates the
  copy);
* the candidate's own target is a local (non-``::``-qualified,
  non-array-element) — globals are observable from outside and
  must keep their explicit re-evaluation.
"""

from __future__ import annotations

from dataclasses import replace

from ..expr_ast import vars_in_expr_node
from ..ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBlock,
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRIncr,
    IRModule,
    IRProcedure,
    IRScript,
    IRSwitch,
    IRTry,
    IRUpFrame,
    IRWhile,
)


def gvn_module(module: IRModule) -> IRModule:
    """Return a new module with redundant ``IRAssignExpr`` writes
    replaced by copies from prior equivalent results.

    Operates on every proc body and the top-level script.
    Pure-leaf restriction isn't required here — the rewrite
    just substitutes equivalent values; it doesn't drop side
    effects.  The original module is left untouched (every IR
    node is frozen).
    """
    new_top = _gvn_script(module.top_level)
    new_procs: dict[str, IRProcedure] = {}
    changed_any = new_top is not module.top_level
    for qname, proc in module.procedures.items():
        new_body = _gvn_script(proc.body)
        if new_body is proc.body:
            new_procs[qname] = proc
        else:
            new_procs[qname] = replace(proc, body=new_body)
            changed_any = True
    if not changed_any:
        return module
    return replace(module, top_level=new_top, procedures=new_procs)


def _gvn_script(script: IRScript) -> IRScript:
    """Single-pass GVN over the top-level statements of ``script``.

    Recurses into nested control-flow bodies but resets the
    value table at each boundary — control flow can change
    which path produced the prior result.
    """
    # value_table maps an expression's hashable key to the local
    # variable name that holds its value.  Cleared on any write
    # to a variable that participates in a tracked expression.
    value_table: dict[tuple, str] = {}
    new_stmts: list = []
    changed = False

    for stmt in script.statements:
        # Recurse into any nested control flow first.  The
        # nested rewrite gets its own value table; we don't
        # propagate equivalences across nested boundaries.
        rewritten = _recurse_nested(stmt)
        if rewritten is not stmt:
            new_stmts.append(rewritten)
            changed = True
            value_table.clear()  # control flow boundary
            continue

        # Top-level GVN: try the substitution rule.  Compute the
        # candidate's expression key BEFORE the write so we can
        # check for a prior equivalent.
        candidate_key: tuple | None = None
        candidate_target: str | None = None
        if isinstance(stmt, IRAssignExpr) and _is_local(stmt.name):
            candidate_target = stmt.name
            candidate_key = _expr_key(stmt.expr)
            if candidate_key is not None and candidate_key in value_table:
                prior_var = value_table[candidate_key]
                # Don't substitute if the prior var was reassigned
                # (would have been cleared from the table) — the
                # presence in value_table guarantees freshness.
                # Replace the IRAssignExpr with a $-substitution
                # of the prior result.
                stmt = IRAssignValue(
                    range=stmt.range,
                    name=stmt.name,
                    value="$" + prior_var,
                )
                changed = True
                # The substituted ``set z $y`` doesn't create a
                # new entry — it just aliases.  Skip the post-
                # statement table-update for this one.
                candidate_key = None
                candidate_target = None

        new_stmts.append(stmt)

        # Invalidate value table entries whose source vars were
        # just written.  ``IRCall`` writes a ``"*"`` marker
        # because any call might side-effect via eval / upvar /
        # info — when we see that, drop the whole table.
        targets = _stmt_writes(stmt)
        if "*" in targets:
            value_table.clear()
        elif targets:
            for k in list(value_table):
                src_vars, _ = k
                if any(t in src_vars for t in targets):
                    del value_table[k]
                elif value_table[k] in targets:
                    del value_table[k]

        # Now (after invalidation) record the new entry if we
        # had a candidate key — this prevents the just-recorded
        # entry from being immediately deleted by its own write.
        if candidate_key is not None and candidate_target is not None:
            value_table[candidate_key] = candidate_target

    if not changed:
        return script
    return IRScript(statements=tuple(new_stmts))


def _recurse_nested(stmt: object) -> object:
    """Recurse into nested control-flow bodies, applying GVN to
    each.  Returns the original instance when no nested rewrite
    occurred."""
    if isinstance(stmt, IRBlock):
        new_body = _gvn_script(stmt.body)
        if new_body is stmt.body:
            return stmt
        return replace(stmt, body=new_body)
    if isinstance(stmt, IRIf):
        new_clauses = []
        clauses_changed = False
        for clause in stmt.clauses:
            new_body = _gvn_script(clause.body)
            if new_body is clause.body:
                new_clauses.append(clause)
            else:
                new_clauses.append(replace(clause, body=new_body))
                clauses_changed = True
        new_else = stmt.else_body
        else_changed = False
        if stmt.else_body is not None:
            new_else = _gvn_script(stmt.else_body)
            else_changed = new_else is not stmt.else_body
        if not clauses_changed and not else_changed:
            return stmt
        return replace(stmt, clauses=tuple(new_clauses), else_body=new_else)
    if isinstance(stmt, IRFor):
        new_init = _gvn_script(stmt.init)
        new_next = _gvn_script(stmt.next)
        new_body = _gvn_script(stmt.body)
        if new_init is stmt.init and new_next is stmt.next and new_body is stmt.body:
            return stmt
        return replace(stmt, init=new_init, next=new_next, body=new_body)
    if isinstance(stmt, (IRWhile, IRForeach, IRCatch, IRUpFrame)):
        new_body = _gvn_script(stmt.body)
        if new_body is stmt.body:
            return stmt
        return replace(stmt, body=new_body)
    if isinstance(stmt, IRTry):
        new_body = _gvn_script(stmt.body)
        new_handlers = []
        handlers_changed = False
        for h in stmt.handlers:
            new_h_body = _gvn_script(h.body)
            if new_h_body is h.body:
                new_handlers.append(h)
            else:
                new_handlers.append(replace(h, body=new_h_body))
                handlers_changed = True
        new_finally = stmt.finally_body
        finally_changed = False
        if stmt.finally_body is not None:
            new_finally = _gvn_script(stmt.finally_body)
            finally_changed = new_finally is not stmt.finally_body
        if new_body is stmt.body and not handlers_changed and not finally_changed:
            return stmt
        return replace(
            stmt,
            body=new_body,
            handlers=tuple(new_handlers),
            finally_body=new_finally,
        )
    if isinstance(stmt, IRSwitch):
        new_arms = []
        arms_changed = False
        for arm in stmt.arms:
            if arm.body is None:
                new_arms.append(arm)
                continue
            new_arm = _gvn_script(arm.body)
            if new_arm is arm.body:
                new_arms.append(arm)
            else:
                new_arms.append(replace(arm, body=new_arm))
                arms_changed = True
        new_default = stmt.default_body
        default_changed = False
        if stmt.default_body is not None:
            new_default = _gvn_script(stmt.default_body)
            default_changed = new_default is not stmt.default_body
        if not arms_changed and not default_changed:
            return stmt
        return replace(stmt, arms=tuple(new_arms), default_body=new_default)
    return stmt


def _is_local(name: str) -> bool:
    """A bare identifier (no ``::``, no array index) — eligible
    GVN target.  Globals stay untouched."""
    return "::" not in name and "(" not in name


def _expr_key(expr) -> tuple | None:
    """Return a hashable key representing ``expr`` plus its source
    variable set, or ``None`` when the expression has any
    dependency the value table can't track (notably embedded
    ``ExprCommand`` substitutions, which can have side effects).
    """
    text = _expr_text(expr)
    if text is None:
        return None
    src_vars = frozenset(vars_in_expr_node(expr))
    return (src_vars, text)


def _expr_text(expr) -> str | None:
    """Render ``expr`` to a stable string for hashing.  Returns
    ``None`` if the tree contains an :class:`~core.compiler.expr_ast.ExprCommand`
    or :class:`~core.compiler.expr_ast.ExprRaw` — both have side
    effects or unknown semantics that GVN can't safely cache.
    """
    from ..expr_ast import (
        ExprBinary,
        ExprCall,
        ExprCommand,
        ExprLiteral,
        ExprRaw,
        ExprString,
        ExprTernary,
        ExprUnary,
        ExprVar,
    )

    if isinstance(expr, ExprCommand):
        return None  # opaque side-effecting subst
    if isinstance(expr, ExprRaw):
        return None  # unparseable — don't trust
    if isinstance(expr, ExprLiteral):
        return f"L:{expr.text}"
    if isinstance(expr, ExprString):
        return f"S:{expr.text}"
    if isinstance(expr, ExprVar):
        return f"V:{expr.name}"
    if isinstance(expr, ExprBinary):
        left = _expr_text(expr.left)
        right = _expr_text(expr.right)
        if left is None or right is None:
            return None
        return f"B:{expr.op.value}({left},{right})"
    if isinstance(expr, ExprUnary):
        op = _expr_text(expr.operand)
        if op is None:
            return None
        return f"U:{expr.op.value}({op})"
    if isinstance(expr, ExprTernary):
        c = _expr_text(expr.condition)
        t = _expr_text(expr.true_branch)
        f = _expr_text(expr.false_branch)
        if c is None or t is None or f is None:
            return None
        return f"T:({c},{t},{f})"
    if isinstance(expr, ExprCall):
        # Tcl ``expr`` functions split into a pure-by-spec majority
        # (``int``, ``abs``, ``sin``, …) and a small denylist of
        # non-deterministic / stateful functions that depend on
        # hidden state.  Caching the latter would collapse two
        # distinct evaluations of e.g. ``rand()`` into one and
        # change observable program output.  Reject any call whose
        # function name appears in the denylist.
        if expr.function in _IMPURE_EXPR_FUNCS:
            return None
        parts: list[str] = []
        for a in expr.args:
            t = _expr_text(a)
            if t is None:
                return None
            parts.append(t)
        return f"F:{expr.function}(" + ",".join(parts) + ")"
    return None


# Tcl ``expr`` functions whose return value depends on hidden mutable
# state.  ``rand()`` reads + advances the PRNG; ``srand(seed)`` resets
# it.  Every other built-in math function is pure.  External / user-
# registered ``expr`` functions are safely opaque too — they're routed
# through the runtime, but a code path emitting an ``ExprCall`` for an
# unknown name would still be wrong to cache because the body could
# read globals; so the safer baseline is "if you're not in the pure
# allowlist, you're impure."  Today we can prove the universe of
# functions reaching here is small (only Tcl built-ins survive the
# IR builder), so a denylist is sufficient.
_IMPURE_EXPR_FUNCS: frozenset[str] = frozenset({"rand", "srand"})


def _stmt_writes(stmt: object) -> set[str]:
    """Return the set of local-variable names ``stmt`` writes.

    The special token ``"*"`` is a "clear-all" sentinel — used for
    calls whose side effects on the caller's locals can't be
    bounded (``eval``, ``uplevel``, ``upvar``, dynamic dispatch).
    Recognised by the caller in ``_gvn_script`` to drop the entire
    value table.
    """
    written: set[str] = set()
    if isinstance(stmt, (IRAssignConst, IRAssignValue, IRAssignExpr, IRIncr)):
        if _is_local(stmt.name):
            written.add(stmt.name)
        # For array writes the rename map covers the array base,
        # but for invalidation purposes the array's other elements
        # might also have changed — clear the base's entry.
        elif "(" in stmt.name and "::" not in stmt.name:
            base = stmt.name.split("(")[0]
            if base:
                written.add(base)
    elif isinstance(stmt, IRCall):
        for d in stmt.defs:
            if _is_local(d):
                written.add(d)
        # PR #237 review: previously every IRCall added the ``"*"``
        # clear-all marker, which was severely pessimistic — a call
        # to ``list``, ``lindex``, ``string length``, etc. cannot
        # mutate any local but the table got wiped anyway.  Most
        # ``set y [expr {…}]; set d [list a b]; set z [expr {…}]``
        # idioms lost their GVN match across the intervening pure
        # call.  Reuse the inliner's splice-safe allow-list (those
        # commands are by definition incapable of writing caller-
        # frame locals).  Anything outside the allow-list — eval,
        # upvar, uplevel, info, unknown user commands, dynamic
        # dispatch — keeps the conservative clear-all behaviour.
        if not _command_is_value_pure(stmt.command):
            written.add("*")
    return written


def _command_is_value_pure(command: str) -> bool:
    """True iff ``command`` is known to neither read nor write any
    caller-frame local through hidden routes (``upvar`` / ``eval``
    / ``info`` / ``unknown``).  Used by GVN to decide whether the
    value table survives the call.

    Implementation reuses the inliner's curated allow-list
    (:data:`core.compiler.inlining.inline_pass._SPLICE_SAFE_COMMANDS`)
    via late import so we don't introduce a top-level dependency
    cycle.  The allow-list is the right shape for this check:
    every command in it is either pure value computation
    (``list``, ``lindex``, ``string``, …) or has only an
    independent observable side effect (``puts``) — neither
    category mutates the caller's locals.
    """

    from ..inlining.inline_pass import _command_is_splice_safe as _safe

    return _safe(command)


# Re-exposing the special "clear-all" marker so the caller can
# branch on it.  Not in ``__all__``; internal use only.
_CLEAR_ALL = "*"
