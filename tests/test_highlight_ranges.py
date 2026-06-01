"""Highlight-range accuracy tests.

When you hover a CFG node, an IR clause, an optimisation, or a shimmer
warning in the compiler explorer, the editor draws a highlight over the
range the server emitted for that artifact.  The highlight is produced by
slicing the source with the range's offsets, so a wrong range highlights the
wrong text — e.g. ``{$condition}`` shows up as ``{$conditi`` with the closing
brace (and a couple of inner characters) missing.

These tests pull every range the explorer serialises, slice the source the
same way the front-end does, and assert the covered text is a *complete,
well-formed* token: balanced braces and brackets, and — when the span opens
with a delimiter — closed by the matching one.  Several constructs also have
exact-coverage assertions so the off-by-one is pinned per feature, not just
caught in aggregate.

Two conventions matter:

* The explorer front-end slices ``src.substring(startOffset, endOffset)`` —
  an *exclusive* end (see ``tooling/explorer/static/index.html``
  ``highlightSourceRanges``).
* The semantic-model :class:`Range` end is *inclusive* (see
  :func:`server._lsp_conv.to_lsp_range`, which adds 1 to convert it).

So a serialised ``endOffset`` is correct only when it is the exclusive end —
the inclusive end plus one.
"""

from __future__ import annotations

import pytest

from analyser.semantic_model import Range
from shared.ranges import (
    range_from_word_token,
    reset_highlight_source,
    set_highlight_source,
    widen_for_highlight,
)
from shared.tokens import SourcePosition, Token, TokenType
from tooling.cli.serialise import serialise_result
from tooling.explorer.pipeline import run_pipeline

# ── source corpus ──────────────────────────────────────────────────────

CORPUS = {
    "if": "if {$condition} {\n  set x 1\n}\n",
    "if_else": "if {$a} {\n  set x 1\n} else {\n  set x 2\n}\n",
    "if_elseif_else": (
        "if {$a} {\n  set x 1\n} elseif {$b} {\n  set x 2\n} else {\n  set x 3\n}\n"
    ),
    "while": "while {$i < 10} {\n  incr i\n}\n",
    "for": "for {set i 0} {$i < 10} {incr i} {\n  puts $i\n}\n",
    "foreach": "foreach item {a b c} {\n  puts $item\n}\n",
    "switch": "switch -- $x {\n  one {set y 1}\n  two {set y 2}\n  default {set y 0}\n}\n",
    # A switch arm whose *last* statement is optimised (dead-store / fold):
    # the rewrite's reported range must stop before the arm's closing brace,
    # not cover `set y 2}` (regression: glued enclosing brace).
    "switch_opt": "switch -- $x {\n  one {set y 1\n    set y 2}\n}\nputs $y\n",
    "proc": 'proc greet {name} {\n  puts "hi $name"\n}\n',
    "catch": "catch {error boom} result\n",
    "nested": "if {$a} {\n  while {$b} {\n    set z [expr {$z + 1}]\n  }\n}\n",
}


# ── helpers ────────────────────────────────────────────────────────────


# Top-level serialisation keys whose ranges index into the *optimised*
# source rather than the source the user typed.
_OPTIMISED_KEYS = {
    "asmOptimised",
    "wasmOptimised",
    "irOptimised",
    "cfgPreSsaOptimised",
    "cfgPostSsaOptimised",
}

# The green tree is a lossless *lexer* view: its token offsets are raw token
# spans (a single SEP/EOL, or a braced word's opening delimiter + inner text
# without the closer), not the balanced whole-word highlight ranges the IR /
# CFG / optimiser artefacts emit.  Highlighting a green-tree token to its exact
# raw span is correct, so those ranges are exempt from the well-formed-word
# assertion below.
_RAW_TOKEN_KEYS = {"greentree"}


def _explorer_dict(source: str) -> dict:
    return serialise_result(run_pipeline(source))


def _explorer_data(source: str) -> tuple[dict, str, str]:
    """Return ``(serialised_dict, source, optimised_source)``.

    The optimiser may rewrite the source (e.g. ``set z [expr {$z + 1}]`` →
    ``incr z``), so ranges under the optimised asm/wasm views index into the
    optimised text — they must be sliced against that, not the original.
    """
    result = run_pipeline(source)
    return serialise_result(result), result.source, result.optimised_source


def _is_range_dict(obj: object) -> bool:
    return isinstance(obj, dict) and "startOffset" in obj and "endOffset" in obj


def _walk_ranges(obj: object, path: str = ""):
    """Yield ``(path, range_dict)`` for every range dict in *obj*."""
    if _is_range_dict(obj):
        yield path or "<root>", obj
        return
    if isinstance(obj, dict):
        for key, value in obj.items():
            yield from _walk_ranges(value, f"{path}.{key}" if path else str(key))
    elif isinstance(obj, list):
        for i, value in enumerate(obj):
            yield from _walk_ranges(value, f"{path}[{i}]")


def _covered(source: str, rd: dict) -> str:
    """Slice the source the way the explorer front-end does (exclusive end)."""
    return source[rd["startOffset"] : rd["endOffset"]]


