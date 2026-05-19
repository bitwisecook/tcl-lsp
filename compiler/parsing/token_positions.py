"""Shared token-content offset helpers."""

from __future__ import annotations

from analyser.semantic_model import Range

from .tokens import SourcePosition, Token, TokenType


def token_content_shift(token: Token) -> int:
    """Return delimiter shift (0 or 1) for token content start."""
    return 1 if (token.end.offset - token.start.offset + 1) > len(token.text) else 0


def classify_quoted_contexts(all_tokens: list[Token]) -> list[bool]:
    """Return a list parallel to *all_tokens* marking tokens in quoted words.

    A token is "in a quoted word" when the source character sequence it
    represents was enclosed by double quotes — for example the ``}`` ESC in
    ``append result "}"`` or the ``}`` ESC in ``puts "[cmd]}"``.  Consumers
    that treat bare ``}`` or ``]`` ESC tokens as stray punctuation must skip
    these quoted-context tokens because they represent ordinary string
    characters.

    The lexer already exposes enough information to determine quoted
    context, but not via a single field:

    - The ``Token.in_quote`` flag is ``True`` for tokens parsed while the
      lexer is still "inside" an unclosed quote — it tracks the open quote
      *across* tokens that come between substitutions (VAR/CMD) and the
      closing ``"``.  It is **False** on the token that consumes the
      closing ``"`` because the lexer resets ``insidequote`` before
      returning that token.
    - For a single-token quoted ESC (e.g. ``"}"``), the token's source span
      includes the leading ``"`` but excludes the trailing ``"``, so
      ``token_content_shift`` returns 1.

    Combining both signals lets us recognise every quoted ESC:

    * Self-contained ``"}"`` — ESC with ``shift > 0`` and ``in_quote=False``.
    * Leading part of ``"foo$x"`` — ESC with ``in_quote=True``.
    * Trailing part of ``"[cmd]}"`` — ESC with ``shift=0`` but preceded by
      a CMD/VAR token whose ``in_quote`` is ``True``.

    Separator tokens (``SEP``/``EOL``) can only appear *between* Tcl words,
    so they always reset the tracker.
    """
    result = [False] * len(all_tokens)
    prev_in_quote = False
    for i, tok in enumerate(all_tokens):
        if tok.type is TokenType.SEP or tok.type is TokenType.EOL:
            prev_in_quote = False
            continue
        self_opens = tok.type is TokenType.ESC and token_content_shift(tok) > 0
        result[i] = prev_in_quote or self_opens
        prev_in_quote = tok.in_quote
    return result


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
