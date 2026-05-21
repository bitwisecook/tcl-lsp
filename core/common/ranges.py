"""Shared range and position helpers for token-to-semantic-model conversion."""

from __future__ import annotations

from bisect import bisect_right
from contextvars import ContextVar

from ..analysis.semantic_model import Range
from ..parsing.tokens import SourcePosition, Token, TokenType


def range_from_token(tok: Token) -> Range:
    """Build a Range covering exactly one token."""
    return Range(start=tok.start, end=tok.end)


def range_from_word_token(tok: Token) -> Range:
    """Range covering a full word token, *including* its closing delimiter.

    A braced/bracketed word token starts on the opening ``{`` / ``[`` but its
    ``end`` sits on the last *inner* character — the matching closer is exactly
    one position past ``end`` and ``tok.text`` omits it.  For ``STR``
    (``{...}``) and ``CMD`` (``[...]``) tokens, extend the range by that single
    closer character so a whole-word span covers the closer rather than
    stopping one short.

    The closer width comes from the token's *type*, so the span is derived
    straight from the token tree with no source slicing — which is what lets it
    work for tokens whose offsets are absolute but whose surrounding source
    string is only a substring (nested ``proc`` / loop bodies).  ``offset`` is
    always correct; ``character`` assumes a same-line closer, matching
    :func:`widen_range_for_closer`.
    """
    if tok.type in (TokenType.STR, TokenType.CMD):
        end = tok.end
        return Range(
            start=tok.start,
            end=SourcePosition(line=end.line, character=end.character + 1, offset=end.offset + 1),
        )
    return Range(start=tok.start, end=tok.end)


def range_from_tokens(tokens: list[Token]) -> Range:
    """Build a Range spanning from the first to the last token."""
    return Range(start=tokens[0].start, end=tokens[-1].end)


_RANGE_CLOSERS = {'"': '"', "{": "}", "[": "]"}


def widen_range_for_closer(source: str, range_: Range) -> Range:
    """Extend *range_* by one character to include a closing delimiter.

    The lexer's token end omits the closing ``}`` / ``"`` / ``]`` of a
    braced/quoted/bracketed word, so a range built from such a token stops one
    character short.  When *range_* opens with one of those delimiters and the
    matching closer immediately follows its (inclusive) end, return a range
    extended to cover the closer; otherwise return *range_* unchanged.  Only
    same-line closers are widened, so a multi-line ``{ ... \\n}`` is left alone.
    """
    start_off = range_.start.offset
    if not (0 <= start_off < len(source)):
        return range_
    closer = _RANGE_CLOSERS.get(source[start_off])
    end = range_.end
    if closer and end.offset + 1 < len(source) and source[end.offset + 1] == closer:
        return Range(
            start=range_.start,
            end=SourcePosition(line=end.line, character=end.character + 1, offset=end.offset + 1),
        )
    return range_


# Source of the document currently being serialised for the compiler
# explorer.  When set, :func:`widen_for_highlight` extends a braced/quoted
# word's range to cover its closing delimiter — word-token ranges follow the
# "inner-end" convention (the closer is excluded; consumers widen when they
# need it), so the explorer widens at serialisation time, the same way the
# diagnostics pipeline does.
_HIGHLIGHT_SOURCE: ContextVar[str | None] = ContextVar("_HIGHLIGHT_SOURCE", default=None)


def set_highlight_source(source: str | None):
    """Set the source used to widen highlight ranges; returns a reset token."""
    return _HIGHLIGHT_SOURCE.set(source)


def reset_highlight_source(token) -> None:
    _HIGHLIGHT_SOURCE.reset(token)


def widen_for_highlight(range_: Range) -> Range:
    """Widen *range_* to its closing delimiter against the active source.

    No-op when no highlight source is set (see :func:`set_highlight_source`).
    """
    source = _HIGHLIGHT_SOURCE.get()
    if source is None:
        return range_
    return widen_range_for_closer(source, range_)


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
