"""Tests for the type-definition provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from server.features.type_definition import get_type_definition

TEST_URI = "file:///test.tcl"


class TestTypeDefinitionVariable:
    def test_set_with_class_new(self):
        source = textwrap.dedent("""\
            oo::class create Point {
                variable x y
                constructor {a b} { set x $a; set y $b }
            }
            set p [Point new 1 2]
            puts $p
        """)
        # Cursor on `$p` at line 5, col 6.
        locs = get_type_definition(source, TEST_URI, 5, 6)
        assert len(locs) == 1
        # Name range points into the Point class declaration at line 0.
        assert locs[0].range.start.line == 0

    def test_plain_set_returns_empty(self):
        source = textwrap.dedent("""\
            set x 42
            puts $x
        """)
        locs = get_type_definition(source, TEST_URI, 1, 6)
        assert locs == []


class TestTypeDefinitionMethodReceiver:
    def test_my_call_returns_enclosing_class(self):
        source = textwrap.dedent("""\
            oo::class create Animal {
                method speak {} { return "..." }
                method greet {} { my speak }
            }
        """)
        # Cursor on `speak` in `my speak` at line 2.
        locs = get_type_definition(source, TEST_URI, 2, 26)
        assert len(locs) == 1
        # Points at the Animal class name range on line 0.
        assert locs[0].range.start.line == 0
