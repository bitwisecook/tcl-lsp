"""Interprocedural propagation of var-escape summaries.

The intraprocedural pass (``analyse_script``) records each proc's own
``upvar_source_names`` — the literal caller-frame names it reaches via
``upvar <positive-level>``. A caller whose local name matches must
spill that local to the frame so the callee's alias can resolve.

This module runs a worklist fixpoint over the static call graph built
from each proc's ``direct_callees``. After convergence, each proc's
``upvar_source_names`` is the union of its own sources and every
transitive callee's sources. The caller's summary is then augmented:
any of its local vars whose names appear in the callee set are marked
``FRAME``.

``unbounded_upvar_source`` propagates the same way — an unbounded
callee forces every caller in its reachable set to the pessimistic
fallback. (The interprocedural ``dynamic_barrier`` rule.)
"""

from __future__ import annotations

from collections import deque

from ._types import ProcEscapeSummary


def _caller_namespace(qname: str) -> str:
    """Return the enclosing namespace of ``qname``.

    ``::ns::caller`` → ``::ns``; ``::caller`` → ``::``; bare names →
    ``::``. Mirrors the simple suffix-strip model the compiler uses
    elsewhere for namespace resolution.
    """
    if not qname or qname == "::":
        return "::"
    if not qname.startswith("::"):
        return "::"
    trimmed = qname[2:]
    idx = trimmed.rfind("::")
    if idx < 0:
        return "::"
    return "::" + trimmed[:idx]


def _name_candidates(command: str, caller_qname: str) -> tuple[str, ...]:
    """Return the candidate names ``command`` might resolve to in ``summaries``.

    Tcl name lookup for a bare call word searches:

    1. the caller's namespace (``::ns::caller`` calling ``leaf`` →
       ``::ns::leaf``);
    2. walks up each enclosing namespace (``::ns::leaf`` → ``::leaf``);
    3. the global namespace (``::leaf``);
    4. the raw bare form the IR recorded.

    An already-qualified call (starts with ``::``) skips the
    namespace walk — it's absolute.
    """
    if command.startswith("::"):
        return (command,)
    candidates: list[str] = []
    ns = _caller_namespace(caller_qname)
    while True:
        prefix = "" if ns == "::" else ns
        candidate = f"{prefix}::{command}" if prefix else f"::{command}"
        if candidate not in candidates:
            candidates.append(candidate)
        if ns == "::":
            break
        ns = _caller_namespace(ns)
    if command not in candidates:
        candidates.append(command)
    return tuple(candidates)


def _resolve_callee(
    command: str,
    caller_qname: str,
    summaries: dict[str, ProcEscapeSummary],
) -> str | None:
    """Resolve a command word to a qualified proc name in ``summaries``."""
    for candidate in _name_candidates(command, caller_qname):
        if candidate in summaries:
            return candidate
    return None


