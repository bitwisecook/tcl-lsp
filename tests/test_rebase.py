"""Tests for the canonical position-rebase primitives (``shared/rebase.py``)."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from shared import rebase
from shared.diagnostic import Range
from shared.tokens import SourcePosition, Token, TokenType


def _pos(line, char, offset):
    return SourcePosition(line=line, character=char, offset=offset)


class TestShiftPosition:
    def test_line_zero_shifts_char_and_offset(self):
        # A coordinate on the region's first line lands after base_char.
        out = rebase.shift_position(_pos(0, 3, 3), base_line=2, base_char=10, base_offset=100)
        assert out == _pos(2, 13, 103)

    def test_later_line_keeps_char(self):
        # A coordinate on a later line keeps its column; only line + offset move.
        out = rebase.shift_position(_pos(4, 7, 50), base_line=2, base_char=10, base_offset=100)
        assert out == _pos(6, 7, 150)

    def test_base_char_zero_is_pure_line_offset_shift(self):
        # The suffix-shift case: columns preserved on every line.
        assert rebase.shift_position(_pos(0, 5, 5), 3, 0, 30) == _pos(3, 5, 35)
        assert rebase.shift_position(_pos(2, 5, 25), 3, 0, 30) == _pos(5, 5, 55)


class TestShiftRangeToken:
    def test_shift_range(self):
        r = Range(start=_pos(0, 0, 0), end=_pos(1, 4, 9))
        out = rebase.shift_range(r, base_line=5, base_char=2, base_offset=100)
        assert out.start == _pos(5, 2, 100)  # line 0 → char shifts
        assert out.end == _pos(6, 4, 109)  # line 1 → char preserved

    def test_shift_token_preserves_text_and_quote(self):
        tok = Token(
            type=TokenType.ESC, text="x", start=_pos(2, 1, 20), end=_pos(2, 2, 21), in_quote=True
        )
        out = rebase.shift_token(tok, base_line=3, base_char=99, base_offset=5)
        assert out.text == "x"
        assert out.in_quote is True
        assert out.start == _pos(5, 1, 25)  # later line → char preserved
        assert out.end == _pos(5, 2, 26)


class TestDelegationParity:
    """The two former implementations must reproduce exactly via delegation."""

    def test_token_positions_matches_canonical(self):
        from compiler.parsing import token_positions as tp

        p = _pos(3, 4, 40)
        # token_positions' (offset_delta, line_delta) == canonical(base_char=0).
        assert tp.shift_position(p, 7, 2) == rebase.shift_position(
            p, base_line=2, base_char=0, base_offset=7
        )

    def test_conf_wrapped_alias_is_canonical(self):
        from analyser import conf_wrapped

        assert conf_wrapped._shift_position is rebase.shift_position
        assert conf_wrapped._shift_range is rebase.shift_range
