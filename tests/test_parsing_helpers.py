"""Tests for shared parsing helpers introduced by utility lifts."""

from __future__ import annotations

from core.commands.registry import REGISTRY
from core.parsing.argv import widen_argv_tokens_to_word_spans
from core.parsing.command_shapes import extract_single_expr_argument
from core.parsing.known_commands import known_command_names
from core.parsing.lexer import TclLexer
from core.parsing.tokens import Token, TokenType


def _first_command_tokens(source: str) -> tuple[list[Token], list[Token]]:
    lexer = TclLexer(source)
    argv: list[Token] = []
    all_tokens: list[Token] = []
    prev_type = TokenType.EOL

    for tok in lexer.tokenise_all():
        if tok.type in (TokenType.COMMENT,):
            continue
        if tok.type is TokenType.SEP:
            prev_type = tok.type
            continue
        if tok.type in (TokenType.EOL, TokenType.EOF):
            break

        all_tokens.append(tok)
        if prev_type in (TokenType.SEP, TokenType.EOL):
            argv.append(tok)
        prev_type = tok.type

    return argv, all_tokens


def test_known_command_names_is_cached_registry_snapshot() -> None:
    names = known_command_names()
    assert isinstance(names, frozenset)
    assert names is known_command_names()
    assert names == frozenset(REGISTRY.command_names())
    assert "set" in names


def test_widen_argv_tokens_to_word_spans_expands_multitoken_words() -> None:
    argv, all_tokens = _first_command_tokens("set a foo$bar")
    assert argv[2].start.offset == 6
    assert argv[2].end.offset == 8  # raw first-token end ("foo")

    widened = widen_argv_tokens_to_word_spans(argv, all_tokens)
    assert widened[2].start.offset == 6
    assert widened[2].end.offset == 12  # full-word end ("foo$bar")


def test_extract_single_expr_argument_cases() -> None:
    assert extract_single_expr_argument("expr {$a+1}") == "$a+1"
    assert extract_single_expr_argument("expr $x") == "$x"
    assert extract_single_expr_argument("expr [llength $xs]") == "[llength $xs]"
    assert extract_single_expr_argument("set x 1") is None
    assert extract_single_expr_argument("expr $x + 1") is None
