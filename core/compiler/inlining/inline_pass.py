"""S4.2 — IR-level inlining (v0: empty-body splice).

The catalogue stage (:mod:`core.compiler.inlining.decision`) tags
each :class:`~core.compiler.ir.IRProcedure` with an
:class:`~core.compiler.ir.InlineDecision`. This module is the
mechanism that consumes those tags and rewrites the IR module so
the marked calls disappear before codegen.

**Scope of v0.**  Only the simplest case is handled today: a
statement-position :class:`~core.compiler.ir.IRCall` whose resolved
target is an :data:`~core.compiler.ir.InlineDecision.ALWAYS`-tagged
proc with **zero body statements**.  Such a call has no observable
effect (the proc is ``pure_leaf`` so no side effects, and an empty
body returns the empty string which the call site discards), so
the splice is correct without any α-renaming, return-value
plumbing, or label support.  Larger-body inlining is a follow-up
(it needs IRCall result capture for procs whose return value is
consumed, plus α-renaming for the callee's locals).

The pass returns a new :class:`~core.compiler.ir.IRModule`; the
input module is left untouched (every IR node is frozen, so
rebuilding via :func:`dataclasses.replace` is the only option).
"""

from __future__ import annotations

from dataclasses import replace

from ..ir import (
    IRBlock,
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRIfClause,
    IRModule,
    IRProcedure,
    IRScript,
    IRSwitch,
    IRSwitchArm,
    IRTry,
    IRTryHandler,
    IRUpFrame,
    IRWhile,
    InlineDecision,
)
from ..var_escape import ProcEscapeSummary
from ..var_escape._interprocedural import _resolve_callee


def inline_module(
    module: IRModule,
    summaries: dict[str, ProcEscapeSummary],
) -> IRModule:
    """Return a new module with eligible calls inlined.

    The ``module`` must already carry the S4.1 catalogue (run
    :func:`core.compiler.inlining.decision.apply_inline_catalogue`
    first).  ``summaries`` is the interprocedural var-escape result
    used to resolve unqualified call words to their canonical
    qualified target.

    The pass is idempotent: re-running on a module that has no
    remaining inlinable sites returns an equivalent module.
    Procedures whose static-call-count drops to zero post-inlining
    are NOT yet pruned — keeping them around is harmless and lets
    the catalogue tag survive across re-runs.  A future revision
    can add dead-proc DCE once the inliner handles the wider set
    of cases that make it material.
    """

    inlinable = _build_inlinable_set(module)
    if not inlinable:
        return module

    new_top = _rewrite_script(module.top_level, "::", inlinable, summaries)
    new_procs: dict[str, IRProcedure] = {}
    for qname, proc in module.procedures.items():
        new_body = _rewrite_script(proc.body, qname, inlinable, summaries)
        if new_body is proc.body:
            new_procs[qname] = proc
        else:
            new_procs[qname] = replace(proc, body=new_body)
    return replace(module, top_level=new_top, procedures=new_procs)


def _build_inlinable_set(module: IRModule) -> frozenset[str]:
    """Return the qualified names eligible for the v0 splice rule.

    Eligibility: catalogue tag is ``ALWAYS`` AND the body has no
    statements.  ``IF_SINGLE_CALL`` is **not** v0-eligible because
    its profitability depends on the post-inline proc-pruning that
    isn't implemented yet — inlining without pruning would leave a
    redundant copy of the body in the module.
    """

    eligible: set[str] = set()
    for qname, proc in module.procedures.items():
        if proc.inline_decision is not InlineDecision.ALWAYS:
            continue
        if proc.body.statements:
            continue
        eligible.add(qname)
    return frozenset(eligible)


def _rewrite_script(
    script: IRScript,
    caller_qname: str,
    inlinable: frozenset[str],
    summaries: dict[str, ProcEscapeSummary],
) -> IRScript:
    """Return ``script`` with eligible top-level calls dropped.

    Recurses into every nested control-flow body so calls inside
    ``if`` / ``for`` / ``foreach`` / ``while`` / ``catch`` / ``try``
    / ``switch`` arms are also subject to the splice.  Returns the
    original instance unchanged when no rewrites are needed (cheap
    structural sharing for non-affected scripts).
    """

    new_stmts: list = []
    changed = False
    for stmt in script.statements:
        replacements = _rewrite_stmt(stmt, caller_qname, inlinable, summaries)
        if replacements is None:
            new_stmts.append(stmt)
            continue
        changed = True
        new_stmts.extend(replacements)
    if not changed:
        return script
    return IRScript(statements=tuple(new_stmts))


