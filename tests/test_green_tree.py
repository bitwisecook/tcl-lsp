"""Tests for the green token tree: descent, mode tagging, intern sharing."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.parsing.green_tree import (
    Mode,
    NodeKind,
    active_scope,
    green_tree_scope,
    node_for,
    tokenise,
)
from core.parsing.tokens import TokenType


class TestNodeBuilding:
    def test_root_tokens_match_direct_lex(self):
        src = "set a 1\nputs $a\n"
        node = node_for(src)
        types = [t.type for t in node.tokens]
        assert TokenType.VAR in types
        assert node.mode is Mode.SCRIPT
        assert node.kind is NodeKind.ROOT
        assert node.width == len(src)

    def test_positions_are_absolute_at_base(self):
        node = node_for("set a 1", base_offset=100, base_line=5, base_col=3)
        first = node.tokens[0]
        assert first.start.offset == 100
        assert first.start.line == 5
        assert first.start.character == 3


class TestDescent:
    def test_descend_braced_body_anchors_past_brace(self):
        # `proc p {} {set x 1}` — descend the body STR token (no inner padding).
        src = "proc p {} {set x 1}"
        root = node_for(src)
        body = next(t for t in root.tokens if t.type is TokenType.STR and "set x" in t.text)
        child = root.descend(body)
        assert child.kind is NodeKind.BRACED
        assert child.mode is Mode.SCRIPT
        # The child's first real token must be anchored one byte past the `{`.
        first_word = next(t for t in child.tokens if t.type not in (TokenType.SEP, TokenType.EOL))
        assert first_word.start.offset == body.start.offset + 1
        assert src[first_word.start.offset] == "s"  # start of `set`

    def test_descend_is_memoised(self):
        root = node_for("proc p {} { set x 1 }")
        body = next(t for t in root.tokens if t.type is TokenType.STR)
        a = root.descend(body)
        b = root.descend(body)
        assert a is b

    def test_descend_command_substitution(self):
        src = "set y [expr 1]"
        root = node_for(src)
        cmd = next(t for t in root.tokens if t.type is TokenType.CMD)
        child = root.descend(cmd)
        assert child.kind is NodeKind.BRACKETED
        first = next(t for t in child.tokens if t.type not in (TokenType.SEP, TokenType.EOL))
        assert first.start.offset == cmd.start.offset + 1


class TestModeTagging:
    def test_quoted_mode_sets_insidequote(self):
        node = node_for("foo $x", mode=Mode.QUOTED)
        assert node.mode is Mode.QUOTED
        assert node.insidequote is True

    def test_script_mode_default(self):
        node = node_for("foo $x")
        assert node.mode is Mode.SCRIPT
        assert node.insidequote is False


class TestInternSharing:
    def test_same_region_shares_node_within_scope(self):
        src = "set a 1"
        with green_tree_scope():
            n1 = node_for(src)
            n2 = node_for(src)
            assert n1 is n2

    def test_distinct_text_same_base_no_collision(self):
        # Two different bodies both anchored at base 0 (var_refs pattern).
        with green_tree_scope():
            a = node_for("set x 1", 0, 0, 0)
            b = node_for("set y 2", 0, 0, 0)
            assert a is not b
            assert a.text != b.text

    def test_no_scope_builds_fresh_each_time(self):
        # Outside a scope there is no intern index.
        assert active_scope() is None
        n1 = node_for("set a 1")
        n2 = node_for("set a 1")
        assert n1 is not n2

    def test_virtual_insertions_never_interned(self):
        with green_tree_scope():
            a = node_for("set a 1", virtual_insertions={3: "}"})
            b = node_for("set a 1", virtual_insertions={3: "}"})
            assert a is not b

    def test_scope_is_reentrant(self):
        with green_tree_scope() as outer:
            with green_tree_scope() as inner:
                assert inner is outer


class TestTokeniseShim:
    def test_tokenise_returns_node_tokens(self):
        toks, warnings = tokenise("set a 1", 0, 0, 0)
        assert [t.type for t in toks][0] is TokenType.ESC
        assert isinstance(warnings, tuple)

    def test_tokenise_shares_with_node_for(self):
        with green_tree_scope():
            toks, _ = tokenise("set a 1", 0, 0, 0)
            node = node_for("set a 1", 0, 0, 0)
            assert node.tokens is toks
