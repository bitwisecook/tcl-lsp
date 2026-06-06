"""Type-definition and declaration navigation, end-to-end.

Ported from ``tests/test_type_definition.py`` and ``tests/test_declaration.py``.
"""

from __future__ import annotations

import textwrap

from ._lsp_helpers import locations, start_lines


class TestTypeDefinition:
    def test_set_with_class_new(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            oo::class create Point {
                variable x y
                constructor {a b} { set x $a; set y $b }
            }
            set p [Point new 1 2]
            puts $p
        """)
        lsp_server.open_ready(uri, src)
        locs = locations(lsp_server.type_definition(uri, 5, 6))
        assert len(locs) == 1
        assert locs[0]["range"]["start"]["line"] == 0

    def test_plain_set_returns_empty(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 42\nputs $x\n")
        assert locations(lsp_server.type_definition(uri, 1, 6)) == []

    def test_my_call_returns_enclosing_class(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            oo::class create Animal {
                method speak {} { return "..." }
                method greet {} { my speak }
            }
        """)
        lsp_server.open_ready(uri, src)
        locs = locations(lsp_server.type_definition(uri, 2, 26))
        assert len(locs) == 1
        assert locs[0]["range"]["start"]["line"] == 0


class TestDeclaration:
    def test_global_var_in_proc_returns_declaration_not_set(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc p {} {
                global foo
                set foo 42
                return $foo
            }
        """)
        lsp_server.open_ready(uri, src)
        locs = locations(lsp_server.declaration(uri, 3, 12))
        assert len(locs) == 1
        assert locs[0]["range"]["start"]["line"] == 1

    def test_falls_back_to_definition_for_proc(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, 'proc greet {name} { puts "Hello $name" }\ngreet World\n')
        locs = locations(lsp_server.declaration(uri, 1, 1))
        assert len(locs) == 1
        assert locs[0]["range"]["start"]["line"] == 0

    def test_scope_isolation_between_procs(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc alpha {} {
                global shared
                set shared 1
            }
            proc beta {} {
                global shared
                return $shared
            }
        """)
        lsp_server.open_ready(uri, src)
        lines = start_lines(lsp_server.declaration(uri, 6, 12))
        assert 5 in lines
        assert 1 not in lines
