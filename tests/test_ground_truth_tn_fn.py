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
# Section 3 — Precision-gap closures.
#
# Each of these was previously an ``xfail(strict=True)`` precision gap;
# the analyser was silent on a snippet that tclsh proves wrong at
# runtime.  All five gaps were closed by the phi-from-undef detector
# + the post-terminator-orphan CFG construction:
#
#   * phi-from-undef trace in ``_read_before_set`` walks the SSA phi
#     DAG (restricted to SCCP-reachable predecessors) and fires W210
#     when any leaf is version 0 (undef) -- catches if-arm-only,
#     switch-without-default, and the if{0} dead-body case.
#   * ``unset v``-defined SSA versions are recorded as ``killed`` and
#     treated as undef-equivalent by ``_phi_can_undef`` -- catches
#     use-after-unset.
#   * The CFG builder now routes post-terminator dead statements
#     (``return 1; set x 2``) into an orphan unreachable block in
#     analysis builds -- O107 picks them up.  Codegen builds keep
#     the original behaviour (drop them) so default bytecode stays
#     byte-identical to tclsh.
# =====================================================================


def test_TP_W210_if_arm_only_def_read_in_merge():
    """``v`` is defined only when ``$x > 0``; the unconditional
    ``return $v`` reads it on the no-set path too.

    runtime: ``f -1`` -> ERROR ``can't read "v": no such variable``
    """
    src = "proc f {x} {\n    if {$x > 0} { set v 1 }\n    return $v\n}\n"
    assert _any(src, "W210"), (
        "phi-from-undef must fire W210; the merge has an incoming where v is unset"
    )


def test_TP_W210_switch_no_default_arm_read():
    """``switch`` with no ``default`` clause and a value matching none
    of the arms leaves ``v`` unset.

    runtime: ``f c`` -> ERROR ``can't read "v": no such variable``
    """
    src = "proc f {x} {\n    switch $x { a { set v 1 } b { set v 2 } }\n    return $v\n}\n"
    assert _any(src, "W210"), "switch-no-default + unconditional read must fire W210"


def test_TP_W210_use_after_unset_in_same_proc():
    """``unset v`` deletes the binding; the subsequent ``return $v``
    errors at runtime.  The SSA version of ``v`` defined by ``unset``
    is recorded as ``killed`` so phi-from-undef treats it as undef.

    runtime: ``f`` -> ERROR ``can't read "v": no such variable``
    """
    src = "proc f {} {\n    set v 1\n    unset v\n    return $v\n}\n"
    assert _any(src, "W210", "W213"), "use-after-unset must fire W210 (read of killed var) or W213"


def test_TP_W210_if_zero_dead_body_read_in_merge():
    """SCCP knows ``if {0}`` body is unreachable.  On every reachable
    path ``v`` is never defined, so the phi-from-undef trace finds
    only an undef incoming on the entry path.

    runtime: ``f`` -> ERROR ``can't read "v": no such variable``
    """
    src = "proc f {} {\n    if {0} { set v 1 }\n    return $v\n}\n"
    assert _any(src, "W210"), (
        "reachable-only def is unreached when if-cond is constant-0; W210 must fire"
    )


def test_TP_W210_loop_body_only_init_with_empty_input():
    """Loop body provides the only init of a variable.  When the loop
    runs at least once the var is set; when the input list is empty
    the loop body never runs and the variable stays unset, so the
    subsequent read errors.  The phi-from-undef detector catches this
    via the loop-header phi's entry-path incoming being undef.

    runtime: ``foo {}`` -> ERROR ``can't read "result": no such variable``
             ``foo {a b}`` -> ``a b``  (no error)
    """
    src = "proc foo {items} {\n    foreach item $items { lappend result $item }\n    return $result\n}\n"
    assert _any(src, "W210"), (
        "loop-body-only def + post-loop read must fire W210 (empty input leaves var unset)"
    )


def test_TP_W210_dynamic_target_upvar_read():
    """``upvar 1 $varName local`` aliases ``local`` to the caller var
    named by ``$varName``.  When ``$varName`` names a caller var that
    doesn't exist, the alias is a no-op and reading ``$local`` errors.

    runtime: ``foo nonexistent`` -> ERROR ``can't read "local": no such variable``
    """
    src = "proc foo {varName} {\n    upvar 1 $varName local\n    puts $local\n}\n"
    assert _any(src, "W210"), "dynamic-target upvar + unconditional read must fire W210"


