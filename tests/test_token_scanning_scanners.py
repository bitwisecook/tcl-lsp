"""Tests for the green-tree-backed scanners in ``token_scanning``.

These subsume per-consumer raw-lexer loops (var-name extraction, ``[...]``
command-sub scanning, single-command parsing, expr-argument extraction, region
word iteration).  The behaviour must match the hand-rolled loops they replace.
"""

from __future__ import annotations

import pytest

from compiler.parsing import token_scanning as ts
from compiler.parsing.lexer import is_simple_scalar_var_word
from shared.tokens import TokenType


@pytest.mark.parametrize(
    "text,expected",
    [
        ("$abc", "abc"),
        ("${abc}", "abc"),
        ("$ns::v", "ns::v"),
        ("${ns::v}", "ns::v"),
        ("$a(1)", None),  # array element — not a scalar
        ("${a(1)}", None),  # array-shaped name fails the conventional-shape check
        ("$1bad", None),  # must start letter/underscore
        ("x$y", None),  # whole word must be one var ref
        ("[set x]", None),
        ("", None),
    ],
)
def test_extract_scalar_var_name(text, expected):
    assert ts.extract_scalar_var_name(text) == expected


def test_is_simple_scalar_var_word_delegates():
    # The lexer entry point now delegates to extract_scalar_var_name.
    for text in ["$abc", "${abc}", "$a(1)", "x$y", ""]:
        assert is_simple_scalar_var_word(text) == (ts.extract_scalar_var_name(text) is not None)


def test_scan_command_substitutions():
    toks = ts.scan_command_substitutions("a[b]c[d e]f")
    assert [t.type for t in toks] == [TokenType.CMD, TokenType.CMD]
    assert [t.text for t in toks] == ["b", "d e"]
    assert ts.scan_command_substitutions("no subs here") == []


def test_scan_command_substitutions_absolute_offsets():
    # Positions are anchored at the supplied base.
    toks = ts.scan_command_substitutions("[x]", base_offset=100, base_line=5, base_col=10)
    assert len(toks) == 1
    assert toks[0].start.offset == 100  # the CMD token anchors at the '[' = base 100


def test_single_command_substitution():
    tok = ts.single_command_substitution("[set x]")
    assert tok is not None and tok.text == "set x"
    assert ts.single_command_substitution("  [set x]  ") is None  # leading literal
    assert ts.single_command_substitution("[a][b]") is None  # two subs
    assert ts.single_command_substitution("{[set x]}") is None  # braced literal, STR not CMD
    assert ts.single_command_substitution("plain") is None


def test_parse_single_command():
    parsed = ts.parse_single_command("set x [foo $y]")
    assert parsed is not None
    texts, tokens, single = parsed
    # CMD token text is verbatim — the nested ``$y`` is NOT re-normalised.
    assert texts == ["set", "x", "[foo $y]"]
    assert single == [True, True, True]
    assert ts.parse_single_command("a; b") is None  # two commands
    assert ts.parse_single_command("") is None


@pytest.mark.parametrize(
    "cmd_text,expected",
    [
        ("expr {1 + 2}", "1 + 2"),
        ("expr $x", "$x"),  # verbatim — not normalised to ${x}
        ("expr {1};expr {2}", None),  # multi-command
        ("set x 1", None),  # not expr
        ("expr a b", None),  # arg not single-token
    ],
)
def test_extract_single_expr_argument(cmd_text, expected):
    assert ts.extract_single_expr_argument(cmd_text) == expected


def test_extract_single_expr_argument_aliases():
    aliases = frozenset({"::tcl::mathfunc::e"})
    assert ts.extract_single_expr_argument("::tcl::mathfunc::e {1}", expr_aliases=aliases) == "1"


def test_iter_region_words():
    words = list(ts.iter_region_words("pat1 {body 1} pat2 {body 2}"))
    texts = [w for _, w, _ in words]
    assert texts == ["pat1", "body 1", "pat2", "body 2"]
    # every element is flagged as a word start (all separated by spaces)
    assert all(start for _, _, start in words)
