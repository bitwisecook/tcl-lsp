"""Shared CFG edge-routing model for the compiler explorer.

The web GUI (SVG) and the CLI/TUI (ASCII box-drawing gutter) draw control-flow
edges in different media, but they must agree on *routing* — which edges exist
and which lane each one occupies — so a reader sees the same graph everywhere.
That model lives here, once.  The serialiser bakes the per-edge ``lane`` into
the JSON the web ``drawAllCfgEdges`` reads, and ``cli`` / the TUI render the
same lanes as box-drawing characters.

Lane assignment mirrors the old in-JS algorithm exactly (shortest span first →
innermost lane, greedy interval colouring over block ordinals) so both surfaces
nest edges identically.  Only the *drawing* differs between surfaces; the model
does not.
"""

from __future__ import annotations

from dataclasses import dataclass

from compiler.cfg import CFGBranch, CFGFunction, _block_successors


@dataclass(frozen=True, slots=True)
class CFGEdge:
    src: str  # source block name
    dst: str  # destination block name
    src_pos: int  # ordinal index of the source block (0-based, block order)
    dst_pos: int  # ordinal index of the destination block
    kind: str  # "goto" | "true" | "false"
    lane: int  # routing lane (0 = innermost, nearest the blocks)


def assign_lanes(spans: list[tuple[int, int]]) -> list[int]:
    """Greedy interval colouring: shortest span first → innermost lane.

    *spans* is a list of ``(from_pos, to_pos)`` ordinal pairs.  Returns one lane
    index per span (input order).  Two spans share a lane only if their closed
    ordinal intervals do not overlap (touching endpoints count as overlap, so an
    edge into a block and an edge out of that same block never share a lane).
    Identical to the web GUI's lane loop in ``drawOrthogonalEdges``.
    """
    by_span = sorted(range(len(spans)), key=lambda i: abs(spans[i][1] - spans[i][0]))
    lanes: list[list[tuple[int, int]]] = []
    result = [0] * len(spans)
    for i in by_span:
        a, b = spans[i]
        lo, hi = (a, b) if a <= b else (b, a)
        placed: int | None = None
        for lane, intervals in enumerate(lanes):
            if all(hi < x or lo > y for (x, y) in intervals):
                intervals.append((lo, hi))
                placed = lane
                break
        if placed is None:
            lanes.append([(lo, hi)])
            placed = len(lanes) - 1
        result[i] = placed
    return result


def build_cfg_edges(cfg: CFGFunction) -> list[CFGEdge]:
    """Extract control-flow edges from *cfg* with routing lanes assigned.

    Edges follow ``CFGBlock`` successors only (the same set the CFG view draws);
    ``exception_edges`` are analysis-only and not shown here.
    """
    order = list(cfg.blocks)
    pos = {name: i for i, name in enumerate(order)}
    raw: list[tuple[str, str, int, int, str]] = []
    for name in order:
        term = cfg.blocks[name].terminator
        for target in _block_successors(term):
            if target not in pos:
                continue
            kind = "goto"
            if isinstance(term, CFGBranch):
                kind = "true" if target == term.true_target else "false"
            raw.append((name, target, pos[name], pos[target], kind))
    lanes = assign_lanes([(sp, dp) for (_, _, sp, dp, _) in raw])
    return [
        CFGEdge(src=s, dst=d, src_pos=sp, dst_pos=dp, kind=k, lane=lane)
        for (s, d, sp, dp, k), lane in zip(raw, lanes)
    ]
