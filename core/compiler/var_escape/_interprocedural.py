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


def _name_candidates(qname: str) -> tuple[str, ...]:
    """Return the names a ``call.command`` might resolve to in the map.

    Lowering sometimes records commands with the bare name (``foo``) and
    sometimes with a qualified name (``::foo`` or ``::ns::foo``). Callees
    live in the summary map under their *qualified* name, so try the
    bare name first (no prefix) and a ``::``-prefixed variant.
    """
    if qname.startswith("::"):
        return (qname,)
    return (f"::{qname}", qname)


def _resolve_callee(
    command: str,
    summaries: dict[str, ProcEscapeSummary],
) -> str | None:
    """Resolve a command word to a qualified proc name in ``summaries``."""
    for candidate in _name_candidates(command):
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
            resolved = _resolve_callee(cmd, summaries)
            if resolved is not None and resolved != qname:
                callees.add(resolved)
        resolved_callees[qname] = callees

    # Transitive closure per proc: union own + callees'.
    # Worklist seeded with every proc; iterate to a fixpoint.
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

    # Build final summaries: fold transitive sources into ``tags`` by
    # looking at each proc's known-name set (inferred from summary.tags
    # plus any other name the codegen will touch). The per-proc
    # intraprocedural summary already has its known-name set captured
    # in ``tags`` keys (FRAME names) plus the proc's parameters and
    # local assigns — but those last two aren't in the summary today.
    # For soundness we escape any transitive source that matches a
    # name already present in ``tags`` OR that the caller's unbounded
    # flag is set. Callers with ``unbounded_upvar_source`` must also
    # spill every local — we signal that via ``dynamic_barrier``.
    result: dict[str, ProcEscapeSummary] = {}
    for qname, summary in summaries.items():
        sources = transitive_sources[qname]
        unbounded = transitive_unbounded[qname]
        # Spill any name in the caller's tags whose name is in the
        # callee source set. We spill conservatively: callers that
        # don't know their full local-name set (empty tags) gain the
        # FRAME marks lazily when the emitter interns a local of that
        # name — see ``emitter_escape_lookup`` below.
        extra_escapes = set(sources)  # names to FRAME in this caller
        pessimistic = unbounded and not summary.dynamic_barrier
        # Attach the transitive source set to the new summary for the
        # emitter to consult at ``_intern_local`` time, catching locals
        # that weren't in the intraprocedural summary's tag map.
        new_summary = summary.with_escapes(extra_escapes, pessimistic=pessimistic)
        # Promote the transitive fields onto the summary so the codegen
        # can look up "is this name named by any callee's upvar?".
        new_summary = _replace_transitive_fields(
            new_summary,
            upvar_source_names=frozenset(sources),
            unbounded_upvar_source=unbounded,
        )
        result[qname] = new_summary
    return result


def _replace_transitive_fields(
    summary: ProcEscapeSummary,
    *,
    upvar_source_names: frozenset[str],
    unbounded_upvar_source: bool,
) -> ProcEscapeSummary:
    """Return a copy of ``summary`` with the two transitive fields set.

    ``ProcEscapeSummary`` is frozen; this helper avoids sprinkling
    ``dataclasses.replace`` calls throughout the fixpoint driver.
    """
    from dataclasses import replace

    return replace(
        summary,
        upvar_source_names=upvar_source_names,
        unbounded_upvar_source=unbounded_upvar_source,
    )
