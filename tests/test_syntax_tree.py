"""Tests for the canonical red-green concrete syntax tree.

These validate the tree against the lexer (the source of truth) — every byte is
preserved (losslessness) and every fragment resolves to the exact position the
lexer reports (position-equivalence) — plus the structural model and the
byte-identical ``SegmentedCommand`` derivation.
"""

from __future__ import annotations

import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest

from compiler.parsing.green_tree import tokenise
from compiler.parsing.syntax import (
    SyntaxKind,
    SyntaxTree,
    TriviaKind,
    build_document,
    segments_from_document,
)
from compiler.parsing.syntax.green import GreenNode, GreenToken
from compiler.parsing.syntax.red import SyntaxNode
from shared.tokens import TokenType

_TRIVIA = (TokenType.SEP, TokenType.EOL, TokenType.COMMENT)

EDGE_CASES = [
    "set a {abc}",
    "set a {}",
    "set a [x]",
    "set a []",
    'set a "q"',
    'set a "q$x b"',
    "set a {a}b",
    "set a $x",
    "set a ${x}",
    "set a $arr(i)",
    "# comment\nset a 1",
    "puts hi ;# trail",
    "set a {\n multi\n line\n}",
    "{*}$args",
    "a;b",
    "lappend x {*}$args y",
    "#c1\n#c2\nproc p {} {}",
    "#c1\n\n#c2\nset a 1",
    "set a 1   \nset b 2",
    "foo \\\n  bar baz",
    "{a}{*}\\\n",  # word + dangling {*}
    "{*}\\\n ;\nset a 1",  # dangling-marker-only command, then a real one
    "",
    "\n\n\n",
    "if {$x > 1} {puts hi} else {puts bye}",
    "set x 1;set y 2;set z 3",
]


def _gen(rng: random.Random) -> str:
    words = [
        "set",
        "x",
        "{a}",
        "{}",
        "{a b}",
        "[cmd]",
        "[]",
        '"q"',
        '"a$x b"',
        "$v",
        "${v}",
        "$a(i)",
        "{*}",
        "a{b}c",
        "}",
        "1",
        "\\\n",
        "{a\nb}",
    ]
    seps = [" ", "  ", "\t", ";", "\n", "\n\n", " ;", "\t;\t"]
    comments = ["# c", "#", "# a b c"]
    out = []
    for _ in range(rng.randint(0, 12)):
        r = rng.random()
        if r < 0.6:
            out.append(rng.choice(words))
        elif r < 0.9:
            out.append(rng.choice(seps))
        else:
            out.append("\n" + rng.choice(comments) + "\n")
    return "".join(out)


def _commands(doc: GreenNode) -> list[GreenNode]:
    return [c for c in doc.children if isinstance(c, GreenNode)]


def _words(cmd: GreenNode) -> list[GreenNode]:
    return [c for c in cmd.children if isinstance(c, GreenNode) and c.kind is SyntaxKind.WORD]


def _frags(word: GreenNode) -> list[GreenToken]:
    return [c for c in word.children if isinstance(c, GreenToken)]


def _red_fragment_tokens(tree: SyntaxTree):
    out = []
    for cmd in tree.root.children():
        if isinstance(cmd, SyntaxNode):
            out.extend(st.to_token() for st in cmd.tokens())
    return out


class TestLosslessness:
    @pytest.mark.parametrize("src", EDGE_CASES)
    def test_round_trip_edge_cases(self, src):
        doc, _ = build_document(src)
        assert doc.full_text == src

    def test_round_trip_random(self):
        rng = random.Random(20260603)
        for _ in range(3000):
            src = _gen(rng)
            doc, _ = build_document(src)
            assert doc.full_text == src, repr(src)


class TestPositionEquivalence:
    @pytest.mark.parametrize("src", EDGE_CASES)
    def test_fragments_match_lexer(self, src):
        doc, _ = build_document(src)
        red = _red_fragment_tokens(SyntaxTree(doc, text=src))
        lex = [t for t in tokenise(src, 0, 0, 0)[0] if t.type not in _TRIVIA]
        assert red == lex

    def test_random_fragments_match_lexer(self):
        rng = random.Random(11)
        for _ in range(3000):
            src = _gen(rng)
            doc, _ = build_document(src)
            red = _red_fragment_tokens(SyntaxTree(doc, text=src))
            lex = [t for t in tokenise(src, 0, 0, 0)[0] if t.type not in _TRIVIA]
            assert red == lex, repr(src)

    def test_multiline_braced_word_end_is_closer(self):
        src = "set a {\n x\n}"
        doc, _ = build_document(src)
        # The command range end is the closing brace, on its own line.
        seg = segments_from_document(doc, text=src)[0]
        assert src[seg.range.end.offset] == "}"
        assert seg.range.end.line == 2


