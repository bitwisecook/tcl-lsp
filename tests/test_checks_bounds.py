"""Tests for index-bounds and loop-termination checks (W230-W242).

Semantics validated against Tcl 9.0.3 (built from source) — see
``core/analysis/checks/_bounds.py`` for the reference notes.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.analyser import analyse
from core.analysis.semantic_model import Severity


def _diag_with_code(source: str, code: str):
    result = analyse(source)
    return [d for d in result.diagnostics if d.code == code]


# W230: list index out of range (silent commands)


class TestListIndexOutOfRange:
    def test_lindex_positive_overshoot(self):
        diags = _diag_with_code("lindex {a b c} 5", "W230")
        assert len(diags) == 1
        assert diags[0].severity is Severity.WARNING
        assert "list has 3 elements" in diags[0].message

    def test_lindex_negative_literal(self):
        diags = _diag_with_code("lindex {a b c} -1", "W230")
        assert len(diags) == 1
        assert "before start of list" in diags[0].message

    def test_lindex_end_minus_underflow(self):
        diags = _diag_with_code("lindex {a b c} end-5", "W230")
        assert len(diags) == 1
        assert "resolves to -3" in diags[0].message

    def test_lindex_in_range_clean(self):
        assert _diag_with_code("lindex {a b c} 0", "W230") == []
        assert _diag_with_code("lindex {a b c} 2", "W230") == []
        assert _diag_with_code("lindex {a b c} end", "W230") == []
        assert _diag_with_code("lindex {a b c} end-2", "W230") == []

    def test_lindex_dynamic_list_skipped(self):
        assert _diag_with_code("lindex $L 5", "W230") == []

    def test_lindex_dynamic_index_skipped(self):
        assert _diag_with_code("lindex {a b c} $i", "W230") == []

    def test_lrange_both_above(self):
        # Slice empty: both indices above the list.
        diags = _diag_with_code("lrange {a b c} 10 20", "W230")
        assert len(diags) == 1
        assert "slice is empty" in diags[0].message

    def test_lrange_both_below_with_end_minus(self):
        # Slice empty: both indices resolve below 0.
        diags = _diag_with_code("lrange {a b c} end-99 -50", "W230")
        assert len(diags) == 1

    def test_lrange_single_out_of_range_clean(self):
        # Only one index out of range: Tcl clamps to a valid slice.
        assert _diag_with_code("lrange {a b c} 0 100", "W230") == []
        assert _diag_with_code("lrange {a b c} -5 2", "W230") == []

    def test_linsert_not_flagged(self):
        # linsert always produces a sensible result — no diagnostic.
        assert _diag_with_code("linsert {a b c} -5 X", "W230") == []
        assert _diag_with_code("linsert {a b c} end+99 X", "W230") == []

    def test_lreplace_both_below_prepends(self):
        # Tcl clamps both indices to 0 and prepends X rather than
        # replacing an element — almost certainly a bug.
        diags = _diag_with_code("lreplace {a b c} end-99 end-99 X", "W230")
        assert len(diags) == 1
        assert "prepends instead of replacing" in diags[0].message

    def test_lreplace_both_above_appends(self):
        diags = _diag_with_code("lreplace {a b c} 10 20 X", "W230")
        assert len(diags) == 1
        assert "appends instead of replacing" in diags[0].message

    def test_lreplace_first_greater_than_last(self):
        diags = _diag_with_code("lreplace {a b c} 2 0 X", "W230")
        assert len(diags) == 1
        assert "touches no element" in diags[0].message


# W231: lset index out of range (hard error)


class TestLsetIndexOutOfRange:
    def test_lset_negative_literal(self):
        diags = _diag_with_code("lset L -1 X", "W231")
        assert len(diags) == 1
        assert diags[0].severity is Severity.WARNING
        assert "raises 'index out of range'" in diags[0].message

    def test_lset_negative_big(self):
        diags = _diag_with_code("lset L -100 X", "W231")
        assert len(diags) == 1

    def test_lset_with_known_list_length(self):
        source = "set L {a b c}\nlset L 5 X"
        diags = _diag_with_code(source, "W231")
        assert len(diags) == 1

    def test_lset_in_range_clean(self):
        source = "set L {a b c}\nlset L 0 X"
        assert _diag_with_code(source, "W231") == []

    def test_lset_end_minus_in_range(self):
        source = "set L {a b c}\nlset L end-2 X"
        assert _diag_with_code(source, "W231") == []

    def test_lset_end_minus_underflow_when_length_known(self):
        source = "set L {a}\nlset L end-2 X"
        diags = _diag_with_code(source, "W231")
        assert len(diags) == 1

    def test_lset_append_position_clean(self):
        # Tcl 9: lset L end+1 X appends; lset L <length> X appends too.
        assert _diag_with_code("set L {a b c}\nlset L end+1 X", "W231") == []
        assert _diag_with_code("set L {a b c}\nlset L 3 X", "W231") == []

    def test_lset_past_append_flagged(self):
        source = "set L {a b c}\nlset L end+2 X"
        diags = _diag_with_code(source, "W231")
        assert len(diags) == 1

    def test_lset_dynamic_index_skipped(self):
        assert _diag_with_code("lset L $i X", "W231") == []

    def test_lset_nested_top_level_index_valid_but_inner_cannot_be_checked(self):
        # Codex P2: after `set L {{a} {b} {c}}`, `lset L 0 2 X` looks
        # like index 2 into the top-level list (valid: length 3), but
        # the second index applies to the sublist `{a}` where it is
        # out of range.  We cannot see the sublist length, so we must
        # back off completely for multi-index forms — no W231.
        source = "set L {{a} {b} {c}}\nlset L 0 2 X"
        assert _diag_with_code(source, "W231") == []

    def test_lset_nested_negative_still_flagged(self):
        # Plain negative literals are invalid at every nesting level.
        source = "set L {{a} {b}}\nlset L 0 -1 X"
        diags = _diag_with_code(source, "W231")
        assert len(diags) == 1

    def test_lset_inside_proc_ignores_outer_set(self):
        # Copilot scope guard: a `set L {...}` at the top level must
        # NOT be used to infer the list length of an `lset` inside a
        # proc body, because the proc's `L` is a different variable.
        source = """
