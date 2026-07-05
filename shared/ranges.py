# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Shared range and position helpers for token-to-semantic-model conversion."""

from __future__ import annotations

from bisect import bisect_right
from contextvars import ContextVar

from shared.diagnostic import Range
from shared.tokens import SourcePosition, Token, TokenType


def range_from_token(tok: Token) -> Range:
    """Build a Range covering exactly one token."""
    return Range(start=tok.start, end=tok.end)


def _closer_position(end: SourcePosition, last_inner_char: str) -> SourcePosition:
    """Position of the closing delimiter one character past *end*.

    *last_inner_char* is the character at ``end.offset`` (the last character
    inside the word).  When it is a newline the closer sits at column 0 of the
    next line, so the line/column must advance accordingly — keeping them
    consistent with ``offset`` for line/column-based consumers such as
    :func:`server._lsp_conv.to_lsp_range`.
    """
    if last_inner_char in ("\n", "\r"):
        return SourcePosition(line=end.line + 1, character=0, offset=end.offset + 1)
    return SourcePosition(line=end.line, character=end.character + 1, offset=end.offset + 1)


def range_from_word_token(tok: Token) -> Range:
    """Range covering a full word token, *including* its closing delimiter.

    A braced/bracketed word token starts on the opening ``{`` / ``[`` but its
    ``end`` normally sits on the last *inner* character — the matching closer is
    exactly one position past ``end`` and ``tok.text`` omits it.  For ``STR``
    (``{...}``) and ``CMD`` (``[...]``) tokens, extend the range by that single
    closer character so a whole-word span covers the closer rather than
    stopping one short.

    An *empty* ``{}`` / ``[]`` is the exception: it has no inner character, so
    the lexer already places ``end`` *on* the closer.  Advancing again would
    overshoot into whatever follows — for a trailing ``{}`` argument that is the
    enclosing body's closing brace, which corrupts the command span and
    produces a phantom stray ``}`` (issue #527).  ``span_extra`` — the bytes the
    span carries beyond the opener and content — is ``0`` when the closer is
    excluded and ``>= 1`` when it is already covered, so it distinguishes the
    two cases without touching the source.

    The closer width comes from the token's *type*, so the span is derived
    straight from the token tree with no source slicing — which is what lets it
    work for tokens whose offsets are absolute but whose surrounding source
    string is only a substring (nested ``proc`` / loop bodies).  Quoted
    ``"..."`` words (which lex as ``ESC``) are deliberately **not** widened
    here: the closer character cannot be derived from the token *type* without
    source, and ``cmd.range`` consumers (W105 unbraced-body detection, segmenter
    tiling) rely on the inner-end for quoted bodies.  A consumer that needs a
    quoted word's closing ``"`` uses the source-aware
    :func:`word_closer_offset` / :func:`word_end_position` instead.  When the
    closer falls on the next line (a multi-line braced body whose last inner
    character is a newline), the line/column advance with the offset.
    """
    if tok.type in (TokenType.STR, TokenType.CMD):
        span_extra = (tok.end.offset - tok.start.offset) - len(tok.text)
        if span_extra >= 1:
            # Empty ``{}`` / ``[]``: ``end`` already sits on the closer.
            return Range(start=tok.start, end=tok.end)
        last_inner = tok.text[-1] if tok.text else ""
        return Range(start=tok.start, end=_closer_position(tok.end, last_inner))
    return Range(start=tok.start, end=tok.end)


def range_from_tokens(tokens: list[Token]) -> Range:
    """Build a Range spanning from the first to the last token."""
    return Range(start=tokens[0].start, end=tokens[-1].end)


_RANGE_CLOSERS = {'"': '"', "{": "}", "[": "]"}


