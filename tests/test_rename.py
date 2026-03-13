"""Tests for the rename provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsp.features.rename import get_rename_edits, prepare_rename

TEST_URI = "file:///test.tcl"


class TestPrepareRename:
    def test_proc_name(self):
        source = textwrap.dedent("""\
            proc greet {name} { puts "Hello $name" }
            greet World
        """)
        result = prepare_rename(source, TEST_URI, 0, 6)
        assert result is not None
        assert result.placeholder == "greet"

    def test_variable(self):
        source = "set x 42\nputs $x"
        result = prepare_rename(source, TEST_URI, 1, 7)
        assert result is not None
        assert result.placeholder == "x"

    def test_variable_from_definition_site(self):
        source = "set x 42\nputs $x"
        result = prepare_rename(source, TEST_URI, 0, 4)
        assert result is not None
        assert result.placeholder == "x"

    def test_builtin_rejected(self):
        source = "puts hello"
        result = prepare_rename(source, TEST_URI, 0, 1)
        assert result is None

    def test_unknown_rejected(self):
        source = "something_unknown"
        result = prepare_rename(source, TEST_URI, 0, 5)
        assert result is None


class TestRenameProc:
    def test_rename_definition_and_calls(self):
        source = textwrap.dedent("""\
            proc greet {name} { puts "Hello $name" }
            greet World
            greet Everyone
        """)
        edit = get_rename_edits(source, TEST_URI, 0, 6, "welcome")
        assert edit is not None
        assert edit.changes is not None
        assert TEST_URI in edit.changes
        edits = edit.changes[TEST_URI]
        # Should have definition + 2 call sites
        assert len(edits) >= 3
        # All edits should replace with the new name
        assert all(e.new_text == "welcome" for e in edits)

    def test_rename_from_call_site(self):
        source = textwrap.dedent("""\
            proc greet {name} { puts "Hello $name" }
            greet World
        """)
        edit = get_rename_edits(source, TEST_URI, 1, 0, "welcome")
        assert edit is not None
        assert edit.changes is not None
        assert TEST_URI in edit.changes
        edits = edit.changes[TEST_URI]
        assert len(edits) >= 2


class TestRenameVariable:
    def test_rename_var(self):
        source = textwrap.dedent("""\
            set x 42
            puts $x
        """)
        edit = get_rename_edits(source, TEST_URI, 1, 7, "newvar")
        assert edit is not None
        assert edit.changes is not None
        assert TEST_URI in edit.changes
        edits = edit.changes[TEST_URI]
        assert len(edits) >= 1
        assert all(e.new_text == "newvar" for e in edits)

    def test_rename_var_from_definition_site(self):
        source = textwrap.dedent("""\
            set x 42
            puts $x
        """)
        edit = get_rename_edits(source, TEST_URI, 0, 4, "newvar")
        assert edit is not None
        assert edit.changes is not None
        assert TEST_URI in edit.changes
        edits = edit.changes[TEST_URI]
        assert len(edits) >= 2
        assert all(e.new_text == "newvar" for e in edits)

    def test_rename_respects_scope(self):
        source = textwrap.dedent("""\
            set x 1
            proc demo {} {
                set x 2
                puts $x
            }
        """)
        # Rename the inner x (line 3)
        edit = get_rename_edits(source, TEST_URI, 3, 10, "y")
        assert edit is not None
        assert edit.changes is not None
        edits = edit.changes[TEST_URI]
        # Should only rename within the proc scope
        lines = {e.range.start.line for e in edits}
        assert 0 not in lines  # global x untouched


class TestRenameSafety:
    def test_rejects_invalid_new_symbol_name(self):
        source = textwrap.dedent("""\
            proc greet {name} { puts "Hello $name" }
            greet World
        """)
        edit = get_rename_edits(source, TEST_URI, 0, 6, "bad-name")
        assert edit is None

    def test_rejects_proc_collision(self):
        source = textwrap.dedent("""\
            proc greet {name} { puts "Hello $name" }
            proc hello {name} { puts "Hi $name" }
            greet World
        """)
        edit = get_rename_edits(source, TEST_URI, 0, 6, "hello")
        assert edit is None

    def test_rejects_proc_rename_to_builtin(self):
        source = textwrap.dedent("""\
            proc greet {name} { puts "Hello $name" }
            greet World
        """)
        edit = get_rename_edits(source, TEST_URI, 0, 6, "puts")
        assert edit is None

    def test_rejects_var_collision_in_same_scope(self):
        source = textwrap.dedent("""\
            proc demo {} {
                set x 1
                set y 2
                puts $x
            }
        """)
        edit = get_rename_edits(source, TEST_URI, 3, 10, "y")
        assert edit is None
