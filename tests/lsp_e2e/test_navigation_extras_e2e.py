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

"""Call hierarchy, implementation, and linked editing — end-to-end.

Extends the cross-implementation conformance surface (these drive the packaged
server over raw JSON-RPC, so they double as a spec the Rust/Zed port must meet).
Ported from ``tests/test_call_hierarchy.py``, ``tests/test_implementation.py``,
and ``tests/test_linked_editing_range.py``.
"""

from __future__ import annotations

import textwrap

from ._lsp_helpers import start_lines


class TestCallHierarchy:
    def test_prepare_returns_proc_item(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc helper {} { return 1 }\nhelper\n")
        items = lsp_server.prepare_call_hierarchy(uri, 0, 6)
        assert items
        assert items[0]["name"] == "helper"
        assert items[0]["kind"] == 12  # Function

    def test_incoming_calls(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "proc helper {} { return 1 }\nproc caller {} { helper }\ncaller\n"
        lsp_server.open_ready(uri, src)
        item = lsp_server.prepare_call_hierarchy(uri, 0, 6)[0]
        incoming = lsp_server.incoming_calls(item) or []
        callers = {c["from"]["name"] for c in incoming}
        assert "caller" in callers

    def test_outgoing_calls(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "proc helper {} { return 1 }\nproc caller {} { helper }\ncaller\n"
        lsp_server.open_ready(uri, src)
        item = lsp_server.prepare_call_hierarchy(uri, 1, 6)[0]
        outgoing = lsp_server.outgoing_calls(item) or []
        callees = {c["to"]["name"] for c in outgoing}
        assert "helper" in callees


class TestImplementation:
    def test_method_implementations_across_subclasses(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            oo::class create Animal {
                method speak {} {}
            }
            oo::class create Dog {
                superclass Animal
                method speak {} {}
            }
        """)
        lsp_server.open_ready(uri, src)
        # Cursor on Animal's ``speak`` — implementations span both classes.
        lines = start_lines(lsp_server.implementation(uri, 1, 11))
        assert 1 in lines
        assert 5 in lines


class TestLinkedEditingRange:
    def test_recursive_self_calls_linked_with_declaration(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc fib {n} {
                if {$n < 2} { return $n }
                return [expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}]
            }
        """)
        lsp_server.open_ready(uri, src)
        result = lsp_server.linked_editing_range(uri, 0, 6)
        assert result is not None
        ranges = result["ranges"]
        assert len(ranges) >= 2
        assert any(r["start"]["line"] == 0 for r in ranges)
        assert any(r["start"]["line"] == 2 for r in ranges)
        assert "[A-Za-z" in (result.get("wordPattern") or "")

    def test_non_recursive_proc_returns_none(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc greet {} { return hi }\ngreet\n")
        assert not lsp_server.linked_editing_range(uri, 0, 6)
