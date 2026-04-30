"""S4.1 — inlining-eligibility policy.

For every :class:`~core.compiler.ir.IRProcedure` in an
:class:`~core.compiler.ir.IRModule`, decide whether the inliner is
allowed (and willing) to splice the body into a caller in the next
stage S4.2.  The output is purely metadata — this module never
mutates the IR shape, only the per-proc ``inline_decision`` and
``static_call_count`` tags.

The policy combines two inputs:

* ``ProcEscapeSummary.pure_leaf`` from
  :mod:`core.compiler.var_escape` — the soundness predicate.  A proc
  that isn't ``pure_leaf`` may have unbounded behaviour (eval
  fallback, ``upvar`` source, dynamic barrier) and is not safe to
  splice.

* Body size and static call count — the profitability heuristics.
  A 1-statement ``pure_leaf`` proc is always profitable.  A larger
  one is only profitable if it has exactly one static caller, where
  inlining shrinks the module rather than growing it.

The S4.2 inliner consumes ``inline_decision`` and never re-reads the
underlying summary; that lets the policy evolve independently of
the mechanism.
"""

from __future__ import annotations

from dataclasses import replace
from typing import Iterable

from ..ir import (
    InlineDecision,
    IRBlock,
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRModule,
    IRProcedure,
    IRScript,
    IRSwitch,
    IRTry,
    IRUpFrame,
    IRWhile,
)
from ..var_escape import ProcEscapeSummary
from ..var_escape._interprocedural import _resolve_callee

# A pure-leaf proc whose body has at most this many top-level
# statements is unconditionally inlinable.  Counting is *flat* — the
# body of a nested ``if``/``for`` contributes its own statements
# rather than the loop counting as one — to approximate code-size
# growth at the call site.
SMALL_BODY_THRESHOLD: int = 5


def count_statements(script: IRScript | None) -> int:
    """Return the total number of leaf statements reachable in ``script``.

    Nested control-flow bodies are walked transitively so the count
    reflects the inlined code-size cost rather than the syntactic
    statement count.  Synthetic wrapper nodes (``IRBlock``) don't
    count themselves but their inner statements do.
    """

    if script is None:
        return 0
    total = 0
    for stmt in script.statements:
        total += _count_one(stmt)
    return total


def _count_one(stmt: object) -> int:
    if isinstance(stmt, IRBlock):
        return count_statements(stmt.body)
    if isinstance(stmt, IRIf):
        n = 0
        for clause in stmt.clauses:
            n += 1 + count_statements(clause.body)
        if stmt.else_body is not None:
            n += count_statements(stmt.else_body)
        return n
    if isinstance(stmt, IRFor):
        return (
            count_statements(stmt.init)
            + 1
            + count_statements(stmt.next)
            + count_statements(stmt.body)
        )
    if isinstance(stmt, IRWhile):
        return 1 + count_statements(stmt.body)
    if isinstance(stmt, IRForeach):
        return 1 + count_statements(stmt.body)
    if isinstance(stmt, IRCatch):
        return 1 + count_statements(stmt.body)
    if isinstance(stmt, IRTry):
        n = count_statements(stmt.body)
        for handler in stmt.handlers:
            n += count_statements(handler.body)
        if stmt.finally_body is not None:
            n += count_statements(stmt.finally_body)
        return n
    if isinstance(stmt, IRSwitch):
        n = 0
        for arm in stmt.arms:
            n += count_statements(arm.body)
        if stmt.default_body is not None:
            n += count_statements(stmt.default_body)
        return n
    if isinstance(stmt, IRUpFrame):
        return count_statements(stmt.body)
    return 1


