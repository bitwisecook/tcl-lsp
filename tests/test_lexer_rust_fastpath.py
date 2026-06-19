"""Tests for the Rust lexer fast-path token-type compatibility.

When the ``tcl_lsp_rust`` wheel is installed, ``TclLexer.tokenise_all``
dispatches to the Rust tokeniser, which returns PyO3 ``tcl_lsp_py.Token``
objects.  Those tokens carry a ``PyTokenType`` enum that is equal by value
but NOT identity-equal to ``shared.tokens.TokenType``.  The parser compares
token types with ``is`` (e.g. ``tok.type is TokenType.VAR``), so the fast
path must convert the foreign tokens into native ``shared.tokens.Token``
instances or every such check silently fails.

These tests monkeypatch the fast-path entry point with a fake that mimics
the PyO3 ``Token`` surface (``.type.name`` / ``.text`` /
``.start.line|character|offset`` / ``.end.*`` / ``.in_quote``), so they run
even when the wheel is not built in CI.
"""

from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import compiler.parsing.lexer as lexer_mod
from compiler.parsing.lexer import TclLexer
from shared.tokens import SourcePosition, Token, TokenType


@dataclass(frozen=True)
class _FakePyTokenType:
    """Mimics ``tcl_lsp_py.TokenType`` — only ``.name`` is consumed."""

    name: str


@dataclass(frozen=True)
class _FakePySourcePosition:
    """Mimics ``tcl_lsp_py.SourcePosition``."""

    line: int
    character: int
    offset: int


@dataclass(frozen=True)
class _FakePyToken:
    """Mimics ``tcl_lsp_py.Token`` — distinct from ``shared.tokens.Token``."""

    type: _FakePyTokenType
    text: str
    start: _FakePySourcePosition
    end: _FakePySourcePosition
    in_quote: bool = False


def _fake_rust_tokenise(source, *_args, **_kwargs):
    """Stand-in for ``lexer_tokenise_with_config``.

    Returns one ``$var`` VAR token plus an EOL so the conversion exercises a
    type whose ``is``-identity actually matters to the parser, along with the
    ``in_quote`` flag round-trip.
    """
    tokens = [
        _FakePyToken(
            type=_FakePyTokenType("VAR"),
            text="var",
            start=_FakePySourcePosition(0, 0, 0),
            end=_FakePySourcePosition(0, 3, 3),
            in_quote=True,
        ),
        _FakePyToken(
            type=_FakePyTokenType("EOL"),
            text="",
            start=_FakePySourcePosition(0, 4, 4),
            end=_FakePySourcePosition(0, 4, 4),
            in_quote=False,
        ),
    ]
    warnings: list[tuple[SourcePosition, str]] = []
    return tokens, warnings


@pytest.fixture
def _patched_fastpath(monkeypatch):
    monkeypatch.setattr(lexer_mod, "_rust_lexer_tokenise_cfg", _fake_rust_tokenise)


def test_fastpath_returns_native_tokens(_patched_fastpath) -> None:
    """The fast path must yield ``shared.tokens.Token`` instances."""
    tokens = TclLexer("$var").tokenise_all()
    assert all(isinstance(tok, Token) for tok in tokens)


def test_fastpath_token_type_is_identity_equal(_patched_fastpath) -> None:
    """Converted ``.type`` must be the canonical ``TokenType`` singleton.

    This is the core regression: ``tok.type is TokenType.VAR`` is how the
    parser dispatches, and a raw ``PyTokenType`` would fail that ``is`` check.
    """
    tokens = TclLexer("$var").tokenise_all()
    assert tokens[0].type is TokenType.VAR
    assert tokens[1].type is TokenType.EOL


def test_fastpath_preserves_positions_and_text(_patched_fastpath) -> None:
    """Positions, text, and the quoting flag survive the conversion."""
    tokens = TclLexer("$var").tokenise_all()
    var = tokens[0]
    assert var.text == "var"
    assert var.start == SourcePosition(line=0, character=0, offset=0)
    assert var.end == SourcePosition(line=0, character=3, offset=3)
    assert var.in_quote is True
    assert tokens[1].in_quote is False


