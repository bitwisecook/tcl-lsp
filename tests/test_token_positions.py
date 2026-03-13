"""Tests for shared token-content position helpers."""

from __future__ import annotations

from core.parsing.lexer import TclLexer
from core.parsing.token_positions import token_content_base, token_content_shift
from core.parsing.tokens import Token, TokenType


def _first_token(
    source: str,
    token_type: TokenType,
    *,
    text: str | None = None,
) -> Token:
    for tok in TclLexer(source).tokenise_all():
        if tok.type is token_type and (text is None or tok.text == text):
            return tok
    raise AssertionError(f"Token type {token_type} not found in source: {source!r}")


def test_token_content_shift_for_braced_quoted_and_plain_tokens() -> None:
    braced = _first_token("puts {abc}", TokenType.STR, text="abc")
    quoted = _first_token('puts "abc"', TokenType.ESC, text="abc")
    plain = _first_token("puts abc", TokenType.ESC, text="abc")

    assert token_content_shift(braced) == 1
    assert token_content_shift(quoted) == 1
    assert token_content_shift(plain) == 0


def test_token_content_base_offsets_and_columns() -> None:
    braced = _first_token("puts {abc}", TokenType.STR, text="abc")
    plain = _first_token("puts abc", TokenType.ESC, text="abc")

    assert token_content_base(braced) == (braced.start.offset + 1, 0, braced.start.character + 1)
    assert token_content_base(plain) == (plain.start.offset, 0, plain.start.character)
