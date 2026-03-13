"""Shared token-content offset helpers."""

from __future__ import annotations

from ..analysis.semantic_model import Range
from .tokens import SourcePosition, Token


def token_content_shift(token: Token) -> int:
    """Return delimiter shift (0 or 1) for token content start."""
    return 1 if (token.end.offset - token.start.offset + 1) > len(token.text) else 0


def token_content_base(token: Token) -> tuple[int, int, int]:
    """Return (offset, line, col) where token text content begins."""
    shift = token_content_shift(token)
    return (
        token.start.offset + shift,
        token.start.line,
        token.start.character + shift,
    )


def shift_position(pos: SourcePosition, offset_delta: int, line_delta: int) -> SourcePosition:
    """Return a new ``SourcePosition`` with offset and line shifted by deltas.

    Character (column) is unchanged because the position within its line
    is the same — only the absolute offset and line number move.
    """
    return SourcePosition(
        line=pos.line + line_delta,
        character=pos.character,
        offset=pos.offset + offset_delta,
    )


def shift_token(tok: Token, offset_delta: int, line_delta: int) -> Token:
    """Return a new ``Token`` with start/end positions shifted."""
    return Token(
        type=tok.type,
        text=tok.text,
        start=shift_position(tok.start, offset_delta, line_delta),
        end=shift_position(tok.end, offset_delta, line_delta),
        in_quote=tok.in_quote,
    )


def shift_range(r: Range, offset_delta: int, line_delta: int) -> Range:
    """Return a new ``Range`` with start/end positions shifted."""
    return Range(
        start=shift_position(r.start, offset_delta, line_delta),
        end=shift_position(r.end, offset_delta, line_delta),
    )
