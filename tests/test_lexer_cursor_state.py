"""Regression tests for ``TclLexer`` cursor state across ``tokenise_all`` calls.

The lexer's incremental contract is: after ``tokenise_all()`` has drained
the source, the cursor (``self.pos`` and friends) must sit at end-of-source
so that a subsequent ``get_token()`` returns ``None`` (EOF) rather than
re-reading from offset 0.

Today's pure-Python ``tokenise_all`` satisfies this naturally because it
loops over ``get_token()`` until None, and ``get_token()`` advances the
cursor on every call. The contract still needs an explicit guard test so
any future bulk-tokenise fast path (e.g. a Rust-binding shortcut that
bypasses ``get_token``) is forced to finalise the cursor too.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.parsing.lexer import TclLexer
from shared.tokens import TokenType


def _sample_sources() -> list[str]:
    return [
        "",
        "puts hi\n",
        "proc foo {a b} { return [+ $a $b] }\nset x 1\n",
        'set s "hello $world"\n',
        "# leading comment\nputs ok\n",
    ]


@pytest.mark.parametrize("source", _sample_sources())
def test_get_token_after_tokenise_all_returns_none(source: str) -> None:
    lexer = TclLexer(source)
    lexer.tokenise_all()

    # The lexer must NOT re-read from offset 0 — repeated calls return None.
    assert lexer.get_token() is None
    assert lexer.get_token() is None


@pytest.mark.parametrize("source", _sample_sources())
def test_cursor_at_end_after_tokenise_all(source: str) -> None:
    lexer = TclLexer(source)
    lexer.tokenise_all()

    assert lexer.pos == len(source)
    assert lexer._type is TokenType.EOF


def test_tokenise_all_token_count_matches_get_token_loop() -> None:
    src = "proc foo {a b} { return [+ $a $b] }\nset x 1\n"
    bulk = TclLexer(src).tokenise_all()

    incremental = []
    lex2 = TclLexer(src)
    while True:
        tok = lex2.get_token()
        if tok is None:
            break
        incremental.append(tok)

    assert len(bulk) == len(incremental)
    for a, b in zip(bulk, incremental, strict=True):
        assert a.type is b.type
        assert a.text == b.text
        assert a.start == b.start
        assert a.end == b.end


def test_double_tokenise_all_does_not_re_emit_tokens() -> None:
    # A second call to tokenise_all on the same lexer must not re-read from
    # offset 0; the cursor is already at EOF.
    src = "puts ok\n"
    lexer = TclLexer(src)
    first = lexer.tokenise_all()
    second = lexer.tokenise_all()
    assert first  # not empty
    assert second == []
