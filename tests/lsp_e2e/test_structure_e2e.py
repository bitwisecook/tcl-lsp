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

"""Folding ranges, selection ranges, and workspace symbols, end-to-end.

Ported from ``tests/test_workspace_symbols.py`` and the VS Code
``foldingRanges.test.ts`` / ``selectionRange.test.ts`` scenarios.
"""

from __future__ import annotations

FUNCTION = 12


class TestFoldingRange:
    def test_proc_body_folds(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc greet {} {\n    set x 1\n    return $x\n}\n")
        ranges = lsp_server.folding_range(uri) or []
        assert any(r["startLine"] == 0 and r["endLine"] >= 2 for r in ranges)

    def test_no_folds_in_flat_file(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 1\nset y 2\n")
        assert (lsp_server.folding_range(uri) or []) == []


class TestLineContinuationFolding:
    """Backslash line-continuation folding — issue #541, end-to-end.

    A command stretched across physical lines by trailing ``\\`` joins is a
    single logical command; the provider folds the run down to its opening
    line.  The look-alikes that are *not* continuations (an escaped ``\\\\`` is
    a literal backslash) must stay unfolded.  The feature was dropped by the
    folding rewrite and restored in #541 — this pins the editor-visible result.
    """

    @staticmethod
    def _spans(lsp_server, uri) -> set[tuple[int, int]]:
        return {(r["startLine"], r["endLine"]) for r in (lsp_server.folding_range(uri) or [])}

    def test_backslash_continued_command_folds(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "MyProcCall $var1 \\\n           $var2 \\\n           $var3\n")
        # The three physical lines are one logical command; they fold (0..2).
        assert (0, 2) in self._spans(lsp_server, uri), self._spans(lsp_server, uri)

    def test_two_line_continuation_folds(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x $a \\\n    $b\n")
        assert (0, 1) in self._spans(lsp_server, uri), self._spans(lsp_server, uri)

    def test_separate_runs_fold_independently(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "foo $a \\\n   $b\nputs sep\nbar $c \\\n   $d\n")
        spans = self._spans(lsp_server, uri)
        assert {(0, 1), (3, 4)} <= spans, spans

    def test_escaped_backslash_is_not_a_continuation(self, lsp_server, uri_factory):
        # ``puts foo\\`` ends in a *literal* backslash (escaped), so the two
        # lines are separate commands and must not fold.
        uri = uri_factory()
        lsp_server.open_ready(uri, "puts foo\\\\\nputs bar\n")
        assert (0, 1) not in self._spans(lsp_server, uri), self._spans(lsp_server, uri)


class TestSelectionRange:
    def test_widens_from_inner_to_outer(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc greet {} {\n    set x 1\n    return $x\n}\n")
        result = lsp_server.selection_range(uri, [(2, 11)])
        assert result
        node = result[0]
        depth = 0
        while node is not None:
            node = node.get("parent")
            depth += 1
        assert depth >= 2


class TestWorkspaceSymbols:
    def test_find_proc(self, lsp_server, uri_factory):
        a = uri_factory()
        b = uri_factory()
        lsp_server.open_ready(a, "proc greet_uniquely_aaa {} { return }\n")
        lsp_server.open_ready(b, "proc farewell_uniquely_bbb {} { return }\n")
        result = lsp_server.workspace_symbols("greet_uniquely_aaa") or []
        matched = [s for s in result if s.get("name") == "greet_uniquely_aaa"]
        assert len(matched) == 1
        assert matched[0]["kind"] == FUNCTION

    def test_empty_query_returns_all(self, lsp_server, uri_factory):
        # An empty query returns every indexed symbol; assert our uniquely-named
        # procs are present (the shared session holds other tests' docs too).
        a = uri_factory()
        b = uri_factory()
        lsp_server.open_ready(a, "proc foo_empty_q_aaa {} { return }\n")
        lsp_server.open_ready(b, "proc bar_empty_q_bbb {} { return }\n")
        names = {s.get("name") for s in (lsp_server.workspace_symbols("") or [])}
        assert {"foo_empty_q_aaa", "bar_empty_q_bbb"} <= names

    def test_partial_match(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc calculate_total_zzz {} { return }\n")
        result = lsp_server.workspace_symbols("calc") or []
        assert any(s.get("name") == "calculate_total_zzz" for s in result)

    def test_no_match(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc greet {} { return }\n")
        result = lsp_server.workspace_symbols("zzz_no_such_symbol_qqq") or []
        assert all(s.get("name") != "zzz_no_such_symbol_qqq" for s in result)
