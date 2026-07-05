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

"""Tests for shared parsing helpers introduced by utility lifts."""

from __future__ import annotations

from compiler.parsing.argv import widen_argv_tokens_to_word_spans
from compiler.parsing.known_commands import known_command_names
from compiler.parsing.lexer import TclLexer
from compiler.parsing.token_scanning import extract_single_expr_argument
from compiler.registry import REGISTRY
from shared.tokens import Token, TokenType


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
