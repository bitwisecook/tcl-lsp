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

"""Document highlight, end-to-end against the packaged server.

Ported from ``tests/test_document_highlight.py``.  Highlight kinds come back
as raw LSP integers: 1=Text, 2=Read, 3=Write.
"""

from __future__ import annotations

import textwrap

TEXT, READ, WRITE = 1, 2, 3


def _highlights(lsp_server, uri, line, char):
    return lsp_server.document_highlight(uri, line, char) or []


class TestDocumentHighlightProc:
    def test_highlights_proc_definition_and_calls(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc greet {name} { puts "Hello $name" }
            greet World
            greet Everyone
        """)
        lsp_server.open_ready(uri, src)
        hls = _highlights(lsp_server, uri, 0, 6)
        assert len(hls) >= 2
        assert all(h.get("kind", TEXT) == TEXT for h in hls)

    def test_highlights_include_declaration(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc greet {} { return }\ngreet\ngreet\n")
        lines = {h["range"]["start"]["line"] for h in _highlights(lsp_server, uri, 0, 6)}
        assert {0, 1, 2} <= lines

    def test_cursor_on_whitespace_returns_empty(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc greet {} { return }\n")
        assert _highlights(lsp_server, uri, 0, 0) == []

    def test_no_duplicate_ranges(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc greet {} { return }\ngreet\n")
        hls = _highlights(lsp_server, uri, 0, 6)
        keys = [
            (
                h["range"]["start"]["line"],
                h["range"]["start"]["character"],
                h["range"]["end"]["line"],
                h["range"]["end"]["character"],
            )
            for h in hls
        ]
        assert len(keys) == len(set(keys))


class TestDocumentHighlightVariable:
    def test_highlights_variable_uses(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc sum {} {
                set x 1
                set y [expr {$x + 2}]
                return $x
            }
        """)
        lsp_server.open_ready(uri, src)
        hls = _highlights(lsp_server, uri, 3, 12)
        assert len(hls) >= 1
        assert all(h["range"]["start"]["line"] >= 1 for h in hls)

    def test_write_kind_on_definition_read_kind_on_use(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc p {} {
                set x 1
                return $x
            }
        """)
        lsp_server.open_ready(uri, src)
        kinds = {h.get("kind") for h in _highlights(lsp_server, uri, 2, 12)}
        assert WRITE in kinds
        assert READ in kinds

    def test_scope_isolation_between_global_and_local(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            set x 1
            proc uses_x {} {
                set x 2
                puts $x
            }
            puts $x
        """)
        lsp_server.open_ready(uri, src)
        lines = sorted({h["range"]["start"]["line"] for h in _highlights(lsp_server, uri, 3, 10)})
        assert 0 not in lines
        assert 5 not in lines
        assert 2 in lines and 3 in lines
