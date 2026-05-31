"""TP/FP regression tests for the BND (bounds / intervals) family.

Each test pairs to an ``FP-BND-NN`` entry in
``docs/design/compiler/FP.md``.  The Tcl reproducer string is **copied verbatim**
from the doc — if either side drifts the test will visibly catch it.

BND family covers W230 / W231 / W232 / W233 dynamic-bounds determinations
driven by the Phase-3 interval domain.  ``tests/test_interval_bounds.py`` is
the broader corpus (~30 tests covering edge cases — escape sequences, Unicode,
guard narrowing, etc.); these FP-BND tests are the curated must-keep verdicts
that live in lock-step with the FP.md catalog.

Ground truth: real C tclsh 9.0.3.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from server.features.diagnostics import get_diagnostics


def _codes(source: str, code: str):
    return [d for d in get_diagnostics(source) if d.code == code]


# FP-BND-01 — W231 dynamic loop-index past append-slot fires


FP_BND_01_REPRO = """\
proc f {v} {
    # j is bounded [4, 8]; list length 3 -> every iteration is OOR.
    set l {a b c}
    for {set j 4} {$j < 9} {incr j} { lset l $j $v }
}
"""


def test_FP_BND_01_loop_index_past_append_slot_fires():
    """TP: j ∈ [4, 8], list length 3 -> every iteration is out-of-range; W231
    must fire (interval domain proves the OOR)."""
    diags = _codes(FP_BND_01_REPRO, "W231")
    assert diags, "loop index past append slot must fire W231"
    assert "$j" in diags[0].message


# FP-BND-02 — W231 append-slot ($j == length) IS silent


FP_BND_02_REPRO = """\
proc f {v} {
    # j == length -> APPEND slot; must NOT fire W231.
    set l {a b c}
    set j 3
    lset l $j $v
}
"""


def test_FP_BND_02_dynamic_append_slot_silent():
    """FP guard: dynamic append slot (`j == length`) must NOT fire — the
    dynamic-index path uses `> length` (not `>= length`), mirroring FP-NAB-01."""
    assert _codes(FP_BND_02_REPRO, "W231") == []


def test_FP_BND_02_in_range_dynamic_silent():
    """FP control: a clearly-in-range dynamic index must NOT fire."""
    src = "proc f {v} { set l {a b c}\n set j 1\n lset l $j $v }"
    assert _codes(src, "W231") == []


# FP-BND-03 — W232 string index past end fires


FP_BND_03_REPRO = """\
proc f {} {
    # i (10) > string length (5) -> tclsh returns ""; W232 smell.
    set s "hello"
    set i 10
    return [string index $s $i]
}
"""


def test_FP_BND_03_string_index_past_end_fires():
    """TP: `string index $s 10` on a 5-char string is past-end; tclsh returns
    "" silently, so this is a smell-tier W232 (not an error-tier diagnostic)."""
    assert _codes(FP_BND_03_REPRO, "W232")


def test_FP_BND_03_idx_equals_length_fires():
    """TP: idx == length is also out-of-range for `string index` (returns "")."""
    src = 'proc f {} { set s "hello"\n set i 5\n return [string index $s $i] }'
    assert _codes(src, "W232")


def test_FP_BND_03_in_range_silent():
    """FP control: an in-range string index must NOT fire."""
    src = 'proc f {} { set s "hello"\n set i 2\n return [string index $s $i] }'
    assert _codes(src, "W232") == []


def test_FP_BND_03_unknown_string_silent():
    """FP control: unknown string + unknown index is not provable OOR."""
    src = "proc f {s i} { return [string index $s $i] }"
    assert _codes(src, "W232") == []


# FP-BND-04 — W233 division by provably-zero divisor fires


FP_BND_04_REPRO = """\
proc f {} {
    # $d == 0 (CONST) -> tclsh divide-by-zero; W233 fires.
    set d 0
    return [expr {10 / $d}]
}
"""


def test_FP_BND_04_const_zero_divisor_fires():
    """TP: a constant-zero variable divisor proves divide-by-zero."""
    assert _codes(FP_BND_04_REPRO, "W233")


def test_FP_BND_04_literal_div_zero_fires():
    """TP: a literal `1/0` is also a divide-by-zero (the simplest case)."""
    assert _codes("proc f {} { return [expr {1 / 0}] }", "W233")


def test_FP_BND_04_literal_mod_zero_fires():
    """TP: modulo by zero is the same runtime error."""
    assert _codes("proc f {} { return [expr {5 % 0}] }", "W233")


def test_FP_BND_04_nonzero_divisor_silent():
    """FP control: a non-zero const divisor must NOT fire."""
    src = "proc f {} { set d 3\n return [expr {10 / $d}] }"
    assert _codes(src, "W233") == []


def test_FP_BND_04_unknown_divisor_silent():
    """FP control: an unknown param divisor isn't provably zero."""
    src = "proc f {n} { return [expr {10 / $n}] }"
    assert _codes(src, "W233") == []


