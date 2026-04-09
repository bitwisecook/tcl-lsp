"""Pytest contract for `core.parsing.tokens` (`TokenType`, `SourcePosition`, `Token`).

These tests pin the Python-visible API surface that 200+ call sites in
the codebase rely on. They run against whichever implementation
``core.parsing.tokens`` resolves to — Rust-backed (``tcl_lsp_rust``) by
default, pure-Python fallback otherwise. Adding a test here means the
contract holds for both implementations during the Python-to-Rust
rewrite.

Existing tests in ``tests/test_lexer.py``, ``tests/test_token_positions.py``,
``tests/test_incremental_update.py``, ``tests/test_formatter.py``, and the
broader lexer/compiler suite already exercise the types indirectly; this
file consolidates the direct-construction and identity-comparison
checks into one place so a regression in the data layer fails fast.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.parsing.tokens import SourcePosition, Token, TokenType


class TestTokenType:
    def test_all_variants_exist(self):
        names = {
            "ESC",
            "STR",
            "CMD",
            "VAR",
            "SEP",
            "EOL",
            "EOF",
            "COMMENT",
            "EXPAND",
        }
        for name in names:
            assert getattr(TokenType, name).name == name

    def test_class_attributes_are_singletons(self):
        # 200+ call sites use `tok.type is TokenType.X`. Class attribute
        # lookups must return the same Python object on every access.
        assert TokenType.ESC is TokenType.ESC
        assert TokenType.STR is TokenType.STR
        assert TokenType.EOF is TokenType.EOF

    def test_distinct_variants_are_not_identical(self):
        assert TokenType.ESC is not TokenType.STR
        assert TokenType.SEP is not TokenType.EOL

    def test_equality(self):
        assert TokenType.ESC == TokenType.ESC
        assert TokenType.ESC != TokenType.STR

    def test_name_matches_attribute(self):
        assert TokenType.ESC.name == "ESC"
        assert TokenType.COMMENT.name == "COMMENT"
        assert TokenType.EXPAND.name == "EXPAND"

    def test_hashable_and_set_membership(self):
        seen = {TokenType.ESC, TokenType.STR, TokenType.ESC}
        assert seen == {TokenType.ESC, TokenType.STR}
        assert TokenType.ESC in seen
        assert TokenType.VAR not in seen

    def test_value_pattern_match(self):
        # `match`/`case` on `tok.type` is used in core/minifier and
        # vm/substitution; it relies on equality, not identity.
        kind = TokenType.VAR
        match kind:
            case TokenType.VAR:
                label = "var"
            case _:
                label = "other"
        assert label == "var"


class TestSourcePosition:
    def test_construction_with_keywords(self):
        pos = SourcePosition(line=5, character=10, offset=100)
        assert pos.line == 5
        assert pos.character == 10
        assert pos.offset == 100

    def test_construction_positional(self):
        pos = SourcePosition(0, 3, 7)
        assert pos.line == 0
        assert pos.character == 3
        assert pos.offset == 7

    def test_equality(self):
        a = SourcePosition(line=1, character=2, offset=3)
        b = SourcePosition(line=1, character=2, offset=3)
        c = SourcePosition(line=1, character=2, offset=4)
        assert a == b
        assert a != c

    def test_hashable(self):
        a = SourcePosition(0, 0, 0)
        b = SourcePosition(0, 0, 0)
        seen = {a, b, SourcePosition(1, 0, 0)}
        assert len(seen) == 2

    def test_immutable(self):
        pos = SourcePosition(0, 0, 0)
        with pytest.raises((AttributeError, TypeError)):
            pos.line = 5  # type: ignore[misc]


class TestToken:
    def _pos(self, offset: int = 0) -> SourcePosition:
        return SourcePosition(line=0, character=offset, offset=offset)

    def test_construction_with_keywords(self):
        start, end = self._pos(0), self._pos(5)
        tok = Token(type=TokenType.ESC, text="hello", start=start, end=end)
        assert tok.type is TokenType.ESC
        assert tok.text == "hello"
        assert tok.start == start
        assert tok.end == end
        assert tok.in_quote is False

    def test_construction_positional(self):
        start, end = self._pos(0), self._pos(3)
        tok = Token(TokenType.VAR, "abc", start, end)
        assert tok.type is TokenType.VAR
        assert tok.text == "abc"
        assert tok.in_quote is False

    def test_in_quote_default_is_false(self):
        pos = self._pos()
        tok = Token(type=TokenType.ESC, text="x", start=pos, end=pos)
        assert tok.in_quote is False

    def test_in_quote_true_propagates(self):
        pos = self._pos()
        tok = Token(type=TokenType.ESC, text="x", start=pos, end=pos, in_quote=True)
        assert tok.in_quote is True

    def test_type_getter_returns_singleton(self):
        # The Python lexer and many compiler passes rely on
        # `tok.type is TokenType.X` rather than `tok.type == TokenType.X`.
        # The Rust-backed getter must resolve through the class
        # attribute so identity is preserved.
        pos = self._pos()
        tok = Token(type=TokenType.ESC, text="x", start=pos, end=pos)
        assert tok.type is TokenType.ESC

    def test_type_passthrough_preserves_identity(self):
        # Construction patterns like
        # `Token(type=other.type, ...)` must keep singleton identity
        # through one round-trip out and back into the wrapper.
        pos = self._pos()
        original = Token(type=TokenType.STR, text="x", start=pos, end=pos)
        copy = Token(type=original.type, text="y", start=pos, end=pos)
        assert copy.type is TokenType.STR
        assert original.type is copy.type

    def test_equality_compares_all_fields(self):
        pos = self._pos()
        baseline = Token(type=TokenType.ESC, text="x", start=pos, end=pos)
        same = Token(type=TokenType.ESC, text="x", start=pos, end=pos)
        diff_text = Token(type=TokenType.ESC, text="y", start=pos, end=pos)
        diff_kind = Token(type=TokenType.STR, text="x", start=pos, end=pos)
        diff_quote = Token(type=TokenType.ESC, text="x", start=pos, end=pos, in_quote=True)
        assert baseline == same
        assert baseline != diff_text
        assert baseline != diff_kind
        assert baseline != diff_quote

    def test_hashable(self):
        pos = self._pos()
        a = Token(type=TokenType.ESC, text="x", start=pos, end=pos)
        b = Token(type=TokenType.ESC, text="x", start=pos, end=pos)
        c = Token(type=TokenType.ESC, text="x", start=pos, end=pos, in_quote=True)
        seen = {a, b, c}
        assert len(seen) == 2

    def test_immutable(self):
        pos = self._pos()
        tok = Token(type=TokenType.ESC, text="x", start=pos, end=pos)
        with pytest.raises((AttributeError, TypeError)):
            tok.text = "changed"  # type: ignore[misc]


class TestLexerIntegration:
    """Sanity-check that the Python lexer still produces tokens that
    work with the Rust-backed types."""

    def test_lexer_produces_singleton_typed_tokens(self):
        from core.parsing.lexer import TclLexer

        toks = TclLexer("puts hello").tokenise_all()
        words = [t for t in toks if t.type not in (TokenType.SEP, TokenType.EOL)]
        assert len(words) == 2
        assert words[0].type is TokenType.ESC
        assert words[0].text == "puts"
        assert words[1].type is TokenType.ESC
        assert words[1].text == "hello"

    def test_match_on_lexer_token_type(self):
        from core.parsing.lexer import TclLexer

        toks = TclLexer("$var").tokenise_all()
        var = next(t for t in toks if t.type is TokenType.VAR)
        match var.type:
            case TokenType.VAR:
                ok = True
            case _:
                ok = False
        assert ok
