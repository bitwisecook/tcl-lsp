"""Tests for the declaration provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsp.features.declaration import _collect_declaration_ranges, get_declaration

TEST_URI = "file:///test.tcl"


class TestCollectDeclarationRanges:
    def test_global_declaration(self):
        source = "global foo\nset foo 1\n"
        ranges = _collect_declaration_ranges(source, "foo")
        assert len(ranges) == 1
        assert ranges[0].start.line == 0

    def test_variable_declaration(self):
        source = "variable bar\nset bar 2\n"
        ranges = _collect_declaration_ranges(source, "bar")
        assert len(ranges) == 1
        assert ranges[0].start.line == 0

    def test_upvar_integer_level(self):
        source = "upvar 1 caller_var local_var\n"
        ranges = _collect_declaration_ranges(source, "local_var")
        assert len(ranges) == 1
        assert ranges[0].start.line == 0

    def test_upvar_hash_level_arbitrary(self):
        """`upvar #3 other local` should recognise the level and declare `local`."""
        source = "upvar #3 other local\n"
        ranges = _collect_declaration_ranges(source, "local")
        assert len(ranges) == 1
        assert ranges[0].start.line == 0

    def test_upvar_no_level_defaults_to_pair(self):
        source = "upvar other my_var\n"
        ranges = _collect_declaration_ranges(source, "my_var")
        assert len(ranges) == 1
        assert ranges[0].start.line == 0

    def test_variable_with_value_pairs(self):
        """`variable a 1 b 2` declares `a` and `b`, not `1`/`2`."""
        source = "namespace eval foo { variable a 1 b 2 }\n"
        assert len(_collect_declaration_ranges(source, "a")) == 1
        assert len(_collect_declaration_ranges(source, "b")) == 1
        assert len(_collect_declaration_ranges(source, "1")) == 0

    def test_no_declaration(self):
        source = "set x 42\n"
        ranges = _collect_declaration_ranges(source, "x")
        assert ranges == []


class TestGetDeclaration:
    def test_global_var_in_proc_returns_declaration_not_set(self):
        source = textwrap.dedent("""\
            proc p {} {
                global foo
                set foo 42
                return $foo
            }
        """)
        # Cursor on `$foo` in `return $foo` at line 3.
        locs = get_declaration(source, TEST_URI, 3, 12)
        assert len(locs) == 1
        # Should return the `global foo` line (line 1), not the `set foo 42`.
        assert locs[0].range.start.line == 1

    def test_falls_back_to_definition_for_proc(self):
        source = textwrap.dedent("""\
            proc greet {name} { puts "Hello $name" }
            greet World
        """)
        # Cursor on `greet` at line 1 — no declaration, should return proc def.
        locs = get_declaration(source, TEST_URI, 1, 1)
        assert len(locs) == 1
        assert locs[0].range.start.line == 0

    def test_scope_isolation_between_procs(self):
        """`global x` in proc A must not leak into the declaration result
        for a cursor on `$x` inside an unrelated proc B."""
        source = textwrap.dedent("""\
            proc alpha {} {
                global shared
                set shared 1
            }
            proc beta {} {
                global shared
                return $shared
            }
        """)
        # Cursor on `$shared` at line 6 (inside beta).
        locs = get_declaration(source, TEST_URI, 6, 12)
        # Should find beta's global at line 5, NOT alpha's global at line 1.
        lines = [loc.range.start.line for loc in locs]
        assert 5 in lines
        assert 1 not in lines, (
            f"alpha's global should not leak into beta's scope, got lines={lines}"
        )
