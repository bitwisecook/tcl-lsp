"""Shared range and position helpers for token-to-semantic-model conversion."""

from __future__ import annotations

from ..analysis.semantic_model import Range
from ..parsing.tokens import SourcePosition, Token


def range_from_token(tok: Token) -> Range:
    """Build a Range covering exactly one token."""
    return Range(start=tok.start, end=tok.end)


def range_from_tokens(tokens: list[Token]) -> Range:
    """Build a Range spanning from the first to the last token."""
    return Range(start=tokens[0].start, end=tokens[-1].end)


def position_from_relative(
    text: str,
    rel_offset: int,
    *,
    base_line: int,
    base_col: int,
    base_offset: int,
) -> SourcePosition:
    """Map an offset within *text* to an absolute SourcePosition."""
    rel = max(0, min(rel_offset, len(text)))
    line = base_line
    col = base_col
    for ch in text[:rel]:
        if ch == "\n":
            line += 1
            col = 0
        else:
            col += 1
    return SourcePosition(line=line, character=col, offset=base_offset + rel)
