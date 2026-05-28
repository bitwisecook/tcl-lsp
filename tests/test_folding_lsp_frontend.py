"""Front-end tests for folding: drive the real ``textDocument/foldingRange``
handler through ``workspace_state`` and the feature-config gate, the same entry
point the JSON-RPC dispatch calls.

Complements ``tests/test_folding.py`` (which unit-tests ``get_folding_ranges``)
by exercising the server handler wiring: source resolution from the document
store, the ``folding_enabled`` toggle, and that the handler returns
``lsprotocol`` ``FoldingRange`` objects with the expected kinds.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

import server.server as server_module
import server.state as _state
from server.feature_config import FeatureConfig
from server.state import workspace_state


def _fold(uri: str) -> list[types.FoldingRange]:
    params = types.FoldingRangeParams(
        text_document=types.TextDocumentIdentifier(uri=uri),
    )
    return server_module.on_folding_range(params)


def _spans(ranges: list[types.FoldingRange]) -> set[tuple[int, int, object]]:
    return {(r.start_line, r.end_line, r.kind) for r in ranges}


class TestFoldingHandler:
    def test_handler_returns_mixed_kinds(self):
        uri = "file:///folding-frontend.tcl"
        source = (
            "#region Setup\n"  # 0
            "package require Tcl\n"  # 1
            "package require http\n"  # 2
            "#endregion\n"  # 3
            "\n"  # 4
            "# a comment\n"  # 5
            "# block here\n"  # 6
            "proc greet {name} {\n"  # 7
            '    puts "Hello $name"\n'  # 8
            '    puts "Bye $name"\n'  # 9
            "}\n"  # 10
        )
        workspace_state.open(uri, source)
        try:
            spans = _spans(_fold(uri))
        finally:
            workspace_state.close(uri)

        Region = types.FoldingRangeKind.Region
        Comment = types.FoldingRangeKind.Comment
        Imports = types.FoldingRangeKind.Imports

        assert (0, 3, Region) in spans, spans  # #region/#endregion
        assert (1, 2, Imports) in spans, spans  # package require run
        assert (5, 6, Comment) in spans, spans  # comment block
        assert (7, 9, Region) in spans, spans  # proc body

    def test_handler_well_formed(self):
        """Every returned range is whole-line and non-degenerate."""
        uri = "file:///folding-wellformed.tcl"
        source = "proc demo {} {\n    if {1} {\n        puts hi\n    }\n}\n"
        workspace_state.open(uri, source)
        try:
            ranges = _fold(uri)
        finally:
            workspace_state.close(uri)
        assert ranges
        for r in ranges:
            assert r.end_line > r.start_line, r
            assert r.start_line >= 0

    def test_disabled_via_config_returns_empty(self, monkeypatch):
        uri = "file:///folding-disabled.tcl"
        source = "proc foo {} {\n    puts hi\n    puts bye\n}\n"
        workspace_state.open(uri, source)
        monkeypatch.setattr(
            _state,
            "config_for_uri",
            lambda _uri: FeatureConfig(folding_enabled=False),
        )
        try:
            assert _fold(uri) == []
        finally:
            workspace_state.close(uri)
