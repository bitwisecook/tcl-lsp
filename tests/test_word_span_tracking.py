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

"""Regression battery for word-span off-by-one tracking (issue #527 + family).

Word tokens follow the lexer's *inner-end* convention — ``Token.end.offset`` is
the last inner character and the closing ``}`` / ``]`` / ``"`` is one past it —
*except* for an empty ``{}`` / ``[]`` / ``""`` whose ``end`` already sits on the
closer.  Consumers that re-derived the closer as ``end.offset + 1`` overshot by
one for empty words.  The fix routes every closer/full-span computation through
the authoritative accessors in :mod:`shared.ranges` (``word_closer_offset``,
``word_end_position``, ``range_from_word_token``), which derive the bound from
the lexer's content geometry rather than re-computing it.

These tests pin the accessors against the lexer, and assert end-to-end that no
diagnostic or refactor span overshoots on empty/quoted delimiters.

The closer positions are the same ones Tcl's own parser computes: in C Tcl a
braced word's closer sits at ``open + 1 + inner_size`` (``Tcl_ParseBraces`` sets
the content token ``size = src - tokenPtr->start`` and ``*termPtr = src + 1``;
``tmp/tcl9.0.3/generic/tclParse.c``), which for an empty ``{}`` is ``open + 1``
with no special case — the exclusive-end model that the inner-end convention has
to special-case.
"""

from __future__ import annotations

import pytest

from analyser import analyse
from compiler.parsing.lexer import TclLexer
from shared.ranges import range_from_word_token, word_closer_offset, word_end_position
from shared.tokens import TokenType


def _last_word_token(src: str):
    toks = [
        t
        for t in TclLexer(src).tokenise_all()
        if t.type in (TokenType.STR, TokenType.CMD, TokenType.ESC, TokenType.VAR)
    ]
    return toks[-1] if toks else None


class TestWordCloserOffset:
    @pytest.mark.parametrize(
        ("src", "want"),
        [
            ("{}", 1),  # empty brace: end already on the closer
            ("{a}", 2),
            ("{abc}", 4),
            ("{a {b}}", 6),  # nested: outer closer, not the inner one at end
            ("[]", 1),  # empty bracket
            ("[x]", 2),
            ('""', 1),  # empty quote
            ('"a"', 2),
            (r'"a\tb"', 5),  # backslash escape kept verbatim in the token text
            ("set d {}", 7),
            ("return {}", 8),
        ],
    )
    def test_closer_offset(self, src, want):
        tok = _last_word_token(src)
        assert word_closer_offset(tok, src) == want
        assert src[want] in '}]"'

    @pytest.mark.parametrize("src", ["plain", "set", "$x", "$arr(i)"])
    def test_non_delimited_returns_none(self, src):
        tok = _last_word_token(src)
        assert word_closer_offset(tok, src) is None

    @pytest.mark.parametrize("src", ["{abc", "[foo", '"abc'])
    def test_unterminated_returns_none(self, src):
        tok = _last_word_token(src)
        assert word_closer_offset(tok, src) is None


class TestWordEndPosition:
    def test_empty_brace_end_is_the_closer(self):
        # `x {}` — the `{}` token's end IS the closing brace; do not advance.
        src = "x {}"
        tok = _last_word_token(src)
        pos = word_end_position(tok, src)
        assert pos.offset == 3
        assert src[pos.offset] == "}"

    def test_nonempty_brace_advances_to_closer(self):
        src = "x {ab}"
        tok = _last_word_token(src)
        pos = word_end_position(tok, src)
        assert pos.offset == 5
        assert src[pos.offset] == "}"

    def test_quoted_word_covers_closing_quote(self):
        src = 'x "ab"'
        tok = _last_word_token(src)
        pos = word_end_position(tok, src)
        assert pos.offset == 5
        assert src[pos.offset] == '"'

    def test_multiline_closer_keeps_line_column_consistent(self):
        src = "if {$a} {\n  set x 1\n}\n"
        body = [t for t in TclLexer(src).tokenise_all() if t.type is TokenType.STR][-1]
        pos = word_end_position(body, src)
        assert src[pos.offset] == "}"
        # offset and line/column must agree (closer is at column 0 of line 2)
        assert (pos.line, pos.character) == (2, 0)


class TestRangeFromWordToken:
    # ``range_from_word_token`` is token-only (no source): it covers the closer
    # for STR/CMD words and leaves quoted ESC words at their inner end (those are
    # widened source-aware via ``word_end_position`` — see TestWordEndPosition).
    @pytest.mark.parametrize(
        ("src", "covered"),
        [
            ("set x {}", "{}"),  # empty brace — no overshoot (issue #527)
            ("set x {b}", "{b}"),
            ("set x []", "[]"),  # empty bracket
            ("set x [c]", "[c]"),
        ],
    )
    def test_full_word_span_braces_brackets(self, src, covered):
        tok = _last_word_token(src)
        r = range_from_word_token(tok)
        assert src[r.start.offset : r.end.offset + 1] == covered


# Valid Tcl featuring empty and quoted delimiters in many positions.  None of
# these should produce a diagnostic whose highlighted span overshoots its token.
_EMPTY_DELIMITER_SNIPPETS = [
    "set d {}",
    'set s ""',
    "return {}",
    "if {[llength $x] != 2} {return {}}",  # the exact issue #527 line shape
    "if {$x} {set y {}}",
    "proc f {} {}",
    "proc f {a} {return {}}",
    "lappend xs {}",
    "dict set d k {}",
    "dict create a {} b {}",
    "foreach x $l {lappend o {}}",
    'set x [list ""]',
    "namespace eval ns {}",
    'puts "hello"',
    'set m "a b c"',
    'if {$x} {puts ""}',
]


