"""Tests for the go-to-implementation provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsp.features.implementation import get_implementations

TEST_URI = "file:///test.tcl"


class TestImplementationClass:
    def test_class_name_returns_subclasses(self):
        source = textwrap.dedent("""\
            oo::class create Animal {}
            oo::class create Dog { superclass Animal }
            oo::class create Cat { superclass Animal }
        """)
        # Cursor on `Animal` in its declaration (line 0, col 18 = inside "Animal")
        locs = get_implementations(source, TEST_URI, 0, 18)
        lines = sorted(loc.range.start.line for loc in locs)
        assert 1 in lines
        assert 2 in lines

    def test_class_without_subclasses_returns_empty(self):
        source = "oo::class create Lonely {}\n"
        locs = get_implementations(source, TEST_URI, 0, 18)
        assert locs == []

    def test_no_symbol_at_cursor(self):
        source = "oo::class create Dog {}\n"
        locs = get_implementations(source, TEST_URI, 0, 0)
        assert locs == []


class TestImplementationMethod:
    def test_method_overrides_returned(self):
        source = textwrap.dedent("""\
            oo::class create Animal {
                method speak {} { return "..." }
            }
            oo::class create Dog {
                superclass Animal
                method speak {} { return "woof" }
            }
            oo::class create Cat {
                superclass Animal
                method speak {} { return "meow" }
            }
        """)
        # Cursor on `speak` in Animal's declaration (line 1, col 11).
        locs = get_implementations(source, TEST_URI, 1, 11)
        lines = sorted(loc.range.start.line for loc in locs)
        # Animal's own method (line 1), Dog's override (line 5), Cat's (line 9).
        assert 1 in lines
        assert 5 in lines
        assert 9 in lines

    def test_method_call_in_base_resolves_overrides(self):
        source = textwrap.dedent("""\
            oo::class create Animal {
                method speak {} { return "..." }
                method greet {} { my speak }
            }
            oo::class create Dog {
                superclass Animal
                method speak {} { return "woof" }
            }
        """)
        # Cursor on `speak` in `my speak` call at line 2.
        locs = get_implementations(source, TEST_URI, 2, 26)
        lines = sorted(loc.range.start.line for loc in locs)
        # Should include the override in Dog.
        assert 6 in lines
