"""Cross-event variable scope analysis for iRules connection lifecycles.

iRules ``when`` event handlers share a connection-scoped Tcl stack —
variables set in one event persist until the connection closes or the
variable is explicitly ``unset``.  This module computes which local
variables flow across ``when`` event boundaries so that per-procedure
diagnostics (dead stores, read-before-set, unused variables) can be
suppressed when the variable is actually live across events.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

from ..commands.registry.namespace_registry import NAMESPACE_REGISTRY as EVENT_REGISTRY
from .ir import IRCall

if TYPE_CHECKING:
    from .compilation_unit import FunctionUnit
    from .ir import IRModule


@dataclass(frozen=True, slots=True)
class EventVarSummary:
    """Variable summary for a single ``when`` event handler."""

    event: str
    defs: frozenset[str]
    """Variable names defined (set) on at least one path through the event."""
    uses_before_def: frozenset[str]
    """Variable names used at SSA version 0 (read before any local def)."""
    unsets: frozenset[str]
    """Variable names explicitly ``unset`` in this event."""


@dataclass(frozen=True, slots=True)
class ConnectionScope:
    """Cross-event variable scope analysis result."""

    summaries: dict[str, EventVarSummary]
    """Event name → variable summary."""
    cross_event_defs: frozenset[str]
    """Variables defined in one event AND used-before-def in a different event."""
    cross_event_imports: frozenset[str]
    """Variables used-before-def in one event AND defined in a different event."""


def _extract_event_summary(
    event: str,
    fu: "FunctionUnit",
) -> EventVarSummary:
    """Build a variable summary from a compiled ``when`` procedure."""
    defs: set[str] = set()
    uses_v0: set[str] = set()
    unsets: set[str] = set()

    for block in fu.ssa.blocks.values():
        for stmt in block.statements:
            for name in stmt.defs:
                if name.startswith("::") or name.startswith("static::"):
                    continue
                defs.add(name)
            for name, ver in stmt.uses.items():
                if ver == 0 and not name.startswith("::") and not name.startswith("static::"):
                    uses_v0.add(name)
            # Track unsets
            ir_stmt = stmt.statement
            if isinstance(ir_stmt, IRCall) and ir_stmt.command == "unset":
                for name in ir_stmt.defs:
                    if not name.startswith("::") and not name.startswith("static::"):
                        unsets.add(name)

    # Also check branch conditions for version-0 uses
    from .cfg import CFGBranch
    from .expr_ast import vars_in_expr_node

    for bn, ssa_block in fu.ssa.blocks.items():
        cfg_block = fu.cfg.blocks.get(bn)
        if cfg_block is None:
            continue
        term = cfg_block.terminator
        if isinstance(term, CFGBranch):
            for name in vars_in_expr_node(term.condition):
                if name.startswith("::") or name.startswith("static::"):
                    continue
                ver = ssa_block.exit_versions.get(name, 0)
                if ver == 0:
                    uses_v0.add(name)

    return EventVarSummary(
        event=event,
        defs=frozenset(defs),
        uses_before_def=frozenset(uses_v0),
        unsets=frozenset(unsets),
    )


def build_connection_scope(
    when_procedures: dict[str, "FunctionUnit"],
    ir_module: "IRModule",
) -> ConnectionScope:
    """Compute cross-event variable scope from compiled ``when`` procedures.

    *when_procedures* should be the subset of ``CompilationUnit.procedures``
    whose qualified names start with ``::when::``.
    """
    summaries: dict[str, EventVarSummary] = {}
    for qname, fu in when_procedures.items():
        event = qname.removeprefix("::when::")
        summaries[event] = _extract_event_summary(event, fu)

    # Build cross-event sets with event-ordering awareness.
    cross_defs: set[str] = set()
    cross_imports: set[str] = set()

    events = list(summaries.keys())
    for i, ev_a in enumerate(events):
        sum_a = summaries[ev_a]
        for j, ev_b in enumerate(events):
            if i == j:
                continue
            sum_b = summaries[ev_b]

            # Variables defined in A and used-before-def in B
            shared = sum_a.defs & sum_b.uses_before_def
            for var in shared:
                note = EVENT_REGISTRY.variable_scope_note(ev_a, ev_b)
                if note is None:
                    # No scoping concern → the cross-event flow is valid
                    cross_defs.add(var)
                    cross_imports.add(var)

    return ConnectionScope(
        summaries=summaries,
        cross_event_defs=frozenset(cross_defs),
        cross_event_imports=frozenset(cross_imports),
    )