_OPEN_TO_CLOSE = {"{": "}", "[": "]"}
_CLOSERS = set(_OPEN_TO_CLOSE.values())


def _delims_balanced(text: str) -> bool:
    """True when every ``{``/``[`` in *text* is matched, honouring ``\\`` escapes."""
    stack: list[str] = []
    i = 0
    while i < len(text):
        c = text[i]
        if c == "\\":
            i += 2
            continue
        if c in _OPEN_TO_CLOSE:
            stack.append(c)
        elif c in _CLOSERS:
            if not stack or _OPEN_TO_CLOSE[stack[-1]] != c:
                return False
            stack.pop()
        i += 1
    return not stack


def _well_formed(text: str) -> bool:
    """A highlight covers a complete token: non-empty, balanced, and when it
    opens with a delimiter it closes with the matching one."""
    if not text:
        return False
    if not _delims_balanced(text):
        return False
    if text[0] in _OPEN_TO_CLOSE:
        return text[-1] == _OPEN_TO_CLOSE[text[0]]
    return True


# ── generic well-formedness across every serialised range ──────────────


class TestEveryExplorerRangeIsWellFormed:
    @pytest.mark.parametrize("name", sorted(CORPUS))
    def test_every_range_covers_a_complete_token(self, name):
        data, source, optimised = _explorer_data(CORPUS[name])
        problems = []
        for path, rd in _walk_ranges(data):
            top_key = path.split(".", 1)[0].split("[", 1)[0]
            if top_key in _RAW_TOKEN_KEYS:
                continue
            src = optimised if top_key in _OPTIMISED_KEYS else source
            covered = _covered(src, rd)
            if not _well_formed(covered):
                problems.append((path, covered, rd["startOffset"], rd["endOffset"]))
        assert not problems, (
            f"[{name}] {len(problems)} range(s) highlight an incomplete/"
            f"unbalanced span:\n"
            + "\n".join(f"  {p}: covered={c!r} (off {s}..{e})" for p, c, s, e in problems)
        )


# ── exact-coverage assertions per construct ────────────────────────────


def _covered_set(source: str) -> set[str]:
    data = _explorer_dict(source)
    return {_covered(source, rd) for _, rd in _walk_ranges(data)}


class TestExactCoverage:
    def test_if_condition_includes_closing_brace(self):
        source = CORPUS["if"]
        assert "{$condition}" in _covered_set(source)

    def test_if_body_includes_closing_brace(self):
        source = CORPUS["if"]
        covered = _covered_set(source)
        assert any(c.startswith("{") and c.endswith("}") and "set x 1" in c for c in covered)

    def test_while_condition_includes_closing_brace(self):
        assert "{$i < 10}" in _covered_set(CORPUS["while"])

    def test_for_loop_clauses_include_braces(self):
        covered = _covered_set(CORPUS["for"])
        assert "{set i 0}" in covered
        assert "{$i < 10}" in covered
        assert "{incr i}" in covered

    def test_foreach_statement_range_round_trips(self):
        # foreach is not specialised in the serialiser, so it surfaces as a
        # whole-statement range — which must include the closing body brace.
        covered = _covered_set(CORPUS["foreach"])
        assert "foreach item {a b c} {\n  puts $item\n}" in covered

    def test_switch_arm_bodies_include_braces(self):
        covered = _covered_set(CORPUS["switch"])
        assert "{set y 1}" in covered
        assert "{set y 2}" in covered

    def test_command_sub_statement_includes_closing_bracket(self):
        # The set statement carrying a [expr {...}] substitution must keep the
        # closing ``]`` (and the brace inside it).
        covered = _covered_set(CORPUS["nested"])
        assert "set z [expr {$z + 1}]" in covered


# ── multi-range (disjoint) highlights — if / elseif / else ─────────────


def _disjoint_and_ordered(ranges: list[dict]) -> bool:
    """A multi-select highlight is a set of non-overlapping spans in order."""
    spans = sorted((r["startOffset"], r["endOffset"]) for r in ranges)
    return all(spans[i][1] <= spans[i + 1][0] for i in range(len(spans) - 1))


class TestMultiRangeHighlights:
    def test_if_else_branch_spans_are_each_well_formed_and_disjoint(self):
        # An if/else highlight conceptually covers several disjoint source
        # spans (the condition, the then-body, the else-body) with the
        # keywords/whitespace in the gaps between them.  Each span must be a
        # complete token, and collectively they must not overlap.
        source = CORPUS["if_else"]
        data = _explorer_dict(source)
        # Pull the if node's clause + else ranges out of the IR serialisation.
        if_nodes = [n for n in data["ir"]["topLevel"] if n.get("kind") == "IRIf"]
        assert if_nodes, "expected an if node in the IR"
        node = if_nodes[0]
        spans = []
        for child in node.get("children", []):
            if child.get("range"):
                spans.append(child["range"])
        assert len(spans) >= 2, "if/else should expose multiple disjoint ranges"
        for rd in spans:
            assert _well_formed(_covered(source, rd)), (
                f"branch span not well-formed: {_covered(source, rd)!r}"
            )
        assert _disjoint_and_ordered(spans), "branch spans overlap"


