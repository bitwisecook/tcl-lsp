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

"""Warning dataclasses for taint analysis diagnostics."""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

from shared.diagnostic import CodeFix, Range

if TYPE_CHECKING:
    from ..ssa import SSAValueKey
    from ._lattice import ProcTaintSummary, TaintLattice


@dataclass(frozen=True, slots=True)
class TaintWarning:
    """Tainted data flowing into a dangerous sink."""

    range: Range
    variable: str
    sink_command: str
    code: str  # T100
    message: str
    fixes: tuple[CodeFix, ...] = ()


@dataclass(frozen=True, slots=True)
class _InterprocTaintResult:
    """Result of inter-procedural taint analysis."""

    top_taints: dict[SSAValueKey, TaintLattice]
    proc_taints: dict[str, dict[SSAValueKey, TaintLattice]]
    summaries: dict[str, ProcTaintSummary]
