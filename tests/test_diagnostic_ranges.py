"""Range-accuracy tests for diagnostics and optimisations.

The editor draws a squiggle over a diagnostic's range, so the range must
cover *exactly* the offending span — no dropped trailing delimiter, and as
narrow as possible while still covering the whole issue (never the entire
enclosing command/proc when a token suffices).  These tests assert the exact
covered substring, and also assert that clean code produces no diagnostic
(no false positives).

Codes whose ranges are still too wide or drop a trailing delimiter are
tracked as an audit follow-up rather than asserted here, so this file stays
a green description of verified-correct behaviour.
"""

from __future__ import annotations

from lsprotocol import types

from server.features.diagnostics import get_diagnostics


def _covered(source: str, r: types.Range) -> str:
    lines = source.split("\n")
    sl, sc, el, ec = r.start.line, r.start.character, r.end.line, r.end.character
    if sl == el:
        return lines[sl][sc:ec]
    parts = [lines[sl][sc:], *lines[sl + 1 : el], lines[el][:ec]]
    return "\n".join(parts)


def _diags_for_code(source: str, code: str) -> list[types.Diagnostic]:
    return [
        d
        for d in get_diagnostics(source)
        if (d.code if isinstance(d.code, str) else str(d.code)) == code
    ]


def assert_covers(source: str, code: str, expected: str) -> None:
    """Assert at least one *code* diagnostic's range covers exactly *expected*."""
    matches = _diags_for_code(source, code)
    assert matches, f"{code} did not fire on {source!r}"
    covered = {_covered(source, d.range) for d in matches}
    assert expected in covered, (
        f"{code} range should cover {expected!r}; covered {sorted(covered)} "
        f"(ranges: {[(d.range.start.line, d.range.start.character, d.range.end.line, d.range.end.character) for d in matches]})"
    )


def assert_not_flagged(source: str, code: str) -> None:
    matches = _diags_for_code(source, code)
    assert not matches, f"{code} should not fire on {source!r} but did"


# ── exact-range true positives ─────────────────────────────────────────


class TestDiagnosticRangesExact:
    def test_e002_covers_command_word_only(self):
        assert_covers("set\n", "E002", "set")

    def test_w100_covers_the_unbraced_expr_word(self):
        assert_covers("if $x == 1 {puts hi}\n", "W100", "$x")

    def test_w302_covers_catch_keyword_only(self):
        # Regression: W302 used to span the whole ``catch {…}`` statement and
        # drop the closing brace; it now anchors on the ``catch`` word.
        assert_covers("catch {error oops}\n", "W302", "catch")


class TestOptimisationRangesExact:
    def test_o100_covers_the_propagated_reference(self):
        assert_covers("set x [expr {1 + 2}]\nputs $x\n", "O100", "$x")

    def test_o111_covers_the_unbraced_expr_text(self):
        assert_covers("expr $a + $b\n", "O111", "$a + $b")

    def test_o120_covers_the_full_braced_expression(self):
        # The braced expression range must include the closing brace.
        assert_covers('if {$x == "hello"} {puts y}\n', "O120", '{$x == "hello"}')


# ── no false positives ─────────────────────────────────────────────────


class TestNoFalsePositives:
    def test_w100_not_flagged_when_braced(self):
        assert_not_flagged("if {$x == 1} {puts hi}\n", "W100")

    def test_w302_not_flagged_with_result_variable(self):
        assert_not_flagged("catch {error oops} result\n", "W302")

    def test_e002_not_flagged_on_valid_set(self):
        assert_not_flagged("set x 1\n", "E002")

    def test_w210_not_flagged_when_set_before_read(self):
        assert_not_flagged("set x 1\nputs $x\n", "W210")

    def test_w110_not_flagged_when_using_eq(self):
        assert_not_flagged('if {$x eq "hello"} {set x done}\n', "W110")

    def test_clean_script_has_no_diagnostics(self):
        assert get_diagnostics("set x [clock seconds]\nputs $x\n") == []
