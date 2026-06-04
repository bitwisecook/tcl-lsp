"""Golden table for the canonical ``token_scanning.word_piece``.

``word_piece`` was triplicated across ``token_helpers`` (codegen ``$={…}``
marker), ``command_segmenter`` (normalised, no marker), and ``command_shapes``
(verbatim).  They are now one two-flag function; this pins that the flag
combinations reproduce each of the three original behaviours exactly, so a
future change to the shared helper can't silently shift any consumer.
"""

from __future__ import annotations

from compiler.parsing.lexer import TclLexer
from compiler.parsing.token_scanning import BRACED_SCALAR_MARKER_PREFIX as _M
from compiler.parsing.token_scanning import word_piece
from shared.tokens import TokenType

# The three original implementations, reproduced verbatim as the oracle.


def _old_token_helpers(tok):
    if tok.type is TokenType.VAR:
        span_extra = (tok.end.offset - tok.start.offset) - len(tok.text)
        is_braced = span_extra >= 1
        if is_braced and "(" in tok.text and tok.text.endswith(")"):
            return f"{_M}{tok.text}}}"
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


def _old_segmenter(tok):
    if tok.type is TokenType.VAR:
        span_extra = (tok.end.offset - tok.start.offset) - len(tok.text)
        is_braced = span_extra >= 1
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


def test_word_piece_codegen_marker_matches_old_token_helpers():
    for src, tok in _tokens():
        assert word_piece(tok) == _old_token_helpers(tok), src


def test_word_piece_normalised_matches_old_segmenter():
    for src, tok in _tokens():
        assert word_piece(tok, array_codegen_marker=False) == _old_segmenter(tok), src


def test_word_piece_verbatim_matches_old_command_shapes():
    for src, tok in _tokens():
        got = word_piece(tok, array_codegen_marker=False, normalise_var_braces=False)
        assert got == _old_shapes(tok), src


def test_braced_array_scalar_emits_marker_only_with_flag():
    # ``${a(1)}`` is a scalar named ``a(1)`` — the marker form is opt-in.
    lx = TclLexer("${a(1)}")
    tok = lx.get_token()
    assert tok is not None and tok.type is TokenType.VAR and tok.text == "a(1)"
    assert word_piece(tok, array_codegen_marker=True) == f"{_M}a(1)}}"
    assert word_piece(tok, array_codegen_marker=False) == "${a(1)}"
