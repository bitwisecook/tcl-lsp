"""Shared range and position helpers for token-to-semantic-model conversion."""

from __future__ import annotations

from bisect import bisect_right

from compiler.parsing.tokens import SourcePosition, Token
from core.analysis.semantic_model import Range


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
    """Map an offset within *text* to an absolute SourcePosition.

    O(rel_offset) — prefer :func:`position_from_offset` when a
    ``line_starts`` index is available.
    """
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


def position_from_offset(
    absolute_offset: int,
    line_starts: list[int] | tuple[int, ...],
    source_len: int,
) -> SourcePosition:
    """O(log n) offset → SourcePosition using a pre-built line_starts index.

    Use this instead of :func:`position_from_relative` when a shared
    ``line_starts`` array is available (e.g. from ``DocumentBuffer`` or
    ``TclLexer._line_starts``).
    """
    if not line_starts:
        return SourcePosition(line=0, character=0, offset=0)
    safe = max(0, min(absolute_offset, source_len))
    line = bisect_right(line_starts, safe) - 1
    line = max(0, line)
    col = safe - line_starts[line]
    return SourcePosition(line=line, character=col, offset=safe)
