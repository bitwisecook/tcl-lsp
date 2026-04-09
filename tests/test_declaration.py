"""Tests for the declaration provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsp.features.declaration import _scan_declarations, get_declaration

TEST_URI = "file:///test.tcl"


class TestScanDeclarations:
    def test_global_declaration(self):
        source = "global foo\nset foo 1\n"
        ranges = _scan_declarations(source, "foo")
        assert len(ranges) == 1
        assert ranges[0].start.line == 0

    def test_variable_declaration(self):
        source = "variable bar\nset bar 2\n"
        ranges = _scan_declarations(source, "bar")
        assert len(ranges) == 1
        assert ranges[0].start.line == 0

    def test_upvar_declaration(self):
        source = "upvar 1 caller_var local_var\n"
        ranges = _scan_declarations(source, "local_var")
        assert len(ranges) == 1
        assert ranges[0].start.line == 0

    def test_no_declaration(self):
        source = "set x 42\n"
        ranges = _scan_declarations(source, "x")
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