def _rewrite_stmt(
    stmt: object,
    caller_qname: str,
    inlinable: frozenset[str],
    summaries: dict[str, ProcEscapeSummary],
) -> list | None:
    """Return a replacement statement list, or ``None`` to keep ``stmt``.

    A return value of ``[]`` drops the statement entirely (the v0
    empty-body splice).  A non-empty list replaces the statement
    with multiple successors (reserved for future v1 splicing —
    today only the empty case is exercised).
    """

    if isinstance(stmt, IRCall):
        target = _resolve_callee(stmt.command, caller_qname, summaries)
        if target is not None and target in inlinable:
            # v0: the eligible callee has an empty body, so the
            # whole call vanishes.  Argument expressions are pure
            # by definition (the IR doesn't side-effect on
            # argument evaluation; substitution happens at the
            # value level), so dropping them is sound.
            return []
        return None

    if isinstance(stmt, IRBlock):
        new_body = _rewrite_script(stmt.body, caller_qname, inlinable, summaries)
        if new_body is stmt.body:
            return None
        return [replace(stmt, body=new_body)]

    if isinstance(stmt, IRIf):
        new_clauses, clauses_changed = _rewrite_if_clauses(
            stmt.clauses, caller_qname, inlinable, summaries
        )
        new_else = stmt.else_body
        else_changed = False
        if stmt.else_body is not None:
            new_else = _rewrite_script(stmt.else_body, caller_qname, inlinable, summaries)
            else_changed = new_else is not stmt.else_body
        if not clauses_changed and not else_changed:
            return None
        return [replace(stmt, clauses=new_clauses, else_body=new_else)]

    if isinstance(stmt, IRFor):
        new_init = _rewrite_script(stmt.init, caller_qname, inlinable, summaries)
        new_next = _rewrite_script(stmt.next, caller_qname, inlinable, summaries)
        new_body = _rewrite_script(stmt.body, caller_qname, inlinable, summaries)
        if (
            new_init is stmt.init
            and new_next is stmt.next
            and new_body is stmt.body
        ):
            return None
        return [replace(stmt, init=new_init, next=new_next, body=new_body)]

    if isinstance(stmt, (IRWhile, IRForeach, IRCatch, IRUpFrame)):
        new_body = _rewrite_script(stmt.body, caller_qname, inlinable, summaries)
        if new_body is stmt.body:
            return None
        return [replace(stmt, body=new_body)]

    if isinstance(stmt, IRTry):
        new_body = _rewrite_script(stmt.body, caller_qname, inlinable, summaries)
        new_handlers, handlers_changed = _rewrite_try_handlers(
            stmt.handlers, caller_qname, inlinable, summaries
        )
        new_finally = stmt.finally_body
        finally_changed = False
        if stmt.finally_body is not None:
            new_finally = _rewrite_script(
                stmt.finally_body, caller_qname, inlinable, summaries
            )
            finally_changed = new_finally is not stmt.finally_body
        if (
            new_body is stmt.body
            and not handlers_changed
            and not finally_changed
        ):
            return None
        return [
            replace(
                stmt,
                body=new_body,
                handlers=new_handlers,
                finally_body=new_finally,
            )
        ]

    if isinstance(stmt, IRSwitch):
        new_arms, arms_changed = _rewrite_switch_arms(
            stmt.arms, caller_qname, inlinable, summaries
        )
        new_default = stmt.default_body
        default_changed = False
        if stmt.default_body is not None:
            new_default = _rewrite_script(
                stmt.default_body, caller_qname, inlinable, summaries
            )
            default_changed = new_default is not stmt.default_body
        if not arms_changed and not default_changed:
            return None
        return [replace(stmt, arms=new_arms, default_body=new_default)]

    return None


def _rewrite_if_clauses(
    clauses: tuple[IRIfClause, ...],
    caller_qname: str,
    inlinable: frozenset[str],
    summaries: dict[str, ProcEscapeSummary],
) -> tuple[tuple[IRIfClause, ...], bool]:
    new_clauses = []
    changed = False
    for clause in clauses:
        new_body = _rewrite_script(clause.body, caller_qname, inlinable, summaries)
        if new_body is clause.body:
            new_clauses.append(clause)
        else:
            new_clauses.append(replace(clause, body=new_body))
            changed = True
    return tuple(new_clauses), changed


def _rewrite_try_handlers(
    handlers: tuple[IRTryHandler, ...],
    caller_qname: str,
    inlinable: frozenset[str],
    summaries: dict[str, ProcEscapeSummary],
) -> tuple[tuple[IRTryHandler, ...], bool]:
    new_handlers = []
    changed = False
    for handler in handlers:
        new_body = _rewrite_script(handler.body, caller_qname, inlinable, summaries)
        if new_body is handler.body:
            new_handlers.append(handler)
        else:
            new_handlers.append(replace(handler, body=new_body))
            changed = True
    return tuple(new_handlers), changed


def _rewrite_switch_arms(
    arms: tuple[IRSwitchArm, ...],
    caller_qname: str,
    inlinable: frozenset[str],
    summaries: dict[str, ProcEscapeSummary],
) -> tuple[tuple[IRSwitchArm, ...], bool]:
    new_arms = []
    changed = False
    for arm in arms:
        if arm.body is None:
            new_arms.append(arm)
            continue
        new_body = _rewrite_script(arm.body, caller_qname, inlinable, summaries)
        if new_body is arm.body:
            new_arms.append(arm)
        else:
            new_arms.append(replace(arm, body=new_body))
            changed = True
    return tuple(new_arms), changed
