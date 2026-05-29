"""Shared natural-loop / loop-nesting analysis over the CFG + SSA dominators.

Back-edge and natural-loop detection was historically re-derived independently
in several passes (GVN/LICM, shimmer, the bytecode emitter).  This module is the
single source of truth: given a :class:`CFGFunction` and its
:class:`SSAFunction` (which already carries Cooper-Harvey-Kennedy ``idom``), it
finds back edges (``tail -> header`` where *header* dominates *tail*), the
natural-loop body for each, and groups them by header into a :class:`LoopForest`.

Consumers that previously rolled their own detection now build one
:class:`LoopForest` and read from it, so the same (correct) loops are seen
everywhere and the dominator walk is not repeated.
"""

from __future__ import annotations

from dataclasses import dataclass

from .cfg import CFGBranch, CFGFunction, CFGGoto
from .ssa import BlockName, SSAFunction


def dominates(ssa: SSAFunction, dominator: BlockName, node: BlockName) -> bool:
    """Return True if *dominator* dominates *node* in the SSA dominator tree."""
    current: BlockName | None = node
    while current is not None:
        if current == dominator:
            return True
        current = ssa.idom.get(current)
    return False


def cfg_successors(cfg: CFGFunction, block_name: BlockName) -> tuple[BlockName, ...]:
    """Return the CFG successors of *block_name* (``()`` for return/none)."""
    block = cfg.blocks[block_name]
    term = block.terminator
    match term:
        case None:
            return ()
        case CFGGoto(target=target):
            return (target,)
        case CFGBranch(true_target=true_target, false_target=false_target):
            return (true_target, false_target)
        case _:
            return ()


def cfg_predecessors(
    cfg: CFGFunction, executable: set[BlockName]
) -> dict[BlockName, set[BlockName]]:
    """Build the predecessor map restricted to *executable* blocks."""
    preds: dict[BlockName, set[BlockName]] = {bn: set() for bn in executable}
    for bn in executable:
        for succ in cfg_successors(cfg, bn):
            if succ in preds:
                preds[succ].add(bn)
    return preds


def natural_loop_blocks(
    header: BlockName,
    latch: BlockName,
    preds: dict[BlockName, set[BlockName]],
    executable: set[BlockName],
) -> set[BlockName]:
    """Return blocks in the natural loop for one back-edge latch -> header."""
    blocks: set[BlockName] = {header, latch}
    work: list[BlockName] = [latch]

    while work:
        node = work.pop()
        for pred in preds.get(node, set()):
            if pred not in executable or pred in blocks:
                continue
            blocks.add(pred)
            if pred != header:
                work.append(pred)

    return blocks


def loop_defined_variables(
    ssa: SSAFunction,
    loop_blocks: set[BlockName] | frozenset[BlockName],
) -> frozenset[str]:
    """Return variable names defined anywhere inside a loop."""
    defs: set[str] = set()
    for bn in loop_blocks:
        ssa_block = ssa.blocks.get(bn)
        if ssa_block is None:
            continue
        for phi in ssa_block.phis:
            defs.add(phi.name)
        for stmt in ssa_block.statements:
            defs |= set(stmt.defs)
    return frozenset(defs)


@dataclass(frozen=True, slots=True)
class NaturalLoop:
    """One natural loop, identified by its header.

    ``blocks`` is the union of the natural-loop bodies of every back edge that
    targets ``header``; ``latches`` are the tail blocks of those back edges.
    """

    header: BlockName
    blocks: frozenset[BlockName]
    latches: frozenset[BlockName]


@dataclass(frozen=True, slots=True)
class LoopForest:
    """All natural loops of one function, grouped by header.

    ``by_header`` preserves first-encounter order so consumers that iterate
    loops produce deterministic, stable output.
    """

    by_header: dict[BlockName, NaturalLoop]

    @property
    def loops(self) -> tuple[NaturalLoop, ...]:
        return tuple(self.by_header.values())

    @property
    def headers(self) -> frozenset[BlockName]:
        return frozenset(self.by_header)

    def is_empty(self) -> bool:
        return not self.by_header


def build_loop_forest(
    cfg: CFGFunction,
    ssa: SSAFunction,
    executable: set[BlockName] | None = None,
) -> LoopForest:
    """Build the :class:`LoopForest` for *cfg* / *ssa*.

    *executable* defaults to all CFG blocks; callers with reachability
    information (e.g. SCCP) pass ``set(cfg.blocks) - unreachable_blocks`` so
    loops formed only by unreachable blocks are ignored.

    A back edge is ``tail -> succ`` where *succ* dominates *tail*; the natural
    loop bodies of all back edges sharing a header are unioned.
    """
    if executable is None:
        executable = set(cfg.blocks)
    preds = cfg_predecessors(cfg, executable)

    blocks_by_header: dict[BlockName, set[BlockName]] = {}
    latches_by_header: dict[BlockName, set[BlockName]] = {}

    for tail in executable:
        for succ in cfg_successors(cfg, tail):
            if succ not in executable:
                continue
            if not dominates(ssa, succ, tail):
                continue
            body = natural_loop_blocks(succ, tail, preds, executable)
            if succ in blocks_by_header:
                blocks_by_header[succ].update(body)
            else:
                blocks_by_header[succ] = set(body)
            latches_by_header.setdefault(succ, set()).add(tail)

    by_header = {
        header: NaturalLoop(
            header=header,
            blocks=frozenset(body),
            latches=frozenset(latches_by_header[header]),
        )
        for header, body in blocks_by_header.items()
    }
    return LoopForest(by_header=by_header)
