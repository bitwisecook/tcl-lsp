"""Tests for the workspace symbol provider."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from core.analysis.analyser import analyse
from lsp.features.workspace_symbols import get_workspace_symbols
from lsp.workspace.workspace_index import WorkspaceIndex


def _index_with(sources: dict[str, str]) -> WorkspaceIndex:
    idx = WorkspaceIndex()
    for uri, src in sources.items():
        result = analyse(src)
        idx.update(uri, result)
    return idx


class TestWorkspaceSymbols:
    def test_find_proc(self):
        idx = _index_with(
            {
                "file:///a.tcl": "proc greet {} { return }",
                "file:///b.tcl": "proc farewell {} { return }",
            }
        )
        symbols = get_workspace_symbols("greet", idx)
        assert len(symbols) == 1
        assert symbols[0].name == "greet"

    def test_empty_query_returns_all(self):
        idx = _index_with(
            {
                "file:///a.tcl": "proc foo {} { return }",
                "file:///b.tcl": "proc bar {} { return }",
            }
        )
        symbols = get_workspace_symbols("", idx)
        names = {s.name for s in symbols}
        assert "foo" in names
        assert "bar" in names

    def test_partial_match(self):
        idx = _index_with(
            {
                "file:///a.tcl": "proc calculate_total {} { return }",
            }
        )
        symbols = get_workspace_symbols("calc", idx)
        assert len(symbols) == 1
        assert symbols[0].name == "calculate_total"

    def test_no_match(self):
        idx = _index_with(
            {
                "file:///a.tcl": "proc greet {} { return }",
            }
        )
        symbols = get_workspace_symbols("zzz_no_match", idx)
        assert len(symbols) == 0

    def test_symbol_kind_is_function(self):
        idx = _index_with(
            {
                "file:///a.tcl": "proc greet {} { return }",
            }
        )
        symbols = get_workspace_symbols("greet", idx)
        assert symbols[0].kind == types.SymbolKind.Function
