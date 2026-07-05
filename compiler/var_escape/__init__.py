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

"""Per-proc static analysis: which Tcl variables must live in the runtime frame.

A variable is tagged ``FRAME`` if the analysis cannot prove that it is only
read or written through statically resolved positions in the proc body.
Otherwise it stays ``LOCAL`` and the WASM codegen keeps it in a WASM local
slot.

The public entry point is :func:`analyse_var_escape`, which consumes a
:class:`~compiler.compilation_unit.CompilationUnit` and produces a map
from qualified proc name to :class:`ProcEscapeSummary`.
"""

from __future__ import annotations

from ._api import TOP_LEVEL_QNAME, analyse_var_escape
from ._interprocedural import solve_interprocedural_escape
from ._types import (
    Barrier,
    BarrierKind,
    EscapeReason,
    EscapeReasonKind,
    EscapeTag,
    ProcEscapeSummary,
)

__all__ = [
    "Barrier",
    "BarrierKind",
    "EscapeReason",
    "EscapeReasonKind",
    "EscapeTag",
    "ProcEscapeSummary",
    "TOP_LEVEL_QNAME",
    "analyse_var_escape",
    "solve_interprocedural_escape",
]