def solve_interprocedural_escape(
    summaries: dict[str, ProcEscapeSummary],
) -> dict[str, ProcEscapeSummary]:
    """Return a new map of summaries with callee-induced escapes folded in.

    The input ``summaries`` is the per-proc (intraprocedural) result.
    The output is keyed identically; each value is the interprocedural
    summary suitable for codegen consumption.
    """
    if not summaries:
        return {}

    # Callees keyed by qualified name → set of qualified callee names
    # that are actually present in ``summaries``. Commands that don't
    # resolve to a tracked proc (builtins, unknown) are dropped — they
    # either can't escape (builtins) or are handled by the
    # intraprocedural pessimistic fallback.
    resolved_callees: dict[str, set[str]] = {}
    for qname, summary in summaries.items():
        callees: set[str] = set()
        for cmd in summary.direct_callees:
            resolved = _resolve_callee(cmd, qname, summaries)
            if resolved is not None and resolved != qname:
                callees.add(resolved)
        resolved_callees[qname] = callees

    # Transitive closure per proc: union own + callees'.
    # Worklist seeded with every proc; iterate to a fixpoint.
    #
    # ``has_fallback`` is **not** propagated. It is consumed by codegen
    # to decide whether the proc body itself ever dispatches through
    # ``tcl_eval`` (and therefore needs a runtime frame). A compiled
    # callee's fallback is entirely the callee's business — caller-to-
    # callee dispatch goes through a direct WASM call, not the eval
    # fallback. Unioning would force a frame on every caller of every
    # non-leaf proc, defeating the elision optimisation.
    transitive_sources: dict[str, set[str]] = {
        qname: set(summary.upvar_source_names) for qname, summary in summaries.items()
    }
    transitive_unbounded: dict[str, bool] = {
        qname: summary.unbounded_upvar_source for qname, summary in summaries.items()
    }

    # Reverse edges: qname → set of qnames that call it.
    callers_of: dict[str, set[str]] = {qname: set() for qname in summaries}
    for qname, callees in resolved_callees.items():
        for callee in callees:
            callers_of.setdefault(callee, set()).add(qname)

    worklist: deque[str] = deque(summaries.keys())
    in_worklist: set[str] = set(summaries.keys())
    while worklist:
        qname = worklist.popleft()
        in_worklist.discard(qname)
        # Propagate this proc's current sources into its callers.
        current_sources = transitive_sources[qname]
        current_unbounded = transitive_unbounded[qname]
        for caller in callers_of.get(qname, ()):
            changed = False
            if not current_sources.issubset(transitive_sources[caller]):
                transitive_sources[caller].update(current_sources)
                changed = True
            if current_unbounded and not transitive_unbounded[caller]:
                transitive_unbounded[caller] = True
                changed = True
            if changed and caller not in in_worklist:
                worklist.append(caller)
                in_worklist.add(caller)

    # For soundness we propagate every transitive source name onto the
    # caller summary. Names already present in ``tags`` are marked
    # ``FRAME`` immediately by ``with_escapes``; names not yet present
    # in the caller's tag map are still carried in
    # ``upvar_source_names`` so the emitter can apply the mark lazily
    # when that local is later interned. Callers with
    # ``unbounded_upvar_source`` must also spill every local — we
    # signal that via ``dynamic_barrier``.
    #
    # ``has_fallback`` is intentionally NOT touched here — it stays
    # the intraprocedural value the per-proc walk recorded.
    # Derive the final ``has_fallback`` for each proc:
    #   final = intraproc has_fallback           (definite reasons)
    #         | has_call_fallback & not all-callees-resolve
    #
    # A proc whose only non-frameless IRCalls resolve to compiled
    # procs (direct WASM calls) doesn't hit the eval fallback on
    # those calls, so the call-based ``has_call_fallback`` is
    # downgraded to False.  Any definite reason
    # (IRBarrier / ``{*}`` / non-frameless ``[cmd subst]`` / dynamic
    # command word) is preserved.
    downgraded_fallback: dict[str, bool] = {}
    for qname, summary in summaries.items():
        if summary.has_call_fallback:
            all_resolve = all(
                _resolve_callee(cmd, qname, summaries) is not None for cmd in summary.direct_callees
            )
            call_fallback_final = not all_resolve
        else:
            call_fallback_final = False
        downgraded_fallback[qname] = summary.has_fallback or call_fallback_final

    # For soundness we propagate every transitive source name onto the
    # caller summary. Names already present in ``tags`` are marked
    # ``FRAME`` immediately by ``with_escapes``; names not yet present
    # in the caller's tag map are still carried in
    # ``upvar_source_names`` so the emitter can apply the mark lazily
    # when that local is later interned. Callers with
    # ``unbounded_upvar_source`` must also spill every local — we
    # signal that via ``dynamic_barrier``.
    #
    # ``has_fallback`` is NOT propagated transitively; a callee's
    # interpreter dispatch is entirely its own business.  It may,
    # however, be downgraded for the current proc if every recorded
    # callee resolves to a compiled proc (see ``downgraded_fallback``).
    result: dict[str, ProcEscapeSummary] = {}
    for qname, summary in summaries.items():
        sources = transitive_sources[qname]
        unbounded = transitive_unbounded[qname]
        # Propagate the full transitive callee source set into this
        # caller. ``with_escapes`` applies immediate ``FRAME`` marks to
        # any matching existing tags; the promoted transitive fields
        # below let the emitter catch later-interned locals of the
        # same name.
        extra_escapes = set(sources)
        pessimistic = unbounded and not summary.dynamic_barrier
        new_summary = summary.with_escapes(extra_escapes, pessimistic=pessimistic)
        new_summary = _replace_transitive_fields(
            new_summary,
            upvar_source_names=frozenset(sources),
            unbounded_upvar_source=unbounded,
            has_fallback=downgraded_fallback[qname],
        )
        result[qname] = new_summary
    return result


def _replace_transitive_fields(
    summary: ProcEscapeSummary,
    *,
    upvar_source_names: frozenset[str],
    unbounded_upvar_source: bool,
    has_fallback: bool,
) -> ProcEscapeSummary:
    """Return a copy of ``summary`` with the transitive fields set.

    ``ProcEscapeSummary`` is frozen; this helper avoids sprinkling
    ``dataclasses.replace`` calls throughout the fixpoint driver.

    ``has_fallback`` here is the interprocedurally-derived value —
    the intraproc definite reasons plus any ``has_call_fallback``
    callees that failed to resolve. It is **not** a transitive
    union from callees.
    """
    from dataclasses import replace

    return replace(
        summary,
        upvar_source_names=upvar_source_names,
        unbounded_upvar_source=unbounded_upvar_source,
        has_fallback=has_fallback,
    )
