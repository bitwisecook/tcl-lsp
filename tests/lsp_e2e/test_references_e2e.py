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

"""Find-references, end-to-end against the packaged server.

Full-parity port of ``tests/test_references.py``.
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

    def test_find_indented_proc_call(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc greet {} { return }\n    greet\n")
        assert 1 in start_lines(lsp_server.references(uri, 0, 6))

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

    def test_find_proc_call_in_nested_braced_body(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc greet {} { return }
            if {1} {
                greet
            }
        """)
        lsp_server.open_ready(uri, src)
        assert 2 in start_lines(lsp_server.references(uri, 0, 6))

    def test_qualified_calls_do_not_cross_namespace(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            namespace eval a {
                proc helper {} { return 1 }
                helper
            }
            namespace eval b {
                proc helper {} { return 2 }
                helper
            }
            a::helper
            b::helper
        """)
        lsp_server.open_ready(uri, src)
        lines = start_lines(lsp_server.references(uri, 1, 10))
        assert 2 in lines
        assert 8 in lines
        assert 6 not in lines
        assert 9 not in lines


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

    def test_find_namespace_var_refs(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            namespace eval myns {
                variable nsVar 1
                puts $nsVar
            }
        """)
        lsp_server.open_ready(uri, src)
        lines = start_lines(lsp_server.references(uri, 2, 10))
        assert 1 in lines
        assert 2 in lines

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


class TestClassSuperclassMixinReferences:
    """A class's find-references spans its use in superclass/mixin declarations."""

    def test_superclass_and_mixin_references(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            oo::class create Animal {
                method speak {} {return noise}
            }
            oo::class create Dog {
                superclass Animal
                method speak {} {return woof}
            }
            oo::define Cat {
                mixin Animal
            }
        """)
        lsp_server.open_ready(uri, src)
        s = starts(lsp_server.references(uri, 0, 17))
        assert (0, 17) in s  # definition
        assert (4, 15) in s  # superclass Animal
        assert (8, 10) in s  # mixin Animal
