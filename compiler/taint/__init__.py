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

"""Taint analysis for untrusted I/O data.

Tracks data provenance through the SSA graph using a colour-aware
lattice.  See individual submodules for detailed documentation.
"""

from __future__ import annotations

from ._api import find_taint_warnings
from ._lattice import MethodTaintSummary, ProcTaintSummary, TaintLattice, taint_join
from ._propagation import taint_propagation
from ._types import (
    TaintWarning,
)

__all__ = [
    "MethodTaintSummary",
    "ProcTaintSummary",
    "TaintLattice",
    "TaintWarning",
    "find_taint_warnings",
    "taint_join",
    "taint_propagation",
]
