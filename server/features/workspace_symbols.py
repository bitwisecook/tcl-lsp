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

"""Workspace symbol provider -- Cmd+T to search procs across all files."""

from __future__ import annotations

from lsprotocol import types

from server._lsp_conv import to_lsp_range


def get_workspace_symbols(
    query: str,
    workspace_index,
) -> list[types.WorkspaceSymbol]:
    """Return workspace symbols matching a query string."""
    symbols: list[types.WorkspaceSymbol] = []
    query_lower = query.lower()

    for qname in workspace_index.all_proc_names():
        # Match on qualified name or tail name
        tail = qname.rsplit("::", 1)[-1]
        if query_lower and query_lower not in qname.lower() and query_lower not in tail.lower():
            continue

        entries = workspace_index.find_proc(qname)
        for entry in entries:
            if entry.proc is None:
                continue
            proc_def = entry.proc
            symbols.append(
                types.WorkspaceSymbol(
                    name=proc_def.name,
                    kind=types.SymbolKind.Function,
                    location=types.Location(
                        uri=entry.uri,
                        range=to_lsp_range(proc_def.name_range),
                    ),
                    container_name=qname if qname != f"::{proc_def.name}" else None,
                )
            )

    return symbols
