"""Ground-truth True-Negative / True-Positive / False-Negative regression tests.

Every case here was driven through real C ``tclsh9.0`` during authoring;
the ``runtime:`` line in each docstring records exactly what tclsh did
when the snippet was executed.  The tests then lock in the analyser's
current verdict against that ground truth:

* **TN** (true negative) — the analyser is correctly silent; tclsh
  shows the construct runs without error.  Locked in to prevent a
  future false-positive regression.
* **TP** (true positive) — the analyser correctly fires; tclsh either
  errors at runtime (a real bug) or executes a code-smell pattern that
  the diagnostic is designed to flag (e.g. W220 dead-store always
  succeeds at runtime — the smell is the wasted assignment).
* **FN** (false negative) — the analyser stays silent on a snippet
  that tclsh proves wrong at runtime.  Marked ``@pytest.mark.xfail
  (strict=True)`` so the test flips to a failure (prompting its own
  removal) the moment the precision gap is closed.

These cases are deliberately minimal but represent real distillations
of patterns found in tcllib / tklib / Tcl 9.0 stdlib code.  The
``runtime:`` lines were captured at authoring time using ``tclsh9.0``
from the project's ``tmp/tcl9.0.3/`` build.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest

from server.features.diagnostics import get_diagnostics


def _codes(source: str, *codes: str) -> list[str]:
    """All firings of *codes* on *source* (returns the per-firing code list)."""
    wanted = set(codes)
    return [d.code for d in get_diagnostics(source) if d.code in wanted]


def _any(source: str, *codes: str) -> bool:
    return bool(_codes(source, *codes))


# =====================================================================
# Section 1 — True Negatives: analyser correctly silent.
# Each ``runtime:`` line was captured from real tclsh9.0.
# =====================================================================


def test_TN_foreach_lappend_accumulator_no_shimmer():
    """The canonical Tcl accumulator idiom: ``set r {}`` then per-iteration
    ``lappend r $x``.  ``{}`` is the typeless empty value, ``lappend``
    promotes it to a list once on the first iteration — that's a one-time
    intrep transition, not per-iteration shimmer.  S100/S101/S102 must
    stay silent.

    runtime: ``puts [f]`` -> ``3``  (no error)
    """
    src = "proc f {} {\n    set r {}\n    foreach x {1 2 3} { lappend r $x }\n    return [llength $r]\n}\n"
    assert not _any(src, "S100", "S101", "S102")


def test_TN_info_exists_guards_read():
    """``if {[info exists v]} { return $v }`` — the canonical
    test-before-use idiom.  The body is reachable only when ``v`` is
    defined, so reading ``$v`` is safe.  Must not fire W210.

    runtime: ``puts [f]`` -> empty line  (no error)
    """
    src = "proc f {} {\n    if {[info exists v]} { return $v }\n    return {}\n}\n"
    assert not _any(src, "W210")


def test_TN_catch_result_var_always_defined():
    """``catch {body} msg`` ALWAYS writes ``msg``: the result string on
    success, the error message on failure.  Reading ``$msg`` after a
    bare ``catch`` is safe.

    runtime: ``puts [f]`` -> ``1``  (no error)
    """
    src = "proc f {} {\n    catch {set x 1} msg\n    return $msg\n}\n"
    assert not _any(src, "W210")


def test_TN_uplevel_alias_writes_caller_local():
    """A callee that takes ``upvar 1 <name> alias`` defines the
    caller-side variable.  ``init`` writes ``$v`` in caller's frame;
    reading ``$v`` after ``init`` is safe.

    runtime: ``puts [f]`` -> ``42``  (no error)
    """
    src = (
        "proc init {} {\n    upvar 1 v out\n    set out 42\n}\n"
        "proc f {} {\n    init\n    return $v\n}\n"
    )
    assert not _any(src, "W210")


def test_TN_dict_with_known_dict_literal():
    """``dict with d {body}`` exposes each key in ``d`` as a local
    inside ``body``.  When ``d`` is a known literal containing ``a``
    and ``b``, reading ``$a`` inside the body is safe.

    runtime: ``puts [f]`` -> ``ok``  (no error)
    """
    src = "proc f {} {\n    set d {a 1 b 2}\n    dict with d { puts $a }\n    return ok\n}\n"
    assert not _any(src, "W210")


def test_TN_incr_on_unset_var_is_legal_tcl85plus():
    """In Tcl 8.5+ ``incr z`` initialises ``z`` to 0 if unset.  Reading
    ``$z`` after a bare ``incr z`` is safe.  Must NOT fire W210/W213.

    runtime: ``puts [f]`` -> ``1``  (no error)
    """
    src = "proc f {} {\n    incr z\n    return $z\n}\n"
    assert not _any(src, "W210", "W213")


def test_TN_lset_append_slot_at_len_is_legal():
    """``lset l N v`` where ``N == llength``$l is the documented
    append slot — appends ``v`` to the list.  Only ``N > llength`` is
    out-of-range.  Must NOT fire W231.

    runtime: ``puts [f]`` -> ``a b c X``  (no error)
    """
    src = "proc f {} {\n    set l {a b c}\n    lset l 3 X\n    return $l\n}\n"
    assert not _any(src, "W231")


def test_TN_lrange_clamps_last_index():
    """``lrange`` clamps an out-of-range last index to ``end``; this is
    the documented idiom for "take up to N more elements from here".
    Must NOT fire a bounds smell.

    runtime: ``puts [f]`` -> ``b c``  (no error)
    """
    src = "proc f {} { return [lrange {a b c} 1 100] }\n"
    assert not _any(src, "W230", "W231")


def test_TN_lindex_end_minus_k_in_range():
    """``lindex $l end-K`` where ``K < llength`` returns the K-th-from-
    last element.  In-range, must NOT fire W230.

    runtime: ``puts [f]`` -> ``b``  (no error)
    """
    src = "proc f {} { return [lindex {a b c} end-1] }\n"
    assert not _any(src, "W230")


def test_TN_args_is_auto_bound():
    """``args`` is the implicit catch-all parameter — always defined as
    a list (possibly empty).  Reading ``$args`` first thing must NOT
    fire W210.

    runtime: ``puts [f a b c]`` -> ``a b c``  (no error)
    """
    src = "proc f {args} { return $args }\n"
    assert not _any(src, "W210")


def test_TN_safe_eval_with_list_form():
    """``eval [list ...]`` is the documented safe idiom: ``list``
    builds a single, properly-quoted command, and ``eval`` runs it
    exactly once.  Must NOT fire W101/W105 ('double substitution').

    runtime: ``puts [f 1]`` -> ``1``  (no error)
    """
    src = "proc f {x} {\n    eval [list set y $x]\n    return $y\n}\n"
    assert not _any(src, "W101", "W105")


def test_TN_uplevel_single_pure_var_body():
    """``uplevel 1 $body`` where ``$body`` is a single pure-var
    reference is the safe single-substitution idiom (tclsh evaluates
    ``$body`` once in the target frame — no concat / no second
    substitution).  Must NOT fire W301.

    runtime: ``set x 0; f {set x 1}; puts $x`` -> ``1``  (no error)
    """
    src = "proc f {body} { uplevel 1 $body }\n"
    assert not _any(src, "W301")


def test_TN_for_loop_var_is_auto_defined():
    """The ``for {set i 0} ...`` form defines ``$i`` in the init clause;
    reading ``$i`` in the body / next is safe.

    runtime: ``puts [f]`` -> ``2``  (no error)
    """
    src = "proc f {} {\n    set last 0\n    for {set i 0} {$i < 3} {incr i} { set last $i }\n    return $last\n}\n"
    assert not _any(src, "W210")


def test_TN_foreach_multivar_unpacks_safely():
    """``foreach {k v} {a 1 b 2} { ... }`` binds both ``$k`` and ``$v``
    each iteration.  Reading either in the body is safe.

    runtime: ``puts [f]`` -> ``a=1,b=2``  (no error)
    """
    src = (
        "proc f {} {\n"
        "    set out {}\n"
        "    foreach {k v} {a 1 b 2} { lappend out $k=$v }\n"
        "    return [join $out ,]\n"
        "}\n"
    )
    assert not _any(src, "W210")


# =====================================================================
# Section 2 — True Positives: analyser correctly fires.
# Some codes (W220, W214, W211, W230, W232, S100..S102, W101, W105) are
# code smells — tclsh succeeds at runtime, but the diagnostic is still
# the right call.  Others (W231, W233, W307, W210) flag patterns that
# DO error at runtime.
# =====================================================================


def test_TP_W307_var_holds_literal_non_command():
    """``set cmd nope; $cmd arg`` dispatches a literal that is not a
    known command.  tclsh at runtime: ``invalid command name "nope"``.

    runtime: ``f`` -> ERROR ``invalid command name "nope"``
    """
    src = "proc f {} {\n    set cmd nope\n    $cmd arg\n}\n"
    assert _any(src, "W307")


def test_TP_W231_lset_index_out_of_range_errors():
    """``lset l 99 X`` on a 3-element list is genuinely out-of-range
    (not the append slot).  tclsh ERRORS at runtime.

    runtime: ``f`` -> ERROR ``list index out of range``
    """
    src = "proc f {} {\n    set l {a b c}\n    lset l 99 X\n}\n"
    assert _any(src, "W231")


def test_TP_W231_lset_end_on_empty_list_errors():
    """``lset {} end X`` — ``end`` on an empty list is out of range.
    tclsh ERRORS.

    runtime: ``catch f msg; puts $msg`` -> ``list index "end" out of range``
    """
    src = "proc f {} {\n    set l {}\n    lset l end X\n}\n"
    assert _any(src, "W231")


def test_TP_W233_divide_by_zero_proves_runtime_error():
    """The interval-bounds W233 fires when the divisor is provably
    ``[0,0]``.  tclsh ERRORS.

    runtime: ``f`` -> ERROR ``divide by zero``
    """
    src = "proc f {} { return [expr {10/0}] }\n"
    assert _any(src, "W233")


def test_TP_W210_array_element_read_before_any_set():
    """No prior write to ``arr(x)`` anywhere in the proc.  tclsh
    ERRORS at runtime.

    runtime: ``f`` -> ERROR ``can't read "arr(x)": no such variable``
    """
    src = "proc f {} { return $arr(x) }\n"
    assert _any(src, "W210")


def test_TP_W220_classic_dead_store_pre_overwrite():
    """``set x 1`` immediately followed by ``set x 2`` — the first
    store is observationally unreachable.  tclsh succeeds at runtime
    (the smell isn't a runtime error, it's wasted work).

    runtime: ``puts [f]`` -> ``2``  (no error; W220 is a smell)
    """
    src = "proc f {} {\n    set x 1\n    set x 2\n    return $x\n}\n"
    assert _any(src, "W220")


def test_TP_W214_unused_parameter_in_single_proc():
    """``proc lonely {a b} { puts $a }`` — ``b`` is genuinely unused;
    no peer family, no dispatcher evidence.  tclsh succeeds at
    runtime (the smell is API noise).

    runtime: ``lonely 1 2`` -> ``1``  (no error)
    """
    src = "proc lonely {a b} { puts $a }\n"
    assert _any(src, "W214")


def test_TP_W211_W220_array_element_only_written():
    """``set arr(x) 1`` with no subsequent read of any ``arr`` element.
    tclsh succeeds; the diagnostic flags a wasted store.

    runtime: ``puts [f]`` -> ``0``  (no error; smell only)
    """
    src = "proc f {} {\n    set arr(x) 1\n    return 0\n}\n"
    assert _any(src, "W211", "W220")


def test_TP_W101_W105_eval_concatenation_injection():
    """``eval "set x $input"`` — ``$input`` is concatenated into the
    eval'd string, which is parsed as Tcl source.  Any spaces /
    metachars in ``$input`` break or attack the parse.  tclsh runs
    fine for benign input but the construct is a known injection
    smell.

    runtime: ``f 1`` -> ``$x`` becomes ``1`` (no error for this input)
             ``f {1;exec rm}`` -> would execute rm  (the smell)
    """
    src = 'proc f {input} { eval "set x $input" }\n'
    assert _any(src, "W101", "W105")


def test_TP_W232_string_index_past_end_is_smell():
    """``string index abc 99`` returns the empty string in tclsh — not
    an error, but a smell (programmer almost certainly didn't mean to
    index past the end).

    runtime: ``puts <[f]>`` -> ``<>``  (no error)
    """
    src = "proc f {} { return [string index abc 99] }\n"
    assert _any(src, "W232")


def test_TP_S101_per_iteration_mixed_intrep_thrash():
    """Per-iteration mix of ``string length`` (string intrep) and
    ``lindex`` (list intrep) on the same loop variable — genuine
    per-iteration shimmer.  tclsh runs fine; the diagnostic flags
    the wasted intrep regeneration.

    runtime: ``puts [f {{1 2} {3 4}}]`` -> ``ok``  (no error; perf smell)
    """
    src = (
        "proc f {l} {\n"
        "    foreach x $l {\n"
        "        set a [string length $x]\n"
        "        set b [lindex $x 0]\n"
        "    }\n"
        "    return ok\n"
        "}\n"
    )
    assert _any(src, "S100", "S101", "S102")


def test_TP_W220_dead_after_error_command():
    """``error oops`` unconditionally raises; statements after it are
    unreachable.  Analyser fires W220 on the dead ``set x 2``.

    runtime: ``catch f msg; puts $msg`` -> ``oops``  (the dead set never runs)
    """
    src = "proc f {} {\n    error oops\n    set x 2\n}\n"
    assert _any(src, "W220")


# =====================================================================
# Section 3 — False Negatives.
#
# Each xfail records a precision gap where the analyser is silent but
# tclsh proves the snippet errors at runtime.  ``strict=True`` flips
# the xfail to a failure the moment the gap is closed -- prompting
# removal of the xfail marker.
# =====================================================================


@pytest.mark.xfail(
    strict=True,
    reason="FN: read of var set only on some if-arms — phi-from-undef not flagged",
)
def test_FN_if_arm_only_def_read_in_merge():
    """``v`` is defined only when ``$x > 0``; the unconditional
    ``return $v`` reads it on the no-set path too.

    runtime: ``f -1`` -> ERROR ``can't read "v": no such variable``
    """
    src = "proc f {x} {\n    if {$x > 0} { set v 1 }\n    return $v\n}\n"
    assert _any(src, "W210"), (
        "phi-from-undef should fire W210; the merge has an incoming where v is unset"
    )


@pytest.mark.xfail(
    strict=True,
    reason="FN: switch without default — phi-from-undef on the no-arm path",
)
def test_FN_switch_no_default_arm_read():
    """``switch`` with no ``default`` clause and a value matching none
    of the arms leaves ``v`` unset.

    runtime: ``f c`` -> ERROR ``can't read "v": no such variable``
    """
    src = "proc f {x} {\n    switch $x { a { set v 1 } b { set v 2 } }\n    return $v\n}\n"
    assert _any(src, "W210"), "switch-no-default + unconditional read should fire W210"


@pytest.mark.xfail(
    strict=True,
    reason="FN: use-after-unset not flagged (Place model doesn't track unset as a kill)",
)
def test_FN_use_after_unset_in_same_proc():
    """``unset v`` deletes the binding; the subsequent ``return $v``
    errors at runtime.

    runtime: ``f`` -> ERROR ``can't read "v": no such variable``
    """
    src = "proc f {} {\n    set v 1\n    unset v\n    return $v\n}\n"
    assert _any(src, "W210", "W213"), (
        "use-after-unset must fire either W210 (read of unset) or W213"
    )


@pytest.mark.xfail(
    strict=True,
    reason=(
        "FN: if{0} body is dead per SCCP (I230/O112 do fire), but the read in the "
        "merge isn't flagged W210 — read_before_set doesn't propagate the dead-body kill"
    ),
)
def test_FN_if_zero_dead_body_read_in_merge():
    """SCCP knows ``if {0}`` body is unreachable (I230 + O112 fire).
    On every reachable path ``v`` is never defined, so reading
    ``$v`` after the dead if is a real W210.

    runtime: ``f`` -> ERROR ``can't read "v": no such variable``
    """
    src = "proc f {} {\n    if {0} { set v 1 }\n    return $v\n}\n"
    assert _any(src, "W210"), (
        "reachable-only def is unreached when if-cond is constant-0; read_before_set "
        "should consume SCCP reachability"
    )


@pytest.mark.xfail(
    strict=True,
    reason=(
        "FN: post-return code is unreachable but neither O107 nor W220 fires on the dead store"
    ),
)
def test_FN_dead_store_after_unconditional_return():
    """``return 1`` makes ``set x 2`` unreachable.  No diagnostic
    currently fires.

    runtime: ``puts [f]`` -> ``1``  (the dead store never runs)
    """
    src = "proc f {} {\n    return 1\n    set x 2\n}\n"
    assert _any(src, "O107", "W220"), (
        "Statement after unconditional return should fire O107 (unreachable) or W220 (dead store)"
    )
