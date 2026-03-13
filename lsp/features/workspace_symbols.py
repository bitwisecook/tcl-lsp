"""Workspace symbol provider -- Cmd+T to search procs across all files."""

from __future__ import annotations

from lsprotocol import types

from core.common.lsp import to_lsp_range


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
