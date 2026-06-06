"""Go-to-definition, end-to-end against the packaged server.

Ported from ``tests/test_definition.py`` plus the VS Code
``definition.test.ts`` scenario (navigate from a call site to the proc).
"""

from __future__ import annotations

import textwrap

from ._lsp_helpers import locations


class TestProcDefinition:
    def test_jump_to_proc(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, 'proc greet {name} { puts "Hello $name" }\ngreet World\n')
        locs = locations(lsp_server.definition(uri, 1, 2))
        assert len(locs) >= 1
        assert locs[0]["uri"] == uri
        assert locs[0]["range"]["start"]["line"] == 0

    def test_no_definition_for_builtin(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "puts hello\n")
        assert locations(lsp_server.definition(uri, 0, 2)) == []

    def test_proc_in_namespace(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            namespace eval myns {
                proc helper {} { return 1 }
            }
            myns::helper
        """)
        lsp_server.open_ready(uri, src)
        locs = locations(lsp_server.definition(uri, 3, 7))
        assert len(locs) >= 1
        assert locs[0]["range"]["start"]["line"] == 1

    def test_recursive_call_navigates_to_definition(self, lsp_server, uri_factory):
        # Mirrors editors/vscode/src/test/definition.test.ts.
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc fib {n} {
                if {$n < 2} { return $n }
                return [expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}]
            }
            puts "fib(10) = [fib 10]"
        """)
        lsp_server.open_ready(uri, src)
        # cursor on the [fib 10] call on the last line
        line = src.split("\n").index('puts "fib(10) = [fib 10]"')
        col = 'puts "fib(10) = ['.index("[") + 1
        locs = locations(lsp_server.definition(uri, line, col))
        assert len(locs) >= 1
        assert locs[0]["range"]["start"]["line"] == 0


class TestVariableDefinition:
    def test_jump_to_var_definition(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 42\nputs $x\n")
        locs = locations(lsp_server.definition(uri, 1, 7))
        assert len(locs) >= 1
        assert locs[0]["range"]["start"]["line"] == 0

    def test_var_in_proc(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc foo {} {
                set local 42
                puts $local
            }
        """)
        lsp_server.open_ready(uri, src)
        locs = locations(lsp_server.definition(uri, 2, 11))
        assert len(locs) >= 1
        assert locs[0]["range"]["start"]["line"] == 1

    def test_no_definition_for_unknown_var(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "puts $unknown\n")
        assert locations(lsp_server.definition(uri, 0, 8)) == []

    def test_namespace_var_definition(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            namespace eval myns {
                variable nsVar 1
                puts $nsVar
            }
        """)
        lsp_server.open_ready(uri, src)
        locs = locations(lsp_server.definition(uri, 2, 10))
        assert len(locs) >= 1
        assert locs[0]["range"]["start"]["line"] == 1
