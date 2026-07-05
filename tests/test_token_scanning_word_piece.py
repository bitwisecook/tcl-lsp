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

"""Golden table for the canonical ``token_scanning.word_piece``.

After the CST landed (#533), ``word_piece`` carries the braced-vs-bare
distinction by *natural spelling* (no ``$={}`` marker).  It was still
triplicated across ``token_helpers`` (natural-spelling), ``command_segmenter``
(substituted-bare-array stays bare, literal normalises), and the verbatim-bare
shape matcher.  This pins that the one two-flag function reproduces each of
those three behaviours exactly, so a future change to the shared helper can't
silently shift any consumer.
"""

from __future__ import annotations

from compiler.parsing.lexer import TclLexer
from compiler.parsing.token_scanning import word_piece
from shared.tokens import TokenType

# The three original implementations, reproduced verbatim as oracles.


def _old_token_helpers(tok):
    if tok.type is TokenType.VAR:
        is_braced = (tok.end.offset - tok.start.offset) - len(tok.text) >= 1
        if is_braced:
            if "}" in tok.text:
                return "$" + tok.text
            return f"${{{tok.text}}}"
        if "(" in tok.text and tok.text.endswith(")"):
            return "$" + tok.text
        if "}" in tok.text:
            return "$" + tok.text
        return f"${{{tok.text}}}"
    if tok.type is TokenType.CMD:
        return f"[{tok.text}]"
    return tok.text


def _old_segmenter(tok):
    if tok.type is TokenType.VAR:
        is_braced = (tok.end.offset - tok.start.offset) - len(tok.text) >= 1
        if (
            not is_braced
            and "(" in tok.text
            and tok.text.endswith(")")
            and ("$" in tok.text or "[" in tok.text)
        ):
            return "$" + tok.text
        if "}" in tok.text:
            return "$" + tok.text
        return f"${{{tok.text}}}"
    if tok.type is TokenType.CMD:
        return f"[{tok.text}]"
    return tok.text


def _old_shapes(tok):
    if tok.type is TokenType.VAR:
        is_braced = (tok.end.offset - tok.start.offset) > len(tok.text)
        if "}" in tok.text:
            return "$" + tok.text
        if is_braced:
            return f"${{{tok.text}}}"
        return "$" + tok.text
    if tok.type is TokenType.CMD:
        return f"[{tok.text}]"
    return tok.text


_INPUTS = [
    "$a",
    "${a}",
    "$abc",
    "${abc}",
    "$a(1)",
    "${a(1)}",
    "$a($i)",
    "$a([f])",
    "${a(1[expr {3 - 1}])}",
    "$ns::v",
    "${ns::v}",
    "[set x]",
    "foo",
    "$x:y",
    "$arr(a,b)",
    "${arr(a,b)}",
    r"$a(\$lit)",
    "x$y",
    "a[b]c",
]


def _tokens():
    for src in _INPUTS:
        lx = TclLexer(src)
        while (tok := lx.get_token()) is not None:
            if tok.type in (TokenType.SEP, TokenType.EOL, TokenType.COMMENT):
                continue
            yield src, tok


def test_word_piece_default_matches_token_helpers():
    # token_helpers (natural spelling): bare arrays stay bare for the split path.
    for src, tok in _tokens():
        assert word_piece(tok) == _old_token_helpers(tok), src


def test_word_piece_segmenter_form_matches_command_segmenter():
    for src, tok in _tokens():
        assert word_piece(tok, bare_arrays_split=False) == _old_segmenter(tok), src


def test_word_piece_verbatim_matches_shape_matcher():
    for src, tok in _tokens():
        assert word_piece(tok, normalise_var_braces=False) == _old_shapes(tok), src


def test_bare_vs_braced_array_natural_spelling():
    # Bare $a(1) stays bare (codegen splits to an array load); braced ${a(1)}
    # round-trips verbatim (loaded by whole name).  No $={} marker.
    bare = TclLexer("$a(1)").get_token()
    braced = TclLexer("${a(1)}").get_token()
    assert bare is not None and braced is not None
    assert word_piece(bare) == "$a(1)"
    assert word_piece(braced) == "${a(1)}"