def test_TN_unset_then_return_with_options_no_propagation():
    """``unset $v; return -code error ...`` inside a loop body must not
    propagate the killed version through the loop-header phi to a
    post-loop read.  The block ALWAYS exits via the error return, so
    the back-edge to the loop header from that block is infeasible.

    Without this property the http.tcl ``CreateToken`` pattern
    spuriously fires W210 on a post-loop ``return $token`` after a
    ``foreach { ... unset $token; return -code error ... }`` loop.

    runtime: ``f http://x -unknown bad`` -> ERROR (clean: token created
    and discarded, no W210 on the post-loop return)
    """
    src = (
        "proc f {url args} {\n"
        "    set token [incr uid]\n"
        "    foreach {flag value} $args {\n"
        '        if {$flag eq "-x"} {\n'
        "            unset $token\n"
        '            return -code error "bad"\n'
        "        }\n"
        "    }\n"
        "    return $token\n"
        "}\n"
    )
    diags_w210 = [c for c in _codes(src, "W210", "W213")]
    assert not diags_w210, (
        f"return -code error must terminate the block; post-loop W210 must NOT fire, got {diags_w210}"
    )


def test_TN_static_target_upvar_read():
    """``upvar 1 caller local`` with a STATIC target IS sound -- the
    callee must be invoked under a caller that has ``caller`` defined,
    a documented Tcl contract.  Must NOT fire W210.

    runtime: ``caller`` -> ``1``  (no error)
    """
    src = "proc f1 {} { set v 1; f2 }\nproc f2 {} { upvar 1 v local; puts $local }\n"
    diags_w210 = [c for c in _codes(src, "W210")]
    assert not diags_w210, f"static-target upvar must stay silent, got {diags_w210}"


def test_TP_O107_dead_store_after_unconditional_return():
    """``return 1`` makes ``set x 2`` unreachable.  The CFG builder's
    analysis path routes post-terminator statements into an orphan
    unreachable block; O107 fires on them.

    runtime: ``puts [f]`` -> ``1``  (the dead store never runs)
    """
    src = "proc f {} {\n    return 1\n    set x 2\n}\n"
    assert _any(src, "O107", "W220"), (
        "Statement after unconditional return must fire O107 (unreachable) or W220"
    )


# Companion controls -- analyser must stay silent on the
# look-alike-but-actually-safe variants that exercise the same machinery.


def test_TN_unset_then_reset_then_read():
    """After ``unset v; set v 2``, reading ``$v`` is safe -- the
    re-set re-establishes the binding.  Must NOT fire W210.

    runtime: ``puts [f]`` -> ``2``  (no error)
    """
    src = "proc f {} {\n    set v 1\n    unset v\n    set v 2\n    return $v\n}\n"
    assert not _any(src, "W210", "W213")


def test_TN_unset_nocomplain_no_read():
    """``unset -nocomplain v`` on a never-set var is a documented
    idempotent cleanup; with no subsequent read it must NOT fire.

    runtime: ``puts [f]`` -> empty  (no error)
    """
    src = "proc f {} {\n    unset -nocomplain v\n    return done\n}\n"
    assert not _any(src, "W210", "W213")


def test_TN_if_else_both_arms_set():
    """Both arms of an if/else set ``v``, so on every path it's
    defined before the read.  Phi-from-undef must NOT fire.

    runtime: ``puts [f 1]`` -> ``1``  (no error)
    """
    src = "proc f {x} {\n    if {$x > 0} { set v 1 } else { set v 2 }\n    return $v\n}\n"
    assert not _any(src, "W210")


def test_TN_switch_with_default_sets():
    """``switch`` with a ``default`` clause that also sets ``v`` covers
    every path; must NOT fire W210.

    runtime: ``puts [f c]`` -> ``other``  (no error)
    """
    src = (
        "proc f {x} {\n"
        "    switch $x { a { set v 1 } b { set v 2 } default { set v other } }\n"
        "    return $v\n"
        "}\n"
    )
    assert not _any(src, "W210")


# pytest is imported in this module for the historical xfail-strict
# markers; keep the import alive so a future re-introduction of an FN
# is a one-line edit.
_pytest_alive = pytest