class TestStructuralModel:
    def test_braced_word_owns_delimiters_in_raw(self):
        doc, _ = build_document("set a {a b}")
        frag = _frags(_words(_commands(doc)[0])[-1])[0]
        assert frag.token_type is TokenType.STR
        assert frag.raw == "{a b}"  # full span, delimiters included
        assert frag.text == "a b"  # inner text, delimiters excluded

    def test_compound_word_groups_fragments(self):
        doc, _ = build_document("set a {x}y$z")
        word = _words(_commands(doc)[0])[-1]
        assert [f.token_type for f in _frags(word)] == [
            TokenType.STR,
            TokenType.ESC,
            TokenType.VAR,
        ]

    def test_empty_delimiters_round_trip(self):
        for src, raw in [("set a {}", "{}"), ("set a []", "[]")]:
            doc, _ = build_document(src)
            word = _words(_commands(doc)[0])[-1]
            assert _frags(word)[0].raw == raw

    def test_expand_marker_not_a_fragment(self):
        doc, _ = build_document("{*}$args")
        word = _words(_commands(doc)[0])[0]
        assert word.is_expand
        assert len(word.expand_markers) == 1
        assert [f.token_type for f in _frags(word)] == [TokenType.VAR]

    def test_double_expand_markers(self):
        doc, _ = build_document("{*}{*}x")
        word = _words(_commands(doc)[0])[0]
        assert len(word.expand_markers) == 2

    def test_dangling_marker_command_has_no_word(self):
        # `{*}\<nl>` then a real command: the marker-only command has no WORD.
        doc, _ = build_document("{*}\\\n ;\nset a 1")
        has_word = [bool(_words(cmd)) for cmd in _commands(doc)]
        assert has_word.count(False) >= 1  # the dangling-marker command
        assert has_word.count(True) == 1  # `set a 1`


class TestTriviaAttachment:
    def test_leading_whitespace_is_trivia(self):
        doc, _ = build_document("  set a 1")
        first = _frags(_words(_commands(doc)[0])[0])[0]
        assert first.leading[0].kind is TriviaKind.WHITESPACE
        assert first.leading[0].text == "  "

    def test_preceding_comment_captured(self):
        doc, _ = build_document("# hello\n# world\nset a 1")
        assert _commands(doc)[0].preceding_comment == "hello\nworld"

    def test_blank_line_resets_comment(self):
        doc, _ = build_document("# hello\n\nset a 1")
        assert _commands(doc)[0].preceding_comment is None

    def test_trailing_comment_dangles_on_document(self):
        src = "puts hi ;# bye"
        doc, _ = build_document(src)
        # Round-trips even though the comment attaches to no command.
        assert doc.full_text == src
        seg = segments_from_document(doc, text=src)
        assert len(seg) == 1
        assert seg[0].preceding_comment is None


class TestRedLayer:
    def test_lazy_absolute_positions(self):
        src = "set a 1\nputs $x"
        doc, _ = build_document(src)
        tree = SyntaxTree(doc, text=src)
        toks = _red_fragment_tokens(tree)
        var = next(t for t in toks if t.type is TokenType.VAR)
        assert var.start.line == 1
        assert src[var.start.offset] == "$"

    def test_base_anchoring(self):
        # A body lexed at a non-zero anchor reports absolute positions.
        inner = "puts $x"
        doc, _ = build_document(inner, 100, 5, 3)
        tree = SyntaxTree(doc, 100, 5, 3, text=inner)
        cmd = next(tree.root.children())
        assert isinstance(cmd, SyntaxNode)
        first = next(cmd.tokens())
        assert first.start_position.offset == 100
        assert first.start_position.line == 5
        assert first.start_position.character == 3


class TestSegmentDerivation:
    def test_simple_command(self):
        seg = segments_from_document(build_document("set a 1")[0], text="set a 1")[0]
        assert seg.texts == ["set", "a", "1"]
        assert seg.single_token_word == [True, True, True]
        assert seg.range.start.offset == 0
        assert seg.range.end.offset == 6

    def test_compound_word_merges_argv(self):
        src = "set a {x}y"
        seg = segments_from_document(build_document(src)[0], text=src)[0]
        assert seg.texts == ["set", "a", "xy"]
        assert seg.single_token_word == [True, True, False]
        # the merged word token spans both fragments
        assert seg.argv[-1].start.offset == 6
        assert seg.argv[-1].end.offset == 9

    def test_expand_word_flagged(self):
        src = "lappend l {*}$args"
        seg = segments_from_document(build_document(src)[0], text=src)[0]
        assert seg.expand_word == [False, False, True]

    def test_semicolon_splits_commands(self):
        src = "a;b;c"
        segs = segments_from_document(build_document(src)[0], text=src)
        assert [s.name for s in segs] == ["a", "b", "c"]
