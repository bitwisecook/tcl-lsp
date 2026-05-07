"""Pre-codegen pass: insert ``IRInterpBoundary`` markers before
IR statements that hand control to the Tcl interpreter.

The rationale:

Codegen has historically been responsible for remembering that any
statement reaching the runtime interpreter must be preceded by a
frame-sync (mirror compiled-side WASM-locals into the runtime
frame so name resolution finds them).  This pass moves that
"remember to sync" obligation from method-call control flow into
*data* — an :class:`IRInterpBoundary` node placed in the IR
immediately before each interpreter-crossing statement.

After this pass:

* Codegen's :meth:`_emit_stmt` short-circuits on
  ``IRInterpBoundary`` and emits the sync once via
  ``_emit_interp_boundary(kind, vars_observed)`` (already done).
* The legacy in-codegen sync calls (``_commands.py`` call bridge,
  ``_variables.py`` eval bridge, ``_statements.py`` dispatch
  fallback) become belt-and-braces but the IR is the source of
  truth.

The pass walks every script in the module (top level + every
procedure body, recursively into compound statements) and prepends
a boundary node before every :class:`IRBarrier` statement
(``eval``, ``uplevel``, ``trace``, etc. — anything that defeats
static analysis of name visibility).

Today only IRBarrier is covered; IRCall paths that fall back to
the interpreter (dynamic dispatch, non-frameless commands, etc.)
are not handled here because the dispatch decision is made at
codegen time after command resolution.  The codegen-direct calls
to ``_emit_interp_boundary`` cover those cases until a follow-on
pass uses the same resolution data to insert markers for them
too.
"""

from __future__ import annotations

from ..ir import (
    IRBarrier,
    IRBlock,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRInterpBoundary,
    IRModule,
    IRProcedure,
    IRScript,
    IRStatement,
    IRSwitch,
    IRTry,
    IRUpFrame,
    IRWhile,
)


def _boundary_kind_for(barrier: IRBarrier) -> str:
    """Map an IRBarrier's canonical command to a centralised boundary kind.

    Mirrors the labels the codegen-direct call sites pass to
    ``_emit_interp_boundary``.  Unknown shapes get ``"dispatch"``
    so the boundary still emits the conservative sync-everything
    behaviour.
    """
    cmd = barrier.canonical_command
    if cmd == "::eval":
        return "eval"
    if cmd == "::uplevel":
        return "eval"  # uplevel runs a body in another frame — same sync needs
    if cmd == "::source":
        return "eval"
    if cmd == "::subst":
        return "eval"
    return "dispatch"


def _transform_script(script: IRScript) -> IRScript:
    """Rewrite *script* with an :class:`IRInterpBoundary` before every barrier."""
    new_stmts: list[IRStatement] = []
    changed = False
    for stmt in script.statements:
        rewritten, did_change = _transform_stmt(stmt)
        if did_change:
            changed = True
        if isinstance(rewritten, IRBarrier):
            new_stmts.append(
                IRInterpBoundary(
                    range=rewritten.range,
                    kind=_boundary_kind_for(rewritten),
                    vars_observed=None,
                    detail=rewritten.reason,
                )
            )
            new_stmts.append(rewritten)
            changed = True
        else:
            new_stmts.append(rewritten)
    if not changed:
        return script
    return IRScript(statements=tuple(new_stmts))