def _walk_calls(script: IRScript | None) -> Iterable[IRCall]:
    """Yield every :class:`IRCall` reachable from ``script``."""

    if script is None:
        return
    stack: list[object] = list(script.statements)
    while stack:
        node = stack.pop()
        if isinstance(node, IRCall):
            yield node
            continue
        if isinstance(node, IRBlock):
            stack.extend(node.body.statements)
            continue
        if isinstance(node, IRIf):
            for clause in node.clauses:
                stack.extend(clause.body.statements)
            if node.else_body is not None:
                stack.extend(node.else_body.statements)
            continue
        if isinstance(node, IRFor):
            stack.extend(node.init.statements)
            stack.extend(node.next.statements)
            stack.extend(node.body.statements)
            continue
        if isinstance(node, (IRWhile, IRForeach, IRCatch, IRUpFrame)):
            stack.extend(node.body.statements)
            continue
        if isinstance(node, IRTry):
            stack.extend(node.body.statements)
            for handler in node.handlers:
                stack.extend(handler.body.statements)
            if node.finally_body is not None:
                stack.extend(node.finally_body.statements)
            continue
        if isinstance(node, IRSwitch):
            for arm in node.arms:
                if arm.body is not None:
                    stack.extend(arm.body.statements)
            if node.default_body is not None:
                stack.extend(node.default_body.statements)
            continue


def count_static_calls(
    module: IRModule,
    summaries: dict[str, ProcEscapeSummary],
) -> dict[str, int]:
    """Return ``{qname: static_call_count}`` over the whole module.

    Walks the top-level script and every proc body.  An ``IRCall``
    whose ``command`` resolves (per the same namespace-walk rules
    the interprocedural pass uses) to a tracked proc contributes
    ``+1`` to that proc's count.  Calls to unknown commands or
    builtins don't count — they are not inlining candidates.

    The map contains an entry for every proc in ``module.procedures``,
    defaulting to ``0`` so callers can blindly look up by qname.
    """

    counts: dict[str, int] = {qname: 0 for qname in module.procedures}

    def tally(caller_qname: str, script: IRScript | None) -> None:
        if script is None:
            return
        for call in _walk_calls(script):
            target = _resolve_callee(call.command, caller_qname, summaries)
            if target is not None and target in counts:
                counts[target] += 1

    # Top-level callers live in the global namespace ``::``.
    tally("::", module.top_level)
    for qname, proc in module.procedures.items():
        tally(qname, proc.body)
    return counts


def classify_proc(
    proc: IRProcedure,
    summary: ProcEscapeSummary | None,
    static_call_count: int,
    *,
    small_threshold: int = SMALL_BODY_THRESHOLD,
) -> InlineDecision:
    """Apply the S4.1 policy to a single proc.

    A proc is inlinable only if its escape summary is ``pure_leaf``.
    Among the ``pure_leaf`` set, the size and static-call-count
    heuristics decide between ``ALWAYS`` (small enough that growth
    is bounded), ``IF_SINGLE_CALL`` (only one static caller, so
    inlining is size-neutral), and ``NEVER``.
    """

    if summary is None or not summary.pure_leaf:
        return InlineDecision.NEVER
    body_size = count_statements(proc.body)
    if body_size <= small_threshold:
        return InlineDecision.ALWAYS
    if static_call_count == 1:
        return InlineDecision.IF_SINGLE_CALL
    return InlineDecision.NEVER


def apply_inline_catalogue(
    module: IRModule,
    summaries: dict[str, ProcEscapeSummary],
) -> IRModule:
    """Return a new ``IRModule`` with every proc tagged for inlining.

    The input ``module`` is left untouched.  ``IRProcedure`` is
    frozen, so we rebuild each entry via :func:`dataclasses.replace`
    with the freshly computed ``inline_decision`` and
    ``static_call_count`` fields.

    ``summaries`` is the *interprocedural* result from
    :func:`core.compiler.var_escape.solve_interprocedural_escape` —
    only the post-fixpoint ``pure_leaf`` flag is sound (the
    intraproc pass can be over-optimistic about callees).
    """

    call_counts = count_static_calls(module, summaries)
    new_procs: dict[str, IRProcedure] = {}
    for qname, proc in module.procedures.items():
        summary = summaries.get(qname)
        count = call_counts.get(qname, 0)
        decision = classify_proc(proc, summary, count)
        new_procs[qname] = replace(
            proc,
            inline_decision=decision,
            static_call_count=count,
        )
    return replace(module, procedures=new_procs)
