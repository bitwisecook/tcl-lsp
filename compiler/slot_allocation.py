"""Liveness-based interference colouring over proc locals (ANALYSIS ONLY).

Computes, from SSA liveness, the classic **interference-graph colouring**:
variables interfere iff both are live at the same program point; a greedy
colouring assigns the fewest "slots" such that no two interfering variables
collide.

IMPORTANT — this is **not** wired into any emitter, and must not be:

* The **bytecode** path's LVT slot is a *name-table index* — the VM resolves
  ``lvt.entries()[slot]`` to the variable **name** and accesses the frame by
  name (``machine.py``).  Coalescing two variables onto one slot would alias
  them to the same name.  More fundamentally, Tcl compiled locals are
  name-addressable for the proc's whole lifetime (``upvar``/``info locals``/
  ``trace``/``uplevel``), so dataflow-"dead" ≠ "inaccessible" — liveness-based
  slot reuse is **semantically invalid** for Tcl locals (and tclsh never reuses
  LVT slots).  An earlier opt-in wiring was reverted after the differential
  fuzzer caught the aliasing corruption (seed_794).  See the Phase 5 ledger.
* The only sound home for slot reuse is the **WASM** emitter when
  ``_proc_wants_frame()`` is False (genuine i32 storage, frame elided so no
  name-addressing) — a documented, low-value follow-up.

The colouring is retained as an **analysis** — e.g. register-pressure
reporting or a compiler-explorer overlay — where the *result* is informational
and never rewrites slots.  Parameters are pinned to their incoming order
(slots ``0..n-1``); being live on entry they mutually interfere and never share.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from .cfg import CFGBranch, CFGFunction, CFGReturn
from .expr_ast import vars_in_expr_node
from .ssa import SSAFunction, _vars_in_word

if TYPE_CHECKING:
    from .ssa import SSAValueKey


def _add_clique(graph: dict[str, set[str]], live: set[str]) -> None:
    names = sorted(live)
    for n in names:
        graph.setdefault(n, set())
    for i, a in enumerate(names):
        for b in names[i + 1 :]:
            graph[a].add(b)
            graph[b].add(a)


def _terminator_read_names(cfg: CFGFunction, block: str) -> set[str]:
    """Names read by a block's terminator (branch condition / return value).

    These reads occur after every statement in the block, so they must be in
    the live set when the backward walk starts — otherwise a value used *only*
    in a ``return``/``if`` (e.g. a parameter passed straight to ``return [expr
    {$x + $y}]``) would never appear live and would wrongly coalesce."""
    term = cfg.blocks[block].terminator
    if isinstance(term, CFGBranch):
        return set(vars_in_expr_node(term.condition))
    if isinstance(term, CFGReturn):
        names: set[str] = set()
        if term.value is not None:
            names |= _vars_in_word(term.value)
        if term.expr is not None:
            names |= set(vars_in_expr_node(term.expr))
        return names
    return set()


def build_interference(
    cfg: CFGFunction,
    ssa: SSAFunction,
    live_out: dict[str, set[SSAValueKey]],
) -> dict[str, set[str]]:
    """Undirected interference graph over variable *names*, instruction-granular.

    *live_out* is the per-block SSA liveness (``core_analyses._liveness``'s
    second result).  Slot coalescing depends on **liveness only** — never the
    full ``analyse_function`` (SCCP/interval/type), keeping it cheap and free of
    those passes' shared-cache side effects so it is safe to run inside the
    bytecode emitter.

    For each block we seed the live set from its ``live_out`` plus the
    terminator's own reads, then walk statements **backwards**, recording the
    set of simultaneously-live names at every point as an interference clique
    and updating ``live = (live - defs) ∪ uses``.  Phi targets defined at the
    block head are folded in too.  Instruction granularity is what lets two
    straight-line locals with disjoint ranges (``set a …; use a; set b …; use
    b``) share a slot — block-granular liveness would miss that entirely."""
    graph: dict[str, set[str]] = {}

    def names_of(keys: set[tuple[str, int]]) -> set[str]:
        return {name for name, _ in keys}

    for bn, sblock in ssa.blocks.items():
        live = names_of(live_out.get(bn, set())) | _terminator_read_names(cfg, bn)
        _add_clique(graph, live)
        for s in reversed(sblock.statements):
            defs = {n for n in s.defs}
            uses = {n for n in s.uses}
            live = (live - defs) | uses
            _add_clique(graph, live)
        # Phi targets are defined at the block head; they are live across the
        # incoming edges and must not collide with names live there.
        for phi in sblock.phis:
            graph.setdefault(phi.name, set())
            live = live - {phi.name}
            _add_clique(graph, live | {phi.name})
    return graph


def coalesce_slots(
    cfg: CFGFunction,
    ssa: SSAFunction,
    live_out: dict[str, set[SSAValueKey]],
    *,
    params: tuple[str, ...] = (),
) -> dict[str, int]:
    """Assign each variable name a slot index, reusing slots between names whose
    live ranges do not interfere (greedy interference-graph colouring).

    *params* are coloured first, in order, so they occupy slots ``0..n-1``
    (an emitter can keep incoming-argument slots stable); since parameters are
    live on entry they mutually interfere and thus never share a slot.  The
    result maps every name in the interference graph (plus any param) to a slot;
    ``max(result.values()) + 1`` is the coalesced slot count.
    """
    graph = build_interference(cfg, ssa, live_out)
    for p in params:
        graph.setdefault(p, set())

    colour: dict[str, int] = {}

    def assign(name: str) -> None:
        if name in colour:
            return
        used = {colour[nb] for nb in graph.get(name, ()) if nb in colour}
        slot = 0
        while slot in used:
            slot += 1
        colour[name] = slot

    # Parameters first (stable low slots), then the rest by descending degree
    # (colour the most-constrained nodes first — the usual greedy heuristic).
    for p in params:
        assign(p)
    for name in sorted(graph, key=lambda n: (-len(graph[n]), n)):
        assign(name)
    return colour


def slot_count(mapping: dict[str, int]) -> int:
    """Number of distinct slots a coalescing uses (0 for an empty mapping)."""
    return (max(mapping.values()) + 1) if mapping else 0


__all__ = ["build_interference", "coalesce_slots", "slot_count"]