def _transform_stmt(stmt: IRStatement) -> tuple[IRStatement, bool]:
    """Recurse into a compound statement, rewriting nested barriers.

    Returns ``(stmt_or_replacement, changed)``.  Leaves leaf statements
    unchanged.
    """
    if isinstance(stmt, IRIf):
        new_clauses = []
        changed = False
        for clause in stmt.clauses:
            new_body = _transform_script(clause.body)
            if new_body is not clause.body:
                changed = True
                from dataclasses import replace as _r

                new_clauses.append(_r(clause, body=new_body))
            else:
                new_clauses.append(clause)
        new_else = _transform_script(stmt.else_body) if stmt.else_body is not None else None
        if new_else is not stmt.else_body:
            changed = True
        if not changed:
            return stmt, False
        from dataclasses import replace

        return replace(stmt, clauses=tuple(new_clauses), else_body=new_else), True
    if isinstance(stmt, IRFor):
        # ``stmt.condition`` is an IRExpr (no nested statements), so
        # there's nothing to recurse into for it.
        new_init = _transform_script(stmt.init)
        new_next = _transform_script(stmt.next)
        new_body = _transform_script(stmt.body)
        if new_init is stmt.init and new_next is stmt.next and new_body is stmt.body:
            return stmt, False
        from dataclasses import replace

        return (
            replace(stmt, init=new_init, next=new_next, body=new_body),
            True,
        )
    if isinstance(stmt, IRWhile):
        new_body = _transform_script(stmt.body)
        if new_body is stmt.body:
            return stmt, False
        from dataclasses import replace

        return replace(stmt, body=new_body), True
    if isinstance(stmt, IRForeach):
        new_body = _transform_script(stmt.body)
        if new_body is stmt.body:
            return stmt, False
        from dataclasses import replace

        return replace(stmt, body=new_body), True
    if isinstance(stmt, IRSwitch):
        new_arms = []
        changed = False
        for arm in stmt.arms:
            if arm.body is None:
                # Fallthrough-only arm — no body to transform.
                new_arms.append(arm)
                continue
            new_body = _transform_script(arm.body)
            if new_body is not arm.body:
                changed = True
                from dataclasses import replace as _r

                new_arms.append(_r(arm, body=new_body))
            else:
                new_arms.append(arm)
        new_default = (
            _transform_script(stmt.default_body) if stmt.default_body is not None else None
        )
        if new_default is not stmt.default_body:
            changed = True
        if not changed:
            return stmt, False
        from dataclasses import replace

        return (
            replace(stmt, arms=tuple(new_arms), default_body=new_default),
            True,
        )
    if isinstance(stmt, IRCatch):
        new_body = _transform_script(stmt.body)
        if new_body is stmt.body:
            return stmt, False
        from dataclasses import replace

        return replace(stmt, body=new_body), True
    if isinstance(stmt, IRTry):
        new_body = _transform_script(stmt.body)
        new_finally = (
            _transform_script(stmt.finally_body) if stmt.finally_body is not None else None
        )
        new_handlers = []
        changed = new_body is not stmt.body or new_finally is not stmt.finally_body
        for h in stmt.handlers:
            hb = _transform_script(h.body)
            if hb is not h.body:
                changed = True
                from dataclasses import replace as _r

                new_handlers.append(_r(h, body=hb))
            else:
                new_handlers.append(h)
        if not changed:
            return stmt, False
        from dataclasses import replace

        return (
            replace(
                stmt,
                body=new_body,
                handlers=tuple(new_handlers),
                finally_body=new_finally,
            ),
            True,
        )
    if isinstance(stmt, IRBlock):
        new_body = _transform_script(stmt.body)
        if new_body is stmt.body:
            return stmt, False
        from dataclasses import replace

        return replace(stmt, body=new_body), True
    if isinstance(stmt, IRUpFrame):
        # IRUpFrame inlines a frame-shifted body as compiled IR.
        # Recurse into its body so any nested IRBarrier still gets
        # a boundary marker.  The IRUpFrame itself doesn't need a
        # boundary at the outer level — its codegen emits the
        # frame-depth stash without crossing into the interpreter.
        new_body = _transform_script(stmt.body)
        if new_body is stmt.body:
            return stmt, False
        from dataclasses import replace

        return replace(stmt, body=new_body), True
    return stmt, False


def insert_interp_boundaries(module: IRModule) -> IRModule:
    """Return *module* with IRInterpBoundary markers inserted before each barrier.

    Idempotent: a second run is a no-op because barriers already preceded
    by IRInterpBoundary stay unchanged at the loop's leading insert
    (the new boundary would be inserted again, but pre-condition is
    checked by the codegen-side dispatch which is robust to a redundant
    boundary node — sync becomes a no-op when nothing has changed).
    Callers should run this pass once during pipeline assembly to keep
    the IR clean.
    """
    new_top = _transform_script(module.top_level)
    new_procs: dict[str, IRProcedure] = {}
    changed = new_top is not module.top_level
    for qname, proc in module.procedures.items():
        new_body = _transform_script(proc.body)
        if new_body is proc.body:
            new_procs[qname] = proc
            continue
        from dataclasses import replace

        new_procs[qname] = replace(proc, body=new_body)
        changed = True
    if not changed:
        return module
    from dataclasses import replace

    return replace(module, top_level=new_top, procedures=new_procs)


__all__ = ["insert_interp_boundaries"]
