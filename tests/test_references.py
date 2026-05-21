"""Tests for the find-references provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsp.features.references import get_references

TEST_URI = "file:///test.tcl"


class TestProcReferences:
    def test_find_proc_definition_and_calls(self):
        source = textwrap.dedent("""\
            proc greet {name} { puts "Hello $name" }
            greet World
            greet Everyone
        """)
        refs = get_references(source, TEST_URI, 0, 6)
        # Should include definition + call sites
        assert len(refs) >= 2

    def test_exclude_declaration(self):
        source = textwrap.dedent("""\
            proc greet {name} { puts "Hello $name" }
            greet World
        """)
        refs_with = get_references(source, TEST_URI, 0, 6, include_declaration=True)
        refs_without = get_references(source, TEST_URI, 0, 6, include_declaration=False)
        assert len(refs_with) >= len(refs_without)

    def test_find_indented_proc_call(self):
        source = textwrap.dedent("""\
            proc greet {} { return }
                greet
        """)
        refs = get_references(source, TEST_URI, 0, 6)
        call_lines = {loc.range.start.line for loc in refs}
        assert 1 in call_lines

    def test_find_qualified_proc_call_sites(self):
        source = textwrap.dedent("""\
            namespace eval myns {
                proc helper {} { return 1 }
            }
            myns::helper
            ::myns::helper
        """)
        refs = get_references(source, TEST_URI, 1, 10)
        call_lines = {loc.range.start.line for loc in refs}
        assert 3 in call_lines
        assert 4 in call_lines

    def test_find_proc_call_in_nested_braced_body(self):
        source = textwrap.dedent("""\
            proc greet {} { return }
            if {1} {
                greet
            }
        """)
        refs = get_references(source, TEST_URI, 0, 6)
        call_lines = {loc.range.start.line for loc in refs}
        assert 2 in call_lines

    def test_qualified_calls_do_not_cross_namespace(self):
        source = textwrap.dedent("""\
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
        refs = get_references(source, TEST_URI, 1, 10)
        call_lines = {loc.range.start.line for loc in refs}
        assert 2 in call_lines
        assert 8 in call_lines
        assert 6 not in call_lines
        assert 9 not in call_lines


class TestVariableReferences:
    def test_find_var_refs(self):
        source = "set x 42\nputs $x"
        refs = get_references(source, TEST_URI, 1, 7)
        # Should include the ``set x`` definition (line 0) and the ``$x``
        # read (line 1) — both at the variable name's column.
        starts = {(r.range.start.line, r.range.start.character) for r in refs}
        assert starts == {(0, 4), (1, 5)}, starts

    def test_multiple_var_refs(self):
        source = "set x 1\nset x 2\nputs $x"
        # Position on $x at line 2, col 6 (the 'x' after '$')
        refs = get_references(source, TEST_URI, 2, 6)
        # Both ``set x`` writes (lines 0,1) and the ``$x`` read (line 2).
        starts = {(r.range.start.line, r.range.start.character) for r in refs}
        assert starts == {(0, 4), (1, 4), (2, 5)}, starts

    def test_no_refs_for_unknown(self):
        refs = get_references("puts hello", TEST_URI, 0, 6)
        assert len(refs) == 0

    def test_find_namespace_var_refs(self):
        source = textwrap.dedent("""\
            namespace eval myns {
                variable nsVar 1
                puts $nsVar
            }
        """)
        refs = get_references(source, TEST_URI, 2, 10)
        lines = {loc.range.start.line for loc in refs}
        assert 1 in lines
        assert 2 in lines

    def test_var_refs_respect_shadowing_global_target(self):
        source = textwrap.dedent("""\
            set x 1
            puts $x
            proc demo {} {
                set x 2
                puts $x
            }
            demo
        """)
        refs = get_references(source, TEST_URI, 1, 6)
        lines = {loc.range.start.line for loc in refs}
        assert lines == {0, 1}

    def test_var_refs_respect_shadowing_local_target(self):
        source = textwrap.dedent("""\
            set x 1
            puts $x
            proc demo {} {
                set x 2
                puts $x
            }
            demo
        """)
        refs = get_references(source, TEST_URI, 4, 10)
        lines = {loc.range.start.line for loc in refs}
        assert lines == {3, 4}


class TestClassSuperclassMixinReferences:
    """A class's find-references spans its use in superclass/mixin declarations."""

    def test_superclass_and_mixin_references(self):
        source = textwrap.dedent("""\
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
        refs = get_references(source, TEST_URI, 0, 17)  # cursor on Animal definition
        starts = {(loc.range.start.line, loc.range.start.character) for loc in refs}
        assert (0, 17) in starts  # definition
        assert (4, 15) in starts  # superclass Animal
        assert (8, 10) in starts  # mixin Animal
