"""Targeted tests for ``_materialise_rust_optimisations``.

Covers the Rust→Python span conversion edge cases called out in the
C30/C32 PR review:

- Empty spans (``start == end``).
- Single-byte spans.
- Multi-line spans.
- Non-ASCII / Unicode input so the UTF-8 byte-offset → UTF-16 column
  translation is exercised.
- Inclusive-end semantics of Python ``Range.end`` vs. Rust's
  exclusive-end ``Span``.

These tests drive ``_materialise_rust_optimisations`` directly rather
than through the PyO3 bridge so they run on every developer machine,
regardless of whether the Rust wheel is installed.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler.optimiser._manager import _materialise_rust_optimisations


def _one(source: str, start: int, end: int):
    """Helper: materialise a single fake Rust optimisation tuple."""
    tuples = [("O101", "msg", start, end, "repl", None, False)]
    out = _materialise_rust_optimisations(source, tuples)
    assert len(out) == 1
    return out[0]


def test_empty_span_start_equals_end_collapses_to_single_position():
    src = "set x 1\n"
    opt = _one(src, 4, 4)
    assert opt.range.start.offset == 4
    # Python Range.end is inclusive; an empty span collapses to a
    # single position at `start`.
    assert opt.range.end.offset == 4
    assert opt.range.start.line == 0
    assert opt.range.start.character == 4


def test_single_byte_span_ascii():
    src = "set x 1\n"
    opt = _one(src, 4, 5)  # covers just "x"
    assert opt.range.start.offset == 4
    assert opt.range.end.offset == 4  # inclusive end
    assert opt.range.start.character == 4
    assert opt.range.end.character == 4


def test_multi_line_span():
    src = "set x 1\nset y 2\n"
    # Rust exclusive span covering both statements: 0..15.
    opt = _one(src, 0, 15)
    assert opt.range.start.line == 0
    assert opt.range.start.character == 0
    assert opt.range.start.offset == 0
    assert opt.range.end.line == 1
    # end=15 exclusive → end_incl=14 → line 1, col 6 (the `2`).
    assert opt.range.end.offset == 14
    assert opt.range.end.character == 6


def test_multi_byte_unicode_produces_utf16_column():
    # `é` is two UTF-8 bytes (0xC3, 0xA9) but one UTF-16 code unit.
    # Using `é` as the second character means column 2 in the
    # *UTF-16* sense should follow the first byte pair.
    src = "aéb"
    utf8 = src.encode("utf-8")
    assert len(utf8) == 4  # 'a' + 2 bytes for 'é' + 'b'

    # Rust span covering just `b` (byte offsets 3..4).
    opt = _one(src, 3, 4)
    assert opt.range.start.offset == 3
    # 'a' + 'é' = 2 UTF-16 code units.
    assert opt.range.start.character == 2


def test_emoji_produces_surrogate_pair_utf16_column():
    # 🦀 is U+1F980, encoded as 4 UTF-8 bytes and as a UTF-16
    # surrogate pair (2 code units).
    src = "🦀x"
    utf8 = src.encode("utf-8")
    assert len(utf8) == 5  # 4-byte emoji + 'x'

    # Span covering just `x` at UTF-8 offset 4..5.
    opt = _one(src, 4, 5)
    assert opt.range.start.offset == 4
    # Emoji takes 2 UTF-16 code units → column 2.
    assert opt.range.start.character == 2


def test_offset_past_end_clamps():
    src = "abc"
    opt = _one(src, 5, 10)
    # Both positions clamp to the source length.
    assert opt.range.start.offset == 3
    assert opt.range.end.offset == 3


def test_line_break_advances_line_number():
    src = "line0\nline1\nline2"
    # Offset 0 → line 0, offset 6 → line 1 (first char after '\n').
    opt = _one(src, 6, 7)
    assert opt.range.start.line == 1
    assert opt.range.start.character == 0
    assert opt.range.start.offset == 6
