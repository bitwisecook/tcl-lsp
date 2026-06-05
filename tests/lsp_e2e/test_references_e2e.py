"""Find-references, end-to-end against the packaged server.

Ported from ``tests/test_references.py``.
"""

from __future__ import annotations

import textwrap

from ._lsp_helpers import start_lines, starts


class TestProcReferences:
    def test_find_proc_definition_and_calls(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc greet {name} { puts "Hello $name" }
            greet World
            greet Everyone
        """)
        lsp_server.open_ready(uri, src)
        assert len(starts(lsp_server.references(uri, 0, 6))) >= 2

    def test_exclude_declaration(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = 'proc greet {name} { puts "Hello $name" }\ngreet World\n'
        lsp_server.open_ready(uri, src)
        with_decl = starts(lsp_server.references(uri, 0, 6, include_declaration=True))
        without = starts(lsp_server.references(uri, 0, 6, include_declaration=False))
        assert len(with_decl) >= len(without)

    def test_find_qualified_proc_call_sites(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            namespace eval myns {
                proc helper {} { return 1 }
            }
            myns::helper
            ::myns::helper
        """)
        lsp_server.open_ready(uri, src)
        lines = start_lines(lsp_server.references(uri, 1, 10))
        assert 3 in lines
        assert 4 in lines


class TestVariableReferences:
    def test_find_var_refs(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 42\nputs $x\n")
        assert starts(lsp_server.references(uri, 1, 7)) == {(0, 4), (1, 5)}

    def test_multiple_var_refs(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 1\nset x 2\nputs $x\n")
        assert starts(lsp_server.references(uri, 2, 6)) == {(0, 4), (1, 4), (2, 5)}

    def test_no_refs_for_unknown(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "puts hello\n")
        assert starts(lsp_server.references(uri, 0, 6)) == set()

    def test_var_refs_respect_shadowing_global_target(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            set x 1
            puts $x
            proc demo {} {
                set x 2
                puts $x
            }
            demo
        """)
        lsp_server.open_ready(uri, src)
        assert start_lines(lsp_server.references(uri, 1, 6)) == {0, 1}

    def test_var_refs_respect_shadowing_local_target(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            set x 1
            puts $x
            proc demo {} {
                set x 2
                puts $x
            }
            demo
        """)
        lsp_server.open_ready(uri, src)
        assert start_lines(lsp_server.references(uri, 4, 10)) == {3, 4}