def word_closer_offset(tok: Token, source: str, *, base_offset: int = 0) -> int | None:
    """Offset of *tok*'s closing ``}`` / ``]`` / ``"`` in *source*, or ``None``.

    The authoritative way to locate a delimited word's closing delimiter for a
    caller that needs to *slice source* (e.g. extracting the raw argument text of
    a refactor edit).  It must be called rather than re-deriving the position as
    ``tok.end.offset + 1``: the lexer stores word ends in the *inner-end*
    convention (``end`` is the last inner character, closer one past it) **except**
    for an empty ``{}`` / ``[]`` / ``""``, whose ``end`` already sits *on* the
    closer.  ``tok.text`` is the discriminator — an empty word has none — so the
    closer is at ``end`` when the word is empty and ``end + 1`` otherwise.
    Deriving emptiness from ``tok.text`` keeps this correct for quoted words
    whose inner text contains backslash escapes, and for a non-empty word whose
    last inner character is itself a closer (``{a {b}}``).

    Returns ``None`` when *tok* does not begin with an opening delimiter, or when
    the word is unterminated (the computed position is not the matching closer).
    *tok* and *source* must share a coordinate frame: by default *source* is the
    document and the returned offset is absolute.  Pass *base_offset* (the
    absolute start of *source*) when *source* is a region *substring* — both the
    opener and closer indices are shifted by it and the result is region-relative.
    (A ``base_offset`` was removed in ``4092ab3`` when ``cmd.range`` went
    token-only; this re-adds it for the internal terminated checks and the
    switch-case-list raw-span slicer, which genuinely work in a substring frame.)
    Command/word *ranges* are owned by the concrete syntax tree the segmenter
    builds and do not use this — see :mod:`compiler.parsing.syntax`.
    """
    start = tok.start.offset - base_offset
    if not (0 <= start < len(source)):
        return None
    closer = _RANGE_CLOSERS.get(source[start])
    if closer is None:
        return None
    closer_off = (tok.end.offset if not tok.text else tok.end.offset + 1) - base_offset
    if 0 <= closer_off < len(source) and source[closer_off] == closer:
        return closer_off
    return None


def closer_present_in_region(tok: Token, text: str, base_offset: int = 0) -> bool:
    """Whether *tok*'s closing ``}`` / ``]`` / ``"`` is present in *text*.

    The boolean, region-relative sibling of :func:`word_closer_offset` for the
    green-tree descent's terminated checks: *text* is the region containing *tok*
    and *base_offset* its absolute anchor (``0`` when *text* is the document).
    A delimited word is *terminated* exactly when its matching closer is present
    one byte past its inner content, which is what :func:`word_closer_offset`
    locates — so the two stay in lockstep by construction.
    """
    return word_closer_offset(tok, text, base_offset=base_offset) is not None


def word_end_position(tok: Token, source: str) -> SourcePosition:
    """Inclusive end :class:`SourcePosition` covering *tok*'s closing delimiter.

    The position-returning sibling of :func:`word_closer_offset`, for callers
    that build a :class:`Range`/LSP position rather than slice by offset.  For a
    delimited word it returns the position *of* the closing ``}`` / ``]`` / ``"``
    (with line/column advanced when the closer falls on the next line); for an
    empty ``{}`` / ``[]`` / ``""`` the inclusive end already *is* the closer, so
    ``tok.end`` is returned unchanged.  For any non-delimited or unterminated
    token it returns ``tok.end`` unchanged.  Unlike :func:`range_from_word_token`
    this also covers quoted ``"..."`` words, whose opener lives only in *source*.
    """
    closer_off = word_closer_offset(tok, source)
    if closer_off is None or closer_off == tok.end.offset:
        return tok.end
    last_inner = source[tok.end.offset] if 0 <= tok.end.offset < len(source) else ""
    return _closer_position(tok.end, last_inner)


def widen_range_for_closer(source: str, range_: Range) -> Range:
    """Extend *range_* by one character to include a closing delimiter.

    The lexer's token end omits the closing ``}`` / ``"`` / ``]`` of a
    braced/quoted/bracketed word, so a range built from such a token stops one
    character short.  When *range_* opens with one of those delimiters and the
    matching closer immediately follows its (inclusive) end, return a range
    extended to cover the closer; otherwise return *range_* unchanged.  When the
    closer falls on the next line (a multi-line ``{ ... \\n}``), the line/column
    of the extended end advance with the offset so the two stay consistent.
    """
    start_off = range_.start.offset
    if not (0 <= start_off < len(source)):
        return range_
    closer = _RANGE_CLOSERS.get(source[start_off])
    end = range_.end
    # An empty ``{}`` / ``[]`` / ``""`` already ends *on* its closer — the range
    # spans exactly the opener and the closer — so there is nothing to widen.
    # Advancing would overshoot into whatever follows; for a trailing ``{}`` that
    # is the enclosing body's closing brace (issue #527).  This 2-character span
    # is unambiguous: any non-empty word is longer, so a deeper nested closer
    # (``{a {b}`` ending on an inner ``}``) never matches ``start_off + 1``.
    if (
        closer
        and end.offset == start_off + 1
        and end.offset < len(source)
        and source[end.offset] == closer
    ):
        return range_
    if closer and end.offset + 1 < len(source) and source[end.offset + 1] == closer:
        last_inner = source[end.offset] if 0 <= end.offset < len(source) else ""
        return Range(start=range_.start, end=_closer_position(end, last_inner))
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
