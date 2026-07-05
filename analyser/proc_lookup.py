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

"""Shared proc lookup helpers for symbol-oriented LSP features."""

from __future__ import annotations

from .semantic_model import AnalysisResult, ProcDef


def iter_procs_by_reference(
    analysis: AnalysisResult,
    ref: str,
) -> list[tuple[str, ProcDef]]:
    """Return all procs whose names match *ref* in supported reference forms."""
    matches: list[tuple[str, ProcDef]] = []
    for qname, proc_def in analysis.all_procs.items():
        if proc_def.name == ref or qname == ref or qname == f"::{ref}":
            matches.append((qname, proc_def))
    return matches


def find_proc_by_reference(
    analysis: AnalysisResult,
    ref: str,
) -> tuple[str, ProcDef] | None:
    """Return the first proc matching *ref* in supported reference forms."""
    matches = iter_procs_by_reference(analysis, ref)
    if not matches:
        return None
    return matches[0]
