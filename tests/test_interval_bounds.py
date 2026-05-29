"""Interval-driven dynamic bounds checks (Phase 3).

These cover the *dynamic* index cases the syntactic checks in
``analyser/checks/_bounds.py`` deliberately skip: a ``$var`` index whose
:mod:`compiler.intervals` range proves the access is out of range against a
statically-known container length.

Every positive/negative pairing is grounded in Tcl 9.0.3 behaviour:

* ``lindex {a b c} 5`` / ``-2`` → ``""`` (silent, no error)  → W230 smell.
* ``lset l 4 x`` on a length-3 list → ``index "4" out of range`` → W231.
* ``lset l 3 x`` on a length-3 list → legal *append* (``a b c x``) → silent.
* ``lindex $l $j`` with ``0 <= j < [llength $l]`` (idiom) → in range → silent.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser import analyse


def _codes(source: str, code: str):
    return [d for d in analyse(source).diagnostics if d.code == code]


class TestDynamicLindexOutOfRange:
    """W230 — silent out-of-range lindex with a proven ``$var`` index."""

    def test_literal_list_loop_index_past_end_fires(self):
        # j ∈ [3, 9] (init 3, bounded by < 10), list length 3 → always past end.
        src = "proc f {} { for {set j 3} {$j < 10} {incr j} { set x [lindex {a b c} $j] } }"
        diags = _codes(src, "W230")
        assert len(diags) == 1
        assert "$j" in diags[0].message

    def test_const_index_past_end_fires(self):
        # SCCP proves j == 5; list length 3.
        src = "proc f {} { set j 5\n set x [lindex {a b c} $j] }"
        assert len(_codes(src, "W230")) == 1

    def test_tracked_list_var_length_fires(self):
        # `[list a b]` tracks length 2; j == 5 is past the end.
        src = "proc f {} { set l [list a b]\n set j 5\n set x [lindex $l $j] }"
        assert len(_codes(src, "W230")) == 1

    def test_idiomatic_loop_is_silent(self):
        # `$j < [llength $l]` — the canonical in-range iteration; must not warn.
        src = "proc f {l} { for {set j 0} {$j < [llength $l]} {incr j} { set x [lindex $l $j] } }"
        assert _codes(src, "W230") == []

    def test_in_range_const_index_silent(self):
        src = "proc f {} { set j 1\n set x [lindex {a b c} $j] }"
        assert _codes(src, "W230") == []

    def test_unknown_param_index_silent(self):
        # A param index has no bound (TOP) — not provably out of range.
        src = "proc f {i} { set x [lindex {a b c} $i] }"
        assert _codes(src, "W230") == []

    def test_conditional_length_phi_silent(self):
        # The list length is a merge of 2 and 4 (unknown) — must not warn even
        # though j == 5 exceeds both, because the version's length is unproven.
        src = (
            "proc f {c} {\n"
            "    if {$c} { set l {a b} } else { set l {a b c d} }\n"
            "    set j 5\n"
            "    set x [lindex $l $j]\n"
            "}"
        )
        assert _codes(src, "W230") == []

    def test_no_double_fire_with_syntactic_literal_index(self):
        # Literal index is handled by the syntactic check; the interval check
        # must not add a second W230.
        assert len(_codes("lindex {a b c} 5", "W230")) == 1


class TestNestedPositionLindex:
    """The index op nested in another command's arg / a return value — the
    common shapes (`puts [lindex …]`, `lappend out [lindex …]`,
    `return [lindex …]`) the command-substitution intent does not reach."""

    def test_puts_nested_fires(self):
        src = "proc f {} { set j 5\n puts [lindex {a b} $j] }"
        assert len(_codes(src, "W230")) == 1

    def test_lappend_nested_fires(self):
        src = "proc f {} { set j 5\n set r {}\n lappend r [lindex {a b} $j] }"
        assert len(_codes(src, "W230")) == 1

    def test_return_value_fires(self):
        src = "proc f {} { set j 5\n return [lindex {a b} $j] }"
        assert len(_codes(src, "W230")) == 1

    def test_nested_idiomatic_silent(self):
        src = "proc f {l} { for {set j 0} {$j < [llength $l]} {incr j} { puts [lindex $l $j] } }"
        assert _codes(src, "W230") == []

    def test_nested_in_range_silent(self):
        src = "proc f {} { set j 1\n puts [lindex {a b c} $j] }"
        assert _codes(src, "W230") == []

    def test_expr_embedded_fires(self):
        # `[lindex …]` as a command substitution inside an expr body.
        src = (
            "proc f {} { set l {a b c}\n set i 5\n set u [expr {[lindex $l $i] + 1}]\n return $u }"
        )
        assert len(_codes(src, "W230")) == 1

    def test_branch_condition_fires(self):
        src = "proc f {} { set l {a b}\n set i 5\n while {[lindex $l $i]} { break } }"
        assert len(_codes(src, "W230")) == 1

    def test_expr_embedded_in_range_silent(self):
        src = (
            "proc f {} { set l {a b c}\n set i 1\n set u [expr {[lindex $l $i] + 1}]\n return $u }"
        )
        assert _codes(src, "W230") == []


class TestDynamicLsetOutOfRange:
    """W231 — lset with a proven out-of-range ``$var`` index (runtime error)."""

    def test_loop_index_past_append_slot_fires(self):
        # j ∈ [4, 8]; list length 3 → index > length (3) on every iteration.
        src = "proc f {v} { set l {a b c}\n for {set j 4} {$j < 9} {incr j} { lset l $j $v } }"
        diags = _codes(src, "W231")
        assert len(diags) == 1
        assert "$j" in diags[0].message

    def test_append_slot_is_silent(self):
        # index == length is the legal append slot for lset; must not warn.
        src = "proc f {v} { set l {a b c}\n set j 3\n lset l $j $v }"
        assert _codes(src, "W231") == []

    def test_in_range_silent(self):
        src = "proc f {v} { set l {a b c}\n set j 1\n lset l $j $v }"
        assert _codes(src, "W231") == []


class TestDynamicStringIndex:
    """W232 — `string index $s $i` with a proven out-of-range `$var` index,
    against a tracked string length (character count).  tclsh returns "" silently
    for an out-of-range string index (verified), so this is a smell like W230."""

    def test_past_end_fires(self):
        src = 'proc f {} { set s "hello"\n set i 10\n return [string index $s $i] }'
        assert len(_codes(src, "W232")) == 1

    def test_index_equals_length_fires(self):
        # idx == length is out of range for string index (returns "").
        src = 'proc f {} { set s "hello"\n set i 5\n return [string index $s $i] }'
        assert len(_codes(src, "W232")) == 1

    def test_in_range_silent(self):
        src = 'proc f {} { set s "hello"\n set i 2\n return [string index $s $i] }'
        assert _codes(src, "W232") == []

    def test_unknown_silent(self):
        src = "proc f {s i} { return [string index $s $i] }"
        assert _codes(src, "W232") == []

    def test_quoted_escape_uses_resolved_char_length(self):
        # `"a\nb"` is 3 Tcl chars (backslash escape resolved), so index 3 is out
        # of range and index 2 is valid.  tclsh 9.0.3: `string index "a\nb" 3`
        # is "" (out of range), `string length "a\nb"` is 3.
        oob = 'proc f {} { set s "a\\nb"\n set i 3\n return [string index $s $i] }'
        assert len(_codes(oob, "W232")) == 1
        ok = 'proc f {} { set s "a\\nb"\n set i 2\n return [string index $s $i] }'
        assert _codes(ok, "W232") == []

    def test_braced_escape_keeps_source_length(self):
        # `{a\nb}` is a *braced* literal — no backslash processing — so it is 4
        # Tcl chars and index 3 is in range (tclsh: `string length {a\nb}` is 4).
        src = "proc f {} { set s {a\\nb}\n set i 3\n return [string index $s $i] }"
        assert _codes(src, "W232") == []

    def test_unicode_escape_char_length(self):
        # `ÿ` is one char, so `"xÿy"` is 3 chars; index 3 is out of range.
        src = 'proc f {} { set s "x\\u00ffy"\n set i 3\n return [string index $s $i] }'
        assert len(_codes(src, "W232")) == 1


class TestDivideByZero:
    """W233 — division / modulo by a provably-zero divisor (a runtime error in
    tclsh; ``expr {1/0}`` and ``expr {5%0}`` both raise 'divide by zero')."""

    def test_literal_div_zero_fires(self):
        assert len(_codes("proc f {} { return [expr {1 / 0}] }", "W233")) == 1

    def test_literal_mod_zero_fires(self):
        assert len(_codes("proc f {} { return [expr {5 % 0}] }", "W233")) == 1

    def test_const_var_divisor_fires(self):
        src = "proc f {} { set d 0\n return [expr {10 / $d}] }"
        assert len(_codes(src, "W233")) == 1

    def test_zero_excluding_guard_silent(self):
        # `if {$d != 0}` makes the division unreachable when d is provably 0
        # (SCCP prunes the dead branch) — must not warn.
        src = "proc f {} { set d 0\n if {$d != 0} { return [expr {1 / $d}] }\n return safe }"
        assert _codes(src, "W233") == []

    def test_nonzero_divisor_silent(self):
        src = "proc f {} { set d 3\n return [expr {10 / $d}] }"
        assert _codes(src, "W233") == []

    def test_unknown_divisor_silent(self):
        # A param divisor has no bound — not provably zero.
        src = "proc f {n} { return [expr {10 / $n}] }"
        assert _codes(src, "W233") == []

    def test_subtraction_to_zero_fires(self):
        # The interval domain proves $x - $x == 0 when x is constant.
        src = "proc f {} { set x 5\n return [expr {10 / ($x - $x)}] }"
        assert len(_codes(src, "W233")) == 1

    # Tcl `expr` is lazy: the dead arm of `?:` and the short-circuited operand
    # of `&&`/`||` never execute, so a `1/0` there must NOT fire W233.  Verified
    # in tclsh 9.0.3: `expr {0 ? 1/0 : 7}` → 7, `expr {1 || 1/0}` → 1,
    # `expr {0 && 1/0}` → 0 (all complete without a divide-by-zero error).
    def test_dead_ternary_arm_div_zero_silent(self):
        assert _codes("proc f {} { return [expr {0 ? 1/0 : 7}] }", "W233") == []

    def test_short_circuit_or_div_zero_silent(self):
        assert _codes("proc f {} { return [expr {1 || 1/0}] }", "W233") == []

    def test_short_circuit_and_div_zero_silent(self):
        assert _codes("proc f {} { return [expr {0 && 1/0}] }", "W233") == []

    def test_eager_left_of_and_div_zero_fires(self):
        # The left operand of `&&` is always evaluated — `expr {1/0 && 1}`
        # errors in tclsh, so this must still fire.
        assert len(_codes("proc f {} { return [expr {1/0 && 1}] }", "W233")) == 1

    # A lazy arm whose guard is a *compile-time constant* is FORCED to run, so a
    # `1/0` there is a guaranteed runtime error and must fire.  Verified in tclsh
    # 9.0.3: `expr {1 && 1/0}`, `expr {0 || 1/0}`, `expr {1 ? 1/0 : 7}` all raise
    # `divide by zero`.  (Complements the dead-arm suppressions above.)
    def test_forced_and_rhs_div_zero_fires(self):
        # LHS is constant-true, so the && RHS is always evaluated.
        assert len(_codes("proc f {} { return [expr {1 && 1/0}] }", "W233")) == 1

    def test_forced_or_rhs_div_zero_fires(self):
        # LHS is constant-false, so the || RHS is always evaluated.
        assert len(_codes("proc f {} { return [expr {0 || 1/0}] }", "W233")) == 1

    def test_forced_true_ternary_arm_div_zero_fires(self):
        # Condition is constant-true, so the then-arm is always evaluated.
        assert len(_codes("proc f {} { return [expr {1 ? 1/0 : 7}] }", "W233")) == 1

    def test_forced_false_ternary_arm_div_zero_fires(self):
        # Condition is constant-false, so the else-arm is always evaluated.
        assert len(_codes("proc f {} { return [expr {0 ? 7 : 1/0}] }", "W233")) == 1

    def test_bool_keyword_guard_forces_arm(self):
        # `true`/`false`/`yes`/`no`/`on`/`off` are constant guards too.
        assert len(_codes("proc f {} { return [expr {true && 1/0}] }", "W233")) == 1
        assert len(_codes("proc f {} { return [expr {no || 1/0}] }", "W233")) == 1

    def test_nonconstant_guard_arm_stays_silent(self):
        # A *non-constant* guard leaves the arm maybe-dead — no guaranteed error,
        # so W233 must stay silent (preserves the dead-arm discipline).
        assert _codes("proc f {c} { return [expr {$c && 1/0}] }", "W233") == []
        assert _codes("proc f {c} { return [expr {$c || 1/0}] }", "W233") == []
        assert _codes("proc f {c} { return [expr {$c ? 1/0 : 7}] }", "W233") == []


class TestUnreachableBoundsSuppressed:
    """Dynamic-bounds findings must not fire from statically unreachable blocks
    (SCCP marks them dead) — the same reachability discipline as divide-by-zero.
    Regression for find_interval_bounds walking every block."""

    def test_if_false_branch_no_w230(self):
        src = "proc f {} { if {0} { set i 5\n set x [lindex {a b} $i] } }"
        assert _codes(src, "W230") == []

    def test_unreachable_else_branch_no_w230(self):
        # cond is constant-true, so the else arm is unreachable.
        src = "proc f {} { if {1} { set y 1 } else { set i 5\n set x [lindex {a b} $i] } }"
        assert _codes(src, "W230") == []

    def test_reachable_control_still_fires(self):
        # Same out-of-range access in live code must still warn.
        src = "proc f {} { set i 5\n set x [lindex {a b} $i] }"
        assert len(_codes(src, "W230")) == 1


class TestListExpansionLength:
    """[list {*}{...}] expands at runtime, so its element count is not the arg
    count.  Regression for _list_length_map counting `{*}{a b}` as length 1 and
    wrongly flagging a valid index."""

    def test_list_expansion_does_not_false_fire(self):
        # l = {a b} (length 2 after expansion); index 1 is valid.
        src = "proc f {} { set l [list {*}{a b}]\n set i 1\n set x [lindex $l $i] }"
        assert _codes(src, "W230") == []

    def test_plain_list_length_still_inferred(self):
        # Control: no expansion → length 2 is known, index 5 is out of range.
        src = "proc g {} { set l [list a b]\n set i 5\n set x [lindex $l $i] }"
        assert len(_codes(src, "W230")) == 1


class TestIntervalFixpointSoundness:
    """If the interval fixpoint does not converge within the iteration cap, the
    result may be narrower than reality (still ascending) — reporting a finding
    from it would be an unsound false positive.  The unconverged result must
    degrade to TOP so no W230/W231/W232/div-zero finding is derived."""

    def test_unconverged_fixpoint_suppresses_findings(self, monkeypatch):
        import compiler.intervals as iv

        # Force non-convergence (a single iteration never stabilises a loop phi).
        monkeypatch.setattr(iv, "_MAX_ITERS", 1)
        # Without the guard this fires W230 (j proven past the end of a 3-list).
        src = "proc f {} { for {set j 3} {$j < 10} {incr j} { set x [lindex {a b c} $j] } }"
        assert _codes(src, "W230") == []

    def test_unconverged_compute_intervals_are_all_top(self, monkeypatch):
        import compiler.intervals as iv
        from compiler.compilation_unit import compile_source

        monkeypatch.setattr(iv, "_MAX_ITERS", 1)
        cu = compile_source("proc f {} { set i 0\n while {$i < 5} { incr i }\n return $i }")
        fu = cu.procedures["::f"]
        intervals = iv.compute_intervals(fu.cfg, fu.ssa, fu.analysis.values)
        assert intervals and all(itv.is_top for itv in intervals.values())