def test_fastpath_only_when_no_virtuals(_patched_fastpath) -> None:
    """Virtual insertions force the pure-Python path (fast path skipped).

    With virtuals present the lexer must not call the Rust binding (which
    cannot model them); the returned tokens come from ``get_token`` and are
    therefore native already.
    """
    lexer = TclLexer("$var", virtual_insertions={4: "]"})
    tokens = lexer.tokenise_all()
    assert all(isinstance(tok, Token) for tok in tokens)
    # The fake fast path only ever emits a single VAR+EOL pair; the real
    # Python path emits a VAR token for ``$var``.
    assert any(tok.type is TokenType.VAR for tok in tokens)


def _fake_rust_tokenise_byte_positions(source, *args, **_kwargs):
    """Stand-in that returns *byte* offsets like the real Rust lexer.

    Modelled on the source ``"é$v"`` — ``é`` (U+00E9) is two UTF-8 bytes, so
    the ``$v`` VAR token spans bytes 2..4 (code points 1..3).  The fast path
    must hand this stand-in zero bases for a non-ASCII source (so the remap is
    unambiguous); assert that here.
    """
    # args = (expand, irules_sep, strict, base_offset, base_line, base_col)
    assert args[3:] == (0, 0, 0), "non-ASCII source must request zero-based byte positions"
    tokens = [
        _FakePyToken(
            type=_FakePyTokenType("VAR"),
            text="v",
            start=_FakePySourcePosition(0, 2, 2),
            end=_FakePySourcePosition(0, 4, 4),
        ),
    ]
    warnings = [(_FakePySourcePosition(0, 2, 2), "demo warning")]
    return tokens, warnings


def test_fastpath_remaps_byte_offsets_for_non_ascii(monkeypatch) -> None:
    """Non-ASCII sources: byte offsets/columns from Rust are translated back to
    the Python code-point convention for both tokens and warnings."""
    monkeypatch.setattr(lexer_mod, "_rust_lexer_tokenise_cfg", _fake_rust_tokenise_byte_positions)
    lexer = TclLexer("é$v")
    tokens = lexer.tokenise_all()
    var = tokens[0]
    # Byte offsets 2 / 4 (after the 2-byte 'é') become code points 1 / 3.
    assert var.start == SourcePosition(line=0, character=1, offset=1)
    assert var.end == SourcePosition(line=0, character=3, offset=3)
    # The lexer warning is remapped with the same convention.
    assert lexer.warnings == [(SourcePosition(line=0, character=1, offset=1), "demo warning")]


def test_fastpath_ascii_keeps_verbatim_positions(_patched_fastpath) -> None:
    """ASCII sources keep the verbatim copy fast path (byte == code point), so
    positions are taken straight from the Rust binding."""
    tokens = TclLexer("$var").tokenise_all()
    assert tokens[0].start == SourcePosition(line=0, character=0, offset=0)
    assert tokens[0].end == SourcePosition(line=0, character=3, offset=3)


def test_byte_to_position_map_applies_bases_and_newlines() -> None:
    """The byte→code-point map counts code points (not bytes), tracks newlines,
    and re-applies the sub-lexing bases the way ``TclLexer._pos_at`` does."""
    text = "a\né😀b"  # 'a'(1) '\n'(1) 'é'(2) '😀'(4) 'b'(1)
    offsets = {0, 2, 4, 8, 9}
    m = lexer_mod._byte_to_position_map(text, offsets, base_offset=100, base_line=10, base_col=5)
    # Line-0 positions get base_col; later lines start their column at 0.
    assert m[0] == SourcePosition(line=10, character=5, offset=100)
    assert m[2] == SourcePosition(line=11, character=0, offset=102)  # start of 'é'
    assert m[4] == SourcePosition(line=11, character=1, offset=103)  # start of '😀'
    assert m[8] == SourcePosition(line=11, character=2, offset=104)  # start of 'b'
    assert m[9] == SourcePosition(line=11, character=3, offset=105)  # end of text
