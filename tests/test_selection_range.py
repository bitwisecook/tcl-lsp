"""Tests for the selection range provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from lsp.features.selection_range import get_selection_ranges


def _chain_to_list(sr: types.SelectionRange) -> list[types.Range]:
    """Flatten a SelectionRange linked list into a list of ranges (inner to outer)."""
    result = []
    node: types.SelectionRange | None = sr
    while node is not None:
        result.append(node.range)
        node = node.parent
    return result


class TestSelectionRangeBasic:
    def test_cursor_on_variable_in_proc(self):
        source = textwrap.dedent("""\
            proc greet {name} {
                puts "Hello $name"
            }
        """)
        pos = types.Position(line=1, character=10)  # inside "Hello $name"
        results = get_selection_ranges(source, [pos])
        assert len(results) == 1
        chain = _chain_to_list(results[0])
        # Should have multiple levels expanding outward
        assert len(chain) >= 2
        # Outermost should be the whole file
        last = chain[-1]
        assert last.start.line == 0 and last.start.character == 0

    def test_cursor_on_command_name_top_level(self):
        source = "puts hello\n"
        pos = types.Position(line=0, character=1)  # on "puts"
        results = get_selection_ranges(source, [pos])
        assert len(results) == 1
        chain = _chain_to_list(results[0])
        # At minimum: word → command → file
        assert len(chain) >= 2
        # Outermost is whole file
        last = chain[-1]
        assert last.start.line == 0 and last.start.character == 0

    def test_cursor_inside_namespace(self):
        source = textwrap.dedent("""\
            namespace eval myns {
                proc helper {} {
                    return 1
                }
            }
        """)
        pos = types.Position(line=2, character=8)  # on "return"
        results = get_selection_ranges(source, [pos])
        assert len(results) == 1
        chain = _chain_to_list(results[0])
        # Should have: word → command → proc body → namespace body → file
        assert len(chain) >= 3

    def test_empty_file(self):
        results = get_selection_ranges("", [types.Position(line=0, character=0)])
        assert len(results) == 1
        chain = _chain_to_list(results[0])
        # At minimum the whole-file range
        assert len(chain) >= 1

    def test_multiple_positions(self):
        source = textwrap.dedent("""\
            set x 42
            set y 99
        """)
        positions = [
            types.Position(line=0, character=4),
            types.Position(line=1, character=4),
        ]
        results = get_selection_ranges(source, positions)
        assert len(results) == 2
        # Each should have its own chain
        for r in results:
            chain = _chain_to_list(r)
            assert len(chain) >= 1


class TestSelectionRangeChainOrder:
    def test_ranges_expand_outward(self):
        """Each range in the chain should be >= the previous one."""
        source = textwrap.dedent("""\
            proc add {a b} {
                expr {$a + $b}
            }
        """)
        pos = types.Position(line=1, character=6)  # inside expr body
        results = get_selection_ranges(source, [pos])
        chain = _chain_to_list(results[0])

        for i in range(1, len(chain)):
            prev = chain[i - 1]
            curr = chain[i]
            # Current range should start at or before prev
            assert curr.start.line < prev.start.line or (
                curr.start.line == prev.start.line and curr.start.character <= prev.start.character
            )
            # Current range should end at or after prev
            assert curr.end.line > prev.end.line or (
                curr.end.line == prev.end.line and curr.end.character >= prev.end.character
            )

    def test_no_duplicate_ranges(self):
        """Adjacent ranges in the chain should never be identical."""
        source = "set x 42\n"
        pos = types.Position(line=0, character=4)
        results = get_selection_ranges(source, [pos])
        chain = _chain_to_list(results[0])

        for i in range(1, len(chain)):
            prev = chain[i - 1]
            curr = chain[i]
            # At least one boundary must differ
            assert not (
                prev.start.line == curr.start.line
                and prev.start.character == curr.start.character
                and prev.end.line == curr.end.line
                and prev.end.character == curr.end.character
            ), f"Duplicate range at index {i}: {curr}"
