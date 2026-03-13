"""Core types for the optimiser package."""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

from ...analysis.semantic_model import Range
from ..interprocedural import InterproceduralAnalysis
from ..ir import IRModule

if TYPE_CHECKING:
    from ..cfg import CFGFunction

_OPT_PRIORITY = {
    "O126": 10,
    "O124": 10,
    "O112": 9,
    "O109": 8,
    "O108": 7,
    "O107": 6,
    "O122": 6,
    "O121": 5,
    "O123": 5,
    "O125": 5,
    "O119": 5,
    "O120": 5,
    "O118": 5,
    "O117": 5,
    "O116": 5,
    "O115": 5,
    "O114": 5,
    "O113": 5,
    "O110": 5,
    "O104": 4,
    "O103": 3,
    "O102": 2,
    "O101": 1,
    "O100": 0,
}


@dataclass(frozen=True, slots=True)
class Optimisation:
    """A suggested source rewrite."""

    code: str
    message: str
    range: Range
    replacement: str = ""
    group: int | None = None
    hint_only: bool = False


@dataclass(slots=True)
class _StringWriteChain:
    var_word: str
    writes: list[int]
    value: str


@dataclass
class PassContext:
    """Shared mutable state threaded through all optimisation passes."""

    source: str
    optimisations: list[Optimisation]
    interproc: InterproceduralAnalysis
    proc_cfgs: dict[str, tuple[CFGFunction, tuple[str, ...]]]
    propagated_branch_uses: set[tuple[str, int]]
    cross_event_vars: frozenset[str]
    next_group: int
    propagated_use_groups: dict[tuple[str, int], int]
    propagated_expr_stmts: set[tuple[str, int]]
    ir_module: IRModule | None

    def alloc_group(self) -> int:
        """Allocate and return the next group ID."""
        gid = self.next_group
        self.next_group += 1
        return gid
