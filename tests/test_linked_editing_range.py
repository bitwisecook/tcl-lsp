"""Tests for the linkedEditingRange provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from server.features.linked_editing_range import get_linked_editing_ranges

TEST_URI = "file:///test.tcl"  # noqa: F841 (keep parity with other tests)


class TestLinkedEditingRange:
    def test_recursive_self_calls_linked_with_declaration(self):
        source = textwrap.dedent("""\
            proc factorial {n} {
                if {$n <= 1} { return 1 }
                return [expr {$n * [factorial [expr {$n - 1}]]}]
            }
        """)
        # Cursor on the `factorial` declaration at line 0, col 5.
        result = get_linked_editing_ranges(source, 0, 5)
        assert result is not None
        assert len(result.ranges) >= 2
        # Declaration at line 0 is included.
        assert any(r.start.line == 0 for r in result.ranges)
        # The self-call at line 2 is included.
        assert any(r.start.line == 2 for r in result.ranges)
        # Word pattern is sensible.
        assert "[A-Za-z" in (result.word_pattern or "")

    def test_non_recursive_proc_returns_none(self):
        source = "proc greet {} { return hi }\n"
        # Only a declaration, no self-calls.
        result = get_linked_editing_ranges(source, 0, 5)
        assert result is None

    def test_cursor_not_on_proc_returns_none(self):
        source = "proc f {} { return 1 }\n"
        result = get_linked_editing_ranges(source, 0, 0)
        assert result is None

    def test_cursor_on_self_call_site(self):
        source = textwrap.dedent("""\
            proc loop {} {
                loop
            }
        """)
        # Cursor on the `loop` self-call at line 1, col 5.
        result = get_linked_editing_ranges(source, 1, 5)
        assert result is not None
        assert len(result.ranges) >= 2