set L {a b}
proc f {} {
    lset L end+5 X
}
"""
        # Only plain-negative integers would fire in that proc;
        # `end+5` cannot be length-checked because scope-guard
        # rejects the outer `set`.
        assert _diag_with_code(source, "W231") == []


# W232: string index out of range


class TestStringIndexOutOfRange:
    def test_string_index_positive_overshoot(self):
        diags = _diag_with_code("string index abc 10", "W232")
        assert len(diags) == 1
        assert "string has 3 characters" in diags[0].message

    def test_string_index_negative_literal(self):
        diags = _diag_with_code("string index abc -1", "W232")
        assert len(diags) == 1
        assert "negative" in diags[0].message

    def test_string_index_end_minus_underflow(self):
        diags = _diag_with_code("string index abc end-5", "W232")
        assert len(diags) == 1
        assert "resolves to -3" in diags[0].message

    def test_string_index_in_range_clean(self):
        assert _diag_with_code("string index abc 0", "W232") == []
        assert _diag_with_code("string index abc 2", "W232") == []
        assert _diag_with_code("string index abc end", "W232") == []
        assert _diag_with_code("string index abc end-2", "W232") == []

    def test_string_range_empty_slice(self):
        diags = _diag_with_code("string range abc end-99 -50", "W232")
        assert len(diags) == 1
        assert "slice is empty" in diags[0].message

    def test_string_range_both_above(self):
        diags = _diag_with_code("string range abc 10 20", "W232")
        assert len(diags) == 1

    def test_string_range_single_out_clean(self):
        # One index clamps, result non-empty.
        assert _diag_with_code("string range abc 0 100", "W232") == []
        assert _diag_with_code("string range abc -5 2", "W232") == []

    def test_string_replace_no_op(self):
        diags = _diag_with_code("string replace abc -1 -1 X", "W232")
        assert len(diags) == 1
        assert "no-op" in diags[0].message

    def test_string_insert_negative_literal(self):
        diags = _diag_with_code("string insert abc -5 X", "W232")
        assert len(diags) == 1
        assert "negative" in diags[0].message

    def test_string_insert_overshoot_not_flagged(self):
        # Positive overshoot always appends — sensible, no warning.
        assert _diag_with_code("string insert abc 100 X", "W232") == []

    def test_string_index_dynamic_string_with_negative_literal(self):
        # Even with a dynamic string, a plain negative literal is always bogus.
        diags = _diag_with_code("string index $s -1", "W232")
        assert len(diags) == 1

    def test_string_index_dynamic_both_skipped(self):
        assert _diag_with_code("string index $s $i", "W232") == []

    def test_string_index_unbraced_esc_not_length_checked(self):
        # Copilot: ESC literals can contain backslash escapes whose
        # source length differs from runtime length.  We only use the
        # length of braced (STR) literals; for unbraced forms we
        # cannot reliably compare against the runtime string length,
        # so length-based overshoot checks must not fire.
        # Here the source is 4 chars (`\\n` counts as 2), but Tcl sees
        # a 3-char string.  An index of 3 is still out of range at
        # runtime, but we cannot prove it from the source.
        assert _diag_with_code(r"string index a\nb 3", "W232") == []

    def test_string_index_braced_length_checked(self):
        # Braced literal: no backslash processing, so length is
        # authoritative.
        diags = _diag_with_code("string index {abc} 5", "W232")
        assert len(diags) == 1


# W240: loop condition is constant false


class TestLoopConstantFalse:
    def test_while_zero(self):
        diags = _diag_with_code("while {0} { puts hi }", "W240")
        assert len(diags) == 1
        assert diags[0].severity is Severity.WARNING
        assert "never executes" in diags[0].message

    def test_while_false_keyword(self):
        diags = _diag_with_code("while {false} { puts hi }", "W240")
        assert len(diags) == 1

    def test_for_constant_false_cond(self):
        diags = _diag_with_code(
            "for {set i 0} {0} {incr i} { puts hi }",
            "W240",
        )
        assert len(diags) == 1

    def test_while_true_no_W240(self):
        assert _diag_with_code("while {1} { break }", "W240") == []


# W241: provably-infinite loop


class TestLoopProvablyInfinite:
    def test_while_true_no_break(self):
        diags = _diag_with_code("while {1} { puts hi }", "W241")
        assert len(diags) == 1
        assert "provably infinite" in diags[0].message

    def test_while_true_with_break_clean(self):
        diags = _diag_with_code("while {1} { if {$x} { break } }", "W241")
        assert diags == []

    def test_while_true_with_return_clean(self):
        diags = _diag_with_code("while {1} { if {$x} { return 1 } }", "W241")
        assert diags == []

    def test_for_step_zero(self):
        diags = _diag_with_code(
            "for {set i 0} {$i < 10} {incr i 0} { puts hi }",
            "W241",
        )
        assert len(diags) == 1
        assert "step is zero" in diags[0].message

    def test_for_wrong_direction_negative_step(self):
        diags = _diag_with_code(
            "for {set i 0} {$i < 10} {incr i -1} { puts hi }",
            "W241",
        )
        assert len(diags) == 1
        assert "never reached" in diags[0].message

    def test_for_wrong_direction_positive_step(self):
        diags = _diag_with_code(
            "for {set i 10} {$i > 0} {incr i 2} { puts hi }",
            "W241",
        )
        assert len(diags) == 1

    def test_for_not_equal_skip_bound(self):
        # $i != 10 with step 3 starting at 0: 0, 3, 6, 9, 12, ... never equals 10.
        diags = _diag_with_code(
            "for {set i 0} {$i != 10} {incr i 3} { puts hi }",
            "W241",
        )
        assert len(diags) == 1
        assert "never exactly equals 10" in diags[0].message

    def test_for_normal_loop_clean(self):
        diags = _diag_with_code(
            "for {set i 0} {$i < 10} {incr i} { puts $i }",
            "W241",
        )
        assert diags == []

    def test_for_with_break_clean(self):
        diags = _diag_with_code(
            "for {set i 0} {$i < 10} {incr i -1} { if {$x} { break } }",
            "W241",
        )
        assert diags == []

    def test_for_body_mutates_counter_clean(self):
        # Body mutates counter, so step-direction heuristic is inapplicable.
        diags = _diag_with_code(
            "for {set i 0} {$i < 10} {incr i -1} { incr i 2 }",
            "W241",
        )
        assert diags == []

    def test_for_compound_condition_not_flagged(self):
        # Codex P1: a compound condition containing ``&&`` should NOT
        # be reduced to the simple counter form — doing so would
        # prove infinity from a subexpression and miss the other
        # conjunct.  Even though the inner ``$i < 10`` with negative
        # step *looks* infinite, ``&& 0`` makes the whole condition
        # constant-false, so no W241.
        diags = _diag_with_code(
            "for {set i 0} {$i < 10 && 0} {incr i -1} { puts hi }",
            "W241",
        )
        assert diags == []

    def test_for_compound_condition_with_or_not_flagged(self):
        # ``||`` with a subexpression that looks infinite should also
        # not fire: the other disjunct could still end the loop.
        diags = _diag_with_code(
            "for {set i 0} {$i < 10 || $done} {incr i -1} { puts hi }",
            "W241",
        )
        assert diags == []


# W242: loop termination cannot be proven (opt-in HINT)


class TestLoopTerminationUnprovable:
    def test_while_condition_var_never_modified(self):
        diags = _diag_with_code("while {$x < 10} { puts hi }", "W242")
        assert len(diags) == 1
        assert diags[0].severity is Severity.HINT
        assert "cannot be proven" in diags[0].message

    def test_while_condition_var_modified_clean(self):
        diags = _diag_with_code("while {$x < 10} { incr x }", "W242")
        assert diags == []

    def test_while_var_set_inside_clean(self):
        diags = _diag_with_code("while {$x} { set x 0 }", "W242")
        assert diags == []

    def test_while_constant_true_no_W242(self):
        # W241 fires instead.
        assert _diag_with_code("while {1} { puts hi }", "W242") == []

    def test_while_constant_false_no_W242(self):
        # W240 fires instead.
        assert _diag_with_code("while {0} { puts hi }", "W242") == []

    def test_for_counter_not_modified(self):
        diags = _diag_with_code(
            "for {set i 0} {$j < 10} {incr i} { puts hi }",
            "W242",
        )
        assert len(diags) == 1
