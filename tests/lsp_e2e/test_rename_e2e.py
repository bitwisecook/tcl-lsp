"""Rename, end-to-end against the packaged server.

Ported from ``tests/test_rename.py``.  An invalid or unsafe rename comes back
as a ``null`` WorkspaceEdit on the live wire (the ``on_rename`` handler returns
``None``), so the safety cases assert the result carries no edits.
"""

from __future__ import annotations

import textwrap

from ._lsp_helpers import rename_edits


def _texts(lsp_server, uri, line, char, new_name):
    edits = rename_edits(lsp_server.rename(uri, line, char, new_name))
    return {e["newText"] for e in edits.get(uri, [])}


class TestRenameProc:
    def test_rename_definition_and_calls(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc greet {name} { puts "Hello $name" }
            greet World
            greet Everyone
        """)
        lsp_server.open_ready(uri, src)
        edits = rename_edits(lsp_server.rename(uri, 0, 6, "welcome"))[uri]
        assert len(edits) >= 3
        assert all(e["newText"] == "welcome" for e in edits)

    def test_rename_namespaced_proc_preserves_qualifier(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            namespace eval utils {
                proc helper {x} { return $x }
            }
            utils::helper 42
        """)
        lsp_server.open_ready(uri, src)
        texts = _texts(lsp_server, uri, 1, 10, "assist")
        assert "assist" in texts
        assert "utils::assist" in texts

    def test_rename_from_call_site(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, 'proc greet {name} { puts "Hello $name" }\ngreet World\n')
        edits = rename_edits(lsp_server.rename(uri, 1, 0, "welcome"))[uri]
        assert len(edits) >= 2


class TestRenameVariable:
    def test_rename_var(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 42\nputs $x\n")
        texts = _texts(lsp_server, uri, 1, 7, "newvar")
        assert "newvar" in texts
        assert "$newvar" in texts

    def test_rename_var_preserves_braced_form(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 1\nputs ${x}\n")
        texts = _texts(lsp_server, uri, 0, 4, "newvar")
        assert "newvar" in texts
        assert "${newvar}" in texts

    def test_rename_preserves_array_index(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            set arr 1
            puts $arr
            puts $arr(a)
            puts $arr($i)
            puts ${arr(b)}
        """)
        lsp_server.open_ready(uri, src)
        texts = _texts(lsp_server, uri, 0, 4, "new")
        assert {"new", "$new", "$new(a)", "$new($i)", "${new(b)}"} <= texts

    def test_rename_respects_scope(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            set x 1
            proc demo {} {
                set x 2
                puts $x
            }
        """)
        lsp_server.open_ready(uri, src)
        edits = rename_edits(lsp_server.rename(uri, 3, 10, "y"))[uri]
        lines = {e["range"]["start"]["line"] for e in edits}
        assert 0 not in lines


class TestRenameSafety:
    def test_rejects_invalid_new_symbol_name(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, 'proc greet {name} { puts "Hello $name" }\ngreet World\n')
        assert rename_edits(lsp_server.rename(uri, 0, 6, "bad-name")) == {}

    def test_rejects_proc_collision(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc greet {name} { puts "Hello $name" }
            proc hello {name} { puts "Hi $name" }
            greet World
        """)
        lsp_server.open_ready(uri, src)
        assert rename_edits(lsp_server.rename(uri, 0, 6, "hello")) == {}

    def test_rejects_proc_rename_to_builtin(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, 'proc greet {name} { puts "Hello $name" }\ngreet World\n')
        assert rename_edits(lsp_server.rename(uri, 0, 6, "puts")) == {}
