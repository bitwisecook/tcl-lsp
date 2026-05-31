"""Canonical position-rebase primitives.

Shifting a source coordinate by a base ``(line, char, offset)`` is the core
"move a byte-identical result to a new location" operation.  It is used by:

* the incremental reparse suffix-shift (``compiler/parsing/token_positions.py``),
* conf-wrapped iRules body embedding (``analyser/conf_wrapped.py``),
* the ``AnchorTable``-driven analysis rebase (``shared/rope.py`` + later phases).

These were three separate implementations of the same arithmetic; this module
is the single source of truth.

The line-0-vs-later rule matters: a coordinate on the region's **first** line
shifts its *column* by ``base_char`` (it lands after the base position on that
line); coordinates on **later** lines keep their column (they begin a fresh
line at column 0) and shift only line + offset.  The byte offset always shifts
by ``base_offset``.  Passing ``base_char=0`` collapses this to the pure "moved
down N lines / over M bytes, columns preserved" case (the suffix-shift use).
"""

from __future__ import annotations

from shared.diagnostic import Range
from shared.tokens import SourcePosition, Token


def shift_position(
    pos: SourcePosition, base_line: int, base_char: int, base_offset: int
) -> SourcePosition:
    """Shift *pos* by a base line/character/offset (line-0-vs-later rule)."""
    if pos.line == 0:
        return SourcePosition(
            line=pos.line + base_line,
            character=pos.character + base_char,
            offset=pos.offset + base_offset,
        )
    return SourcePosition(
        line=pos.line + base_line,
        character=pos.character,
        offset=pos.offset + base_offset,
    )


def shift_range(rng: Range, base_line: int, base_char: int, base_offset: int) -> Range:
    """Shift both endpoints of *rng*."""
    return Range(
        start=shift_position(rng.start, base_line, base_char, base_offset),
        end=shift_position(rng.end, base_line, base_char, base_offset),
    )


def shift_token(tok: Token, base_line: int, base_char: int, base_offset: int) -> Token:
    """Shift a token's start/end positions."""
    return Token(
        type=tok.type,
        text=tok.text,
        start=shift_position(tok.start, base_line, base_char, base_offset),
        end=shift_position(tok.end, base_line, base_char, base_offset),
        in_quote=tok.in_quote,
    )