# ── unit tests for the range helpers underpinning the fix ──────────────


def _pos(offset: int, *, line: int = 0, character: int | None = None) -> SourcePosition:
    return SourcePosition(
        line=line, character=offset if character is None else character, offset=offset
    )


def _word_token(ttype: TokenType, text: str, start: int, end: int) -> Token:
    return Token(type=ttype, text=text, start=_pos(start), end=_pos(end))


def _offset_of(source: str, line: int, character: int) -> int:
    """Resolve a (line, character) position to an absolute offset in source."""
    line_starts = [0]
    for i, ch in enumerate(source):
        if ch == "\n":
            line_starts.append(i + 1)
    return line_starts[line] + character


class TestRangeFromWordToken:
    def test_str_token_includes_closing_brace(self):
        # `{$condition}` at offsets 0..11: the STR token spans 0..10 (the `{`
        # through the last inner char), and the closer is one past the end.
        tok = _word_token(TokenType.STR, "$condition", start=0, end=10)
        r = range_from_word_token(tok)
        assert (r.start.offset, r.end.offset) == (0, 11)

    def test_cmd_token_includes_closing_bracket(self):
        tok = _word_token(TokenType.CMD, "expr {$z + 1}", start=0, end=13)
        r = range_from_word_token(tok)
        assert r.end.offset == 14

    def test_plain_word_token_unchanged(self):
        tok = _word_token(TokenType.ESC, "set", start=0, end=2)
        r = range_from_word_token(tok)
        assert (r.start.offset, r.end.offset) == (0, 2)

    def test_var_token_unchanged(self):
        tok = _word_token(TokenType.VAR, "x", start=0, end=1)
        r = range_from_word_token(tok)
        assert r.end.offset == 1

    def test_multiline_body_closer_advances_line_and_column(self):
        # `{\n  set x 1\n}` — the closing `}` is on the next line at column 0,
        # so the widened end's line/column must stay consistent with offset.
        source = "if {$a} {\n  set x 1\n}\n"
        # The body STR token: `{` at offset 8, inner ends on the `\n` at 19.
        tok = Token(
            type=TokenType.STR,
            text="\n  set x 1\n",
            start=SourcePosition(line=0, character=8, offset=8),
            end=SourcePosition(line=1, character=9, offset=19),
        )
        r = range_from_word_token(tok)
        assert source[r.end.offset] == "}"
        assert _offset_of(source, r.end.line, r.end.character) == r.end.offset
        assert (r.end.line, r.end.character) == (2, 0)


class TestWidenForHighlight:
    def test_no_op_without_source(self):
        r = Range(start=_pos(3), end=_pos(13))
        assert widen_for_highlight(r) is r

    def test_widens_braced_word_when_source_set(self):
        source = "if {$condition} {body}\n"
        r = Range(start=_pos(3), end=_pos(13))  # `{$condition` (closer at 14)
        token = set_highlight_source(source)
        try:
            widened = widen_for_highlight(r)
        finally:
            reset_highlight_source(token)
        assert widened.end.offset == 14
        assert source[r.start.offset : widened.end.offset + 1] == "{$condition}"

    def test_does_not_widen_non_delimiter_span(self):
        source = "set x 1\n"
        r = Range(start=_pos(0), end=_pos(2))  # `set`
        token = set_highlight_source(source)
        try:
            assert widen_for_highlight(r).end.offset == 2
        finally:
            reset_highlight_source(token)

    def test_multiline_closer_keeps_line_column_consistent(self):
        # Closer on the next line: offset and line/column must agree.
        source = "if {$a} {\n  set x 1\n}\n"
        r = Range(
            start=SourcePosition(line=0, character=8, offset=8),  # `{`
            end=SourcePosition(line=1, character=9, offset=19),  # the `\n`
        )
        token = set_highlight_source(source)
        try:
            widened = widen_for_highlight(r)
        finally:
            reset_highlight_source(token)
        assert source[widened.end.offset] == "}"
        assert _offset_of(source, widened.end.line, widened.end.character) == widened.end.offset


class TestRangeDictConversion:
    def test_inclusive_to_exclusive_end(self):
        # Without a highlight source, range_dict still converts the inclusive
        # semantic-model end to the exclusive end the front-end slices with.
        from tooling.cli.formatters import range_dict

        rd = range_dict(Range(start=_pos(0), end=_pos(2)))
        assert rd["startOffset"] == 0
        assert rd["endOffset"] == 3
        assert rd["endCol"] == 3

    def test_widens_and_converts_with_source(self):
        from tooling.cli.formatters import range_dict

        source = "if {$condition} {body}\n"
        token = set_highlight_source(source)
        try:
            rd = range_dict(Range(start=_pos(3), end=_pos(13)))
        finally:
            reset_highlight_source(token)
        # widened to the closer (14, inclusive) then +1 for exclusive end.
        assert rd["endOffset"] == 15
        assert source[rd["startOffset"] : rd["endOffset"]] == "{$condition}"
