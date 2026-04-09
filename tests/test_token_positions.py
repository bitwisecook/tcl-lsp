"""Tests for shared token-content position helpers."""

from __future__ import annotations

from core.parsing.lexer import TclLexer
from core.parsing.token_positions import (
    classify_quoted_contexts,
    token_content_base,
    token_content_shift,
)
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


def _classify(source: str) -> list[tuple[str, str, bool]]:
    """Return (token_type, token_text, in_quoted) triples for a source string."""
    toks = TclLexer(source).tokenise_all()
    flags = classify_quoted_contexts(toks)
    return [(t.type.name, t.text, f) for t, f in zip(toks, flags)]


def test_classify_quoted_contexts_self_contained_quoted_close_brace() -> None:
    # "}" is a self-contained quoted ESC — shift=1, in_quote=False.
    info = _classify('append result "}"')
    esc_flags = [(name, text, flag) for name, text, flag in info if name == "ESC"]
    # The bare "}" ESC must be marked as quoted, the others must not.
    assert esc_flags == [
        ("ESC", "append", False),
        ("ESC", "result", False),
        ("ESC", "}", True),
    ]


def test_classify_quoted_contexts_continuing_quoted_word_after_cmd() -> None:
    # "[cmd]}" — the trailing '}' ESC has shift=0 but follows a CMD with
    # in_quote=True, so it must be marked quoted.
    info = _classify('puts "[set x 1]}"')
    # Only interesting tokens: the trailing ESC '}' should be quoted.
    names_and_quoted = [(text, flag) for name, text, flag in info if name in ("ESC", "CMD")]
    assert ("}", True) in names_and_quoted


def test_classify_quoted_contexts_continuing_quoted_word_after_var() -> None:
    info = _classify('puts "$x}"')
    names_and_quoted = [(text, flag) for name, text, flag in info if name in ("ESC", "VAR")]
    assert ("}", True) in names_and_quoted


def test_classify_quoted_contexts_actual_stray_brace() -> None:
    # Bare '}' ESC at top level must NOT be marked quoted.
    info = _classify("set x 1\n}\n")
    esc_flags = [(text, flag) for name, text, flag in info if name == "ESC"]
    assert ("}", False) in esc_flags


def test_classify_quoted_contexts_sep_eol_reset_state() -> None:
    # After a quoted word closes, the next token must not be marked quoted.
    info = _classify('set x "a"; set y 1')
    esc_flags = [(text, flag) for name, text, flag in info if name == "ESC"]
    # 'y' comes after the quoted "a" plus a SEP/EOL — it must not be quoted.
    assert ("y", False) in esc_flags
    assert ("1", False) in esc_flags
    # And the quoted "a" itself is quoted.
    assert ("a", True) in esc_flags


def test_classify_quoted_contexts_bracket_in_quoted_string() -> None:
    # The ']' ESC inside a quoted string must be marked quoted.
    info = _classify('puts "]"')
    esc_flags = [(text, flag) for name, text, flag in info if name == "ESC"]
    assert ("]", True) in esc_flags


def test_classify_quoted_contexts_mid_quote_after_cmd_subst_bracket() -> None:
    # "[cmd]]" — the trailing ']' ESC follows a CMD in_quote=True.
    info = _classify('puts "[cmd] ]"')
    names_and_quoted = [(text, flag) for name, text, flag in info if name in ("ESC", "CMD")]
    # The ESC containing ' ]' (or just ']') should be quoted.
    has_quoted_bracket = any(
        "]" in text and flag for _text, flag in names_and_quoted for text in [_text]
    )
    assert has_quoted_bracket
