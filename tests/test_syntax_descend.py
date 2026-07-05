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

"""Tests for CST descent into braced bodies and command substitutions.

Descent re-lexes a ``{…}`` body or ``[…]`` substitution as a child tree anchored
one byte past the opener — the delimiter-excluding interior of a token that owns
the full span.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.parsing.green_tree import node_for
from compiler.parsing.syntax import descend_command, descend_token
from compiler.parsing.syntax.red import SyntaxNode, SyntaxTree
from shared.tokens import TokenType

_TRIVIA = (TokenType.SEP, TokenType.EOL, TokenType.COMMENT)


def _opaque_tokens(src: str):
    return [t for t in node_for(src).tokens if t.type in (TokenType.STR, TokenType.CMD)]


def _red_fragments(tree: SyntaxTree):
    out = []
    for cmd in tree.root.children():
        if isinstance(cmd, SyntaxNode):
            out.extend(st.to_token() for st in cmd.tokens())
    return out


def _single_command(src: str):
    toks = [t for t in node_for(src).tokens if t.type not in _TRIVIA]
    return toks[0].text, [t.text for t in toks[1:]], toks[1:]


class TestDescendToken:
    def test_braced_body_anchored_past_brace(self):
        src = "proc p {} {set x 1}"
        body = next(t for t in _opaque_tokens(src) if "set x" in t.text)
        child = descend_token(body, src)
        assert child.terminated
        first = next(child.root.tokens())
        # The interior starts one byte past the `{`.
        assert first.start_position.offset == body.start.offset + 1
        assert src[first.start_position.offset] == "s"

    def test_command_substitution(self):
        src = "set y [expr 1]"
        cmd = next(t for t in _opaque_tokens(src) if t.type is TokenType.CMD)
        child = descend_token(cmd, src)
        assert child.terminated
        assert next(child.root.tokens()).start_position.offset == cmd.start.offset + 1

    def test_unterminated_brace_is_recovered(self):
        src = "proc p {} {set x 1"
        body = next(t for t in _opaque_tokens(src) if "set x" in t.text)
        child = descend_token(body, src)
        assert not child.terminated
        # A recovered region still carries its inner tokens.
        assert any(t.text == "set" for t in _red_fragments(child.tree))

    def test_unterminated_bracket_is_recovered(self):
        src = "set y [expr 1"
        cmd = next(t for t in _opaque_tokens(src) if t.type is TokenType.CMD)
        assert not descend_token(cmd, src).terminated

    def test_empty_brace_terminated(self):
        src = "proc p {} {}"
        for body in _opaque_tokens(src):
            assert body.text == ""
            assert descend_token(body, src).terminated

    def test_nested_braces_terminated(self):
        src = "nest {a {b} c}"
        body = next(t for t in _opaque_tokens(src) if t.type is TokenType.STR)
        assert descend_token(body, src).terminated


class TestDescendCommand:
    def test_resolves_body_to_child_tree(self):
        src = "proc p {x} {set x 1}"
        name, args, arg_tokens = _single_command(src)
        bodies = descend_command(name, args, arg_tokens, src)
        assert len(bodies) == 1
        assert bodies[0].descended.terminated
        assert any(t.text == "set" for t in _red_fragments(bodies[0].descended.tree))

    def test_expr_arg_not_descended(self):
        # `if` has an EXPR condition and a BODY; only the body is descended.
        src = "if {$x > 1} {puts hi}"
        name, args, arg_tokens = _single_command(src)
        bodies = descend_command(name, args, arg_tokens, src)
        assert len(bodies) == 1
        assert "puts" in bodies[0].text

    def test_empty_body_skipped(self):
        src = "proc p {} {}"
        name, args, arg_tokens = _single_command(src)
        assert descend_command(name, args, arg_tokens, src) == []


class TestMultiLevelDescent:
    def test_substitution_inside_body(self):
        # Descend a body, then descend a [..] inside it against the full source.
        src = "proc p {} {set y [expr {1 + 2}]}"
        body = next(t for t in _opaque_tokens(src) if "set y" in t.text)
        child = descend_token(body, src)
        # The inner CMD token carries absolute positions, so it descends directly
        # against the full source.
        inner_tok = next(
            st.to_token()
            for cmd in child.tree.root.children()
            if isinstance(cmd, SyntaxNode)
            for st in cmd.tokens()
            if st.token_type is TokenType.CMD
        )
        deep = descend_token(inner_tok, src)
        assert deep.terminated
        assert any(t.text == "expr" for t in _red_fragments(deep.tree))


class TestStructuralFraming:
    def test_token_owns_span_child_excludes_delimiters(self):
        src = "proc p {} {set x 1}"
        body = next(t for t in _opaque_tokens(src) if "set x" in t.text)
        # The token owns the full span including braces…
        assert src[body.start.offset] == "{"
        child = descend_token(body, src)
        # …while the descended child begins at the first interior byte.
        first = next(child.root.tokens())
        assert first.start_position.offset == body.start.offset + 1
        assert src[first.start_position.offset] != "{"
