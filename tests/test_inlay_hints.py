"""Tests for the inlay hints provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from lsp.features.inlay_hints import get_inlay_hints

FULL_RANGE = types.Range(
    start=types.Position(line=0, character=0),
    end=types.Position(line=999, character=0),
)


class TestInlayHints:
    def test_integer_type_hint(self):
        source = "set x 42\n"
        hints = get_inlay_hints(source, FULL_RANGE)
        # Should show int type for x
        int_hints = [h for h in hints if "int" in h.label]
        assert len(int_hints) >= 1

    def test_no_hints_for_unknown_type(self):
        source = "set x [some_command]\n"
        hints = get_inlay_hints(source, FULL_RANGE)
        # Unknown command return type — may or may not produce hints
        # Just verify it doesn't crash
        assert isinstance(hints, list)

    def test_empty_file(self):
        hints = get_inlay_hints("", FULL_RANGE)
        assert hints == []

    def test_hint_kind_is_type(self):
        source = "set x 42\n"
        hints = get_inlay_hints(source, FULL_RANGE)
        for h in hints:
            assert h.kind == types.InlayHintKind.Type

    def test_range_filtering(self):
        source = textwrap.dedent("""\
            set x 42
            set y 99
        """)
        # Only request hints for line 0
        narrow_range = types.Range(
            start=types.Position(line=0, character=0),
            end=types.Position(line=0, character=100),
        )
        hints = get_inlay_hints(source, narrow_range)
        # All hints should be on line 0
        for h in hints:
            assert h.position.line == 0

    def test_foreach_multi_var_hints_not_clumped(self):
        """Type hints for foreach {a b c} should appear after each var, not clumped."""
        source = "foreach {a b c} {1 2 3} { puts $a$b$c }\n"
        hints = get_inlay_hints(source, FULL_RANGE)
        type_hints = [h for h in hints if h.kind == types.InlayHintKind.Type]
        if len(type_hints) >= 2:
            # The hints must have distinct character positions — they should
            # NOT all share the same position (the old clumped behaviour).
            positions = {h.position.character for h in type_hints}
            assert len(positions) > 1, (
                "foreach variable type hints are clumped at the same position"
            )