def _overshoots(src: str, rng) -> str | None:
    s, e = rng.start.offset, rng.end.offset
    if not (0 <= s <= e < len(src)):
        return f"out-of-bounds [{s},{e}] len={len(src)}"
    sl = src[s : e + 1]
    for op, cl in (("{", "}"), ("[", "]")):
        if sl.endswith(cl) and sl.count(op) < sl.count(cl):
            return f"unbalanced trailing {cl!r}: slice={sl!r}"
    return None


@pytest.mark.parametrize("src", _EMPTY_DELIMITER_SNIPPETS)
def test_no_diagnostic_span_overshoots_on_empty_or_quoted_delimiters(src):
    result = analyse(src)
    for d in result.diagnostics:
        bad = _overshoots(src, d.range)
        assert bad is None, f"{d.code} on {src!r}: {bad}"


@pytest.mark.parametrize("src", _EMPTY_DELIMITER_SNIPPETS)
def test_refactor_command_spans_do_not_overshoot(src):
    # The refactoring layer must produce balanced command spans too.
    from compiler.parsing.command_segmenter import segment_commands
    from tooling.refactoring._spans import command_span_offsets

    for cmd in segment_commands(src):
        s, e = command_span_offsets(src, cmd)
        sl = src[s:e]
        for op, cl in (("{", "}"), ("[", "]")):
            assert not (sl.endswith(cl) and sl.count(op) < sl.count(cl)), (
                f"command span overshoot in {src!r}: {sl!r}"
            )


class TestCommandRangeCoversClosers:
    """``cmd.range`` is derived token-only (the next SEP/EOL boundary − 1): it
    covers the final word's closing delimiter for braces, brackets, AND quoted
    words, never overshoots an empty `{}`/`""`, and extends to the true end of a
    compound word — with no source re-scan and no base_offset."""

    @pytest.mark.parametrize(
        "src",
        [
            'set x "world"',  # quoted, literal-ending — previously dropped the "
            'set x "$y"',  # quoted, substitution-ending (closing-quote fragment)
            'set x "a$y b"',  # quoted, mixed
            "set x {body}",
            "set x {}",  # empty brace
            'set x ""',  # empty quote
            "set x [f]",
            "set x plain",
            "set x {a}b",  # compound word — covers the trailing `b`, not just `}`
            "if {1} {return {}}",  # the issue #527 shape
        ],
    )
    def test_command_range_covers_full_command(self, src):
        from compiler.parsing.command_segmenter import segment_commands

        (cmd,) = segment_commands(src)
        covered = src[cmd.range.start.offset : cmd.range.end.offset + 1]
        assert covered == src

    def test_command_range_excludes_trailing_whitespace(self):
        # `set x {a} ;` — the command ends at `}`, not the trailing space.
        from compiler.parsing.command_segmenter import segment_commands

        src = "set x {a} ;"
        (cmd,) = segment_commands(src)
        assert src[cmd.range.start.offset : cmd.range.end.offset + 1] == "set x {a}"

    def test_nested_body_command_range_is_token_only(self):
        # Nested body: absolute token offsets, substring source — the command
        # range is now derived token-only (boundary rule), no source/base_offset,
        # and must still cover the literal-ending `"world"` closer.
        from compiler.parsing.command_segmenter import segment_commands

        full = 'proc f {} {set x "world"}'
        body = [t for t in TclLexer(full).tokenise_all() if t.type is TokenType.STR][-1]
        (cmd,) = segment_commands(body.text, body_token=body)
        assert full[cmd.range.start.offset : cmd.range.end.offset + 1] == 'set x "world"'


class TestMigratedWideners:
    """Lock the empty-delimiter behaviour of the migrated raw-source helpers."""

    def test_irules_raw_arg_text_empty_brace_no_overshoot(self):
        from analyser.irules_checks import _raw_arg_text

        # `cmd {}}` — the empty `{}` arg must round-trip as `{}`, not `{}}`.
        src = "cmd {}}"
        tok = next(t for t in TclLexer(src).tokenise_all() if t.type is TokenType.STR)
        assert _raw_arg_text(src, tok) == "{}"

    def test_irules_raw_arg_text_quoted_round_trips(self):
        from analyser.irules_checks import _raw_arg_text

        src = 'cmd "a b"'
        tok = next(
            t for t in TclLexer(src).tokenise_all() if t.type is TokenType.ESC and t.text == "a b"
        )
        assert _raw_arg_text(src, tok) == '"a b"'

    def test_style_widen_for_close_delim_empty_brace_guarded(self):
        from analyser.checks._style import _widen_for_close_delim
        from shared.tokens import SourcePosition

        # `{}}` opener at 0, inclusive end on the empty word's own `}` at 1.
        src = "x {}}"
        start = 2  # the `{`
        inclusive_end = SourcePosition(line=0, character=3, offset=3)  # the `}`
        excl, new_end = _widen_for_close_delim(src, start, inclusive_end)
        # Must not widen into the trailing `}` at offset 4.
        assert excl == 4
        assert new_end.offset == 3
