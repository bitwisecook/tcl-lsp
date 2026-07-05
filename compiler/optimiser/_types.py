# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Core types for the optimiser package."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import TYPE_CHECKING

from compiler.interprocedural import InterproceduralAnalysis
from shared.diagnostic import Range

from ..ir import IRModule

if TYPE_CHECKING:
    from ..cfg import CFGFunction
    from ..command_binding import CommandBinding

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
    "O128": 5,
    "O104": 4,
    "O103": 3,
    "O102": 2,
    "O101": 1,
    "O127": 0,
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
    # When not None the chain is in *list* mode: it began as ``set var <list>``
    # and was extended by ``lappend``, so the accumulated state is a list of
    # elements rendered with list quoting rather than string concatenation.
    elements: list[str] | None = None


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
    # Per-use-site record of which ``(name, version)`` reads were folded away at
    # a given ``(block, statement_index)``.  A statement that folds *one*
    # variable's ``$ref`` (e.g. ``puts "$a $b"`` where only ``$b`` is a same-block
    # constant) must still count as a live consumer of the *other* variables it
    # references — ``propagated_expr_stmts`` is too coarse (per-statement) for
    # that, so DCE consults this finer-grained set instead.
    propagated_use_sites: set[tuple[str, int, str, int]] = field(default_factory=set)
    # Flow-sensitive command-binding lattice for the function currently being
    # processed — set only for the **top-level** script (where definition / rename
    # order is fully known); ``None`` inside proc/method bodies, whose per-function
    # lattice can't see the enclosing module's proc definitions.  Proc-call folding
    # consults it (with ``cur_block``/``cur_idx``) so a ``[a]`` after ``rename a b``
    # is not folded as the now-renamed-away proc.
    command_binding: CommandBinding | None = None
    cur_block: str = ""
    cur_idx: int = -1

    def alloc_group(self) -> int:
        """Allocate and return the next group ID."""
        gid = self.next_group
        self.next_group += 1
        return gid