# FP-BND-05 — W233 short-circuit / dead arm is silent


FP_BND_05_REPRO = """\
proc f {} {
    # `0 ? 1/0 : 7` -> dead arm; tclsh returns 7; W233 must NOT fire.
    return [expr {0 ? 1/0 : 7}]
}
"""


def test_FP_BND_05_dead_ternary_arm_silent():
    """FP guard: the dead arm of a ternary is unevaluated by tclsh (verified:
    `expr {0 ? 1/0 : 7}` returns 7), so W233 must NOT fire."""
    assert _codes(FP_BND_05_REPRO, "W233") == []


def test_FP_BND_05_short_circuit_or_silent():
    """FP guard: `expr {1 || 1/0}` short-circuits before evaluating 1/0
    (tclsh-verified: returns 1)."""
    src = "proc f {} { return [expr {1 || 1/0}] }"
    assert _codes(src, "W233") == []


def test_FP_BND_05_short_circuit_and_silent():
    """FP guard: `expr {0 && 1/0}` short-circuits before evaluating 1/0
    (tclsh-verified: returns 0)."""
    src = "proc f {} { return [expr {0 && 1/0}] }"
    assert _codes(src, "W233") == []


def test_FP_BND_05_guard_excludes_zero_silent():
    """FP control: `if {$d != 0}` guard makes the division unreachable when
    d is provably 0 (SCCP prunes the dead branch)."""
    src = "proc f {} { set d 0\n if {$d != 0} { return [expr {1 / $d}] }\n return safe }"
    assert _codes(src, "W233") == []


# FP-BND-06 — W233 fires when a NON-INTEGER constant guard forces the lazy arm
#
# The forced-arm walk resolves the guard with the same constant-truth engine as
# the W123 dead-arm check (`expr_ast._const_bool`), so a float, a case-
# insensitive boolean keyword, or a unary-prefixed constant is recognised as a
# constant guard — not just a bare integer literal.  tclsh 9.0.3 raises "divide
# by zero" for every must-fire case below; the FP controls short-circuit the
# `1/0` away and stay silent.


def test_FP_BND_06_float_guard_forces_arm_fires():
    """TP: `1.0`/`0.0`/`1.5` are constant-true|false guards — the forced arm's
    `1/0` is a guaranteed divide-by-zero."""
    assert _codes("proc f {} { return [expr {1.0 && 1/0}] }", "W233")
    assert _codes("proc f {} { return [expr {0.0 || 1/0}] }", "W233")
    assert _codes("proc f {} { return [expr {1.5 ? 1/0 : 7}] }", "W233")


def test_FP_BND_06_uppercase_bool_guard_forces_arm_fires():
    """TP: Tcl `expr` accepts booleans case-insensitively, so `True`/`TRUE`/`No`
    are constant guards (a capitalised bool no longer degrades to opaque raw)."""
    assert _codes("proc f {} { return [expr {True && 1/0}] }", "W233")
    assert _codes("proc f {} { return [expr {TRUE && 1/0}] }", "W233")
    assert _codes("proc f {} { return [expr {No || 1/0}] }", "W233")


def test_FP_BND_06_unary_constant_guard_forces_arm_fires():
    """TP: unary `-`/`+` keep the operand's truth (`-1` true), `!`/`not` invert
    it (`!0` true) — both constant, so the forced arm fires."""
    assert _codes("proc f {} { return [expr {-1 && 1/0}] }", "W233")
    assert _codes("proc f {} { return [expr {!0 && 1/0}] }", "W233")


def test_FP_BND_06_false_constant_guard_short_circuits_silent():
    """FP control: `False`/`!1`/`0.0 &&` are constant-FALSE — the `&&` RHS is
    short-circuited away, so no guaranteed error (tclsh returns false)."""
    assert _codes("proc f {} { return [expr {False && 1/0}] }", "W233") == []
    assert _codes("proc f {} { return [expr {!1 && 1/0}] }", "W233") == []
    assert _codes("proc f {} { return [expr {0.0 && 1/0}] }", "W233") == []


def test_FP_BND_06_nonconstant_guard_silent():
    """FP control: a *non-constant* guard leaves the arm maybe-dead — no
    guaranteed error, so W233 must stay silent."""
    assert _codes("proc f {c} { return [expr {$c && 1/0}] }", "W233") == []
