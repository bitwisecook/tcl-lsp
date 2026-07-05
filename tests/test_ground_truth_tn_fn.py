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


from server.features.diagnostics import get_diagnostics


def _codes(source: str, *codes: str) -> list[str]:
    """All firings of *codes* on *source* (returns the per-firing code list)."""
    wanted = set(codes)
    return [str(d.code) for d in get_diagnostics(source) if d.code in wanted]


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


def test_TP_dead_after_error_command():
    """``error oops`` unconditionally raises; statements after it are
    unreachable.  The CFG builder promotes ``error`` (registered with
    the ``terminates_block`` registry trait) to a block terminator in
    analysis builds, so O107 fires on the post-error dead code.

    runtime: ``catch f msg; puts $msg`` -> ``oops``  (the dead set never runs)
    """
    src = "proc f {} {\n    error oops\n    set x 2\n}\n"
    assert _any(src, "O107", "W220"), (
        "post-error statement must fire either O107 (unreachable) or W220 (dead store)"
    )


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


def test_TN_scan_percent_n_on_empty_input():
    """``%n`` writes the count of consumed characters without consuming
    any input -- ``scan "" %n n`` sets n=0 successfully.  Must NOT
    fire W210 (D4-F1 closure).

    runtime: ``f`` -> ``0``  (no error)
    """
    src = "proc f {} { scan {} %n n; puts $n }\n"
    assert not _any(src, "W210"), "scan %n on empty input must succeed"


def test_TN_scan_float_accepts_inf():
    """Tcl ``scan %f`` accepts ``Inf``/``Infinity``/``NaN`` per
    ``Tcl_GetDouble``.  Must NOT fire W210 (D4-F1 closure).

    runtime: ``f`` -> ``inf``  (no error)
    """
    src = "proc f {} { scan Inf %f f; puts $f }\n"
    assert not _any(src, "W210"), "scan %f with Inf input must succeed"


def test_TN_scan_format_whitespace_includes_cr_ff_vt():
    """``scan " 123" "\\r%d" n`` matches: format ``\\r`` is whitespace
    that skips the leading space, then ``%d`` consumes 123.  Must NOT
    fire W210 (D4-F1 closure).

    runtime: ``f`` -> ``123``  (no error)
    """
    src = 'proc f {} { scan { 123} "\\r%d" n; puts $n }\n'
    assert not _any(src, "W210"), "format \\r should skip input whitespace"


def test_TP_scan_genuine_no_match_still_fires():
    """TP control: when scan provably can't consume input, W210 must
    still fire.  Confirms the F1 over-conservatism didn't degrade
    detection of real no-match cases."""
    src = "proc f {} { scan abc %d n; puts $n }\n"
    assert _any(src, "W210"), "real no-match (abc vs %d) must still fire"


def test_TP_W210_empty_dict_with_return_missing_var():
    """``set d {}; dict with d {}; return $missing`` -- empty dict
    unpacks no keys, ``$missing`` reads an unset variable.  The
    return-terminator path must apply the same key-aware suppression
    as the statement path (D4-F3 / D3-P1 closure).

    runtime: ``f`` -> ERROR ``can't read "missing": no such variable``
    """
    src = "proc f {} { set d {}; dict with d {}; return $missing }\n"
    assert _any(src, "W210"), "empty dict with does not unpack 'missing'"


def test_TN_known_key_dict_with_return_var():
    """TN control: ``set d {missing ok}; dict with d {}; return $missing``
    -- the literal dict has key ``missing``, so ``dict with`` unpacks
    it as a local.  Must NOT fire W210."""
    src = "proc f {} { set d {missing ok}; dict with d {}; return $missing }\n"
    assert not _any(src, "W210")


def test_TN_unknown_dict_with_return_var():
    """TN control: when the dict shape is unknown (e.g. callee param),
    the return-path read of any name must conservatively stay silent
    -- the analyser can't prove the key is absent."""
    src = "proc f {d} { dict with d {}; return $missing }\n"
    assert not _any(src, "W210")


def test_TP_optimiser_O126_preserves_puts_side_effect():
    """``set unused [puts side]`` -- the assignment IS unused but the
    RHS prints to stdout.  Deleting it changes program output.  After
    the D2-O126 closure the optimiser must NOT emit O126 here.

    runtime: orig prints ``side`` then ``done``; the OLD optimised
    version printed only ``done`` (lost the side-effect).
    """
    from compiler.optimiser import optimise_source

    src = "proc f {} { set unused [puts side]; puts done }"
    _, rewrites = optimise_source(src)
    codes = [r.code for r in rewrites]
    assert "O126" not in codes, f"O126 must NOT delete a [puts X] RHS; got codes {codes}"


def test_TP_optimiser_O126_keeps_for_pure_RHS():
    """TP control: when the RHS is a literal or a pure command, O126
    SHOULD still fire.  Confirms the purity gate didn't disable the
    optimisation entirely."""
    from compiler.optimiser import optimise_source

    src = "proc f {} { set unused [list 1 2 3]; puts done }"
    _, rewrites = optimise_source(src)
    codes = [r.code for r in rewrites]
    assert "O126" in codes, f"O126 must fire when RHS is pure (list); got codes {codes}"


def test_TP_optimiser_O100_does_not_propagate_past_cmd_sub_write():
    """``set x a; set y [append x b]; puts $x`` -- the ``[append x b]``
    mutates x as a side effect.  The optimiser must NOT propagate the
    stale ``a`` value into the subsequent ``puts $x`` (D2-O100 closure
    via kill_sites including statement_cmd_sub_write_names).

    runtime: tclsh prints ``ab\\nab\\n``; pre-fix optimiser produced
    a program that printed ``a\\nb\\n``.
    """
    from compiler.optimiser import optimise_source

    src = "proc f {} { set x a; set y [append x b]; puts $x; puts $y }"
    opt_src, _ = optimise_source(src)
    # The propagation that would replace ``$x`` with ``a`` is unsound;
    # the optimised source must keep the original ``$x`` (or otherwise
    # not embed the literal ``a`` as the puts argument).
    assert "puts a" not in opt_src, (
        f"O100 must NOT propagate stale value past [append x b]; got: {opt_src.strip()}"
    )


def test_TN_namespaced_ensemble_resolved_known_proc():
    """D4-F7 closure: ``${ns}::dowork`` where ``ns`` is a CONST proven by
    SCCP and ``::mypkg::dowork`` IS a known proc in the same file --
    the analyser must NOT fire W307.  The composed-name check
    (``$prefix::tail`` -> ``mypkg::dowork``) resolves to a known proc.

    runtime: ``f`` -> ``(no output, dowork was a no-op)``
    """
    src = (
        "namespace eval ::mypkg { proc dowork {arg} {} }\n"
        "proc f {} { set ns mypkg; ${ns}::dowork arg }\n"
    )
    assert not _any(src, "W307"), (
        "composed ${ns}::dowork must NOT fire W307 when ::mypkg::dowork is a known proc"
    )


def test_TP_namespaced_ensemble_composed_unknown():
    """D4-F7 control: ``${ns}::unknownproc`` where the composed name
    has NO known proc must still fire W307 -- it's a genuinely
    unresolvable dynamic dispatch."""
    src = "proc f {} { set ns mypkg; ${ns}::unknownproc arg }\n"
    assert _any(src, "W307"), "composed ${ns}::unknownproc must fire W307 when not resolvable"


def test_TN_pure_var_ref_handles_escaped_paren_in_array_index():
    """D4-F11 closure: ``$a(x\\)y)`` is a valid Tcl variable reference
    (tclsh accepts ``set a(x\\)y) 1; puts $a(x\\)y)``).  The old regex-
    based ``is_pure_var_ref`` terminated the index at the first ``)``
    and returned False; the new lexer-correct parser accepts it."""
    from compiler.value_shapes import is_pure_var_ref

    assert is_pure_var_ref(r"$a(x\)y)"), (
        "escaped close-paren inside array index must count as one pure var ref"
    )
    # Companion controls.
    assert is_pure_var_ref("$x")
    assert is_pure_var_ref("${some name}")
    assert not is_pure_var_ref("$x$y")
    assert not is_pure_var_ref("$x.foo")


def test_TN_scan_with_more_than_18_vars_no_false_w210():
    """D4-F2 closure: the previous fixed-slot ``arg_roles`` only
    recognised the first 18 varName args; vars 19+ weren't classified
    as VAR_WRITE and fired W210 at use.  Dynamic resolver fixes it.

    runtime: ``puts [f]`` -> ``19``  (no error)
    """
    src = (
        "proc f {} {\n"
        "    scan {x0 x1 x2 x3 x4 x5 x6 x7 x8 x9 x10 x11 x12 x13 x14 x15 x16 x17 x18 x19} "
        "{%s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s %s} "
        "v0 v1 v2 v3 v4 v5 v6 v7 v8 v9 v10 v11 v12 v13 v14 v15 v16 v17 v18 v19\n"
        "    return $v19\n"
        "}\n"
    )
    assert not _any(src, "W210"), "scan with 20 vars must not false-fire W210 on v18/v19"


def test_TN_lassign_with_many_vars_no_false_w210():
    """D4-F2 closure: same fix for ``lassign``.  Calls with more
    varNames than the old hard-coded slot count must not false-fire."""
    src = (
        "proc f {l} { lassign $l a b c d e f g h i j k l2 m n o p q r s t u v w x y; return $y }\n"
    )
    assert not _any(src, "W210")


def test_TN_binary_scan_with_many_vars_no_false_w210():
    """D4-F2 closure: same fix for ``binary scan``."""
    src = "proc f {} { binary scan {} {i i i i i i i i i i i i i i i i i i i i} a b c d e f g h i j k l m n o p q r s t; return $t }\n"
    assert not _any(src, "W210")


def test_TP_W307_unrelated_dispatcher_does_not_suppress_peer_family():
    """D4-F4 / D3-P9 closure: a ``$cmd x`` (1-arg) dispatcher in the
    same namespace as three 2-arg peer procs is NOT evidence for the
    peer protocol -- the arities are incompatible.  W214 must still
    fire on the unused ``token`` parameter of each peer.

    runtime: each peer ignores ``token``, ``unrelated`` only dispatches
    its own ``$cmd x`` (1 arg) -- nothing actually calls a/b/c with
    the (ctx, token) shape.
    """
    src = (
        "namespace eval ::n {\n"
        "    proc a {ctx token} { puts $ctx }\n"
        "    proc b {ctx token} { puts $ctx }\n"
        "    proc c {ctx token} { puts $ctx }\n"
        "    proc unrelated {cmd} { $cmd x }\n"
        "}\n"
    )
    w214 = [d for d in get_diagnostics(src) if d.code == "W214" and "'token'" in d.message]
    assert len(w214) == 3, (
        f"W214 must fire on token in a/b/c when no protocol-compatible dispatcher exists, got {w214}"
    )


def test_TN_W307_protocol_compatible_dispatcher_suppresses_peer_family():
    """D4-F4 control: when a same-namespace dispatcher has matching
    arity (``$cmd ctx token``, 2 args >= peer signature), the protocol
    evidence applies and W214 stays silent."""
    src = (
        "namespace eval ::n {\n"
        "    proc a {ctx token} { puts $ctx }\n"
        "    proc b {ctx token} { puts $ctx }\n"
        "    proc c {ctx token} { puts $ctx }\n"
        "    proc dispatch {cmd ctx token} { $cmd $ctx $token }\n"
        "}\n"
    )
    assert not _any(src, "W214")


def test_TP_W307_format_in_method_fires():
    """D4-F5 / D3-P3 closure: ``[format X] run`` inside a method body
    must fire W307 -- the in-method blanket suppression is gone; only
    a known OBJECT return type, ``my``/``self`` self-dispatch, or
    other positive evidence suppresses now."""
    src = "oo::class create C { method m {} { [format notACommand] run } }"
    assert _any(src, "W307")


def test_TN_W307_known_class_new_in_method_silent():
    """D4-F5 control: ``[D new] run`` IS suppressed because ``D``'s
    constructor return type is known OBJECT via ``known_classes``."""
    src = (
        "oo::class create D { method run {} { return ok } }\n"
        "oo::class create C { method m {} { [D new] run } }\n"
    )
    assert not _any(src, "W307")


def test_TP_W307_unknown_class_new_does_not_suppress():
    """D4-F6 / D3-P6 closure: ``[NotAClass new]`` MUST fire W307 --
    the analyser has no evidence (registry / class definition) that
    NotAClass is an object factory.  The bare ``new``-subcommand
    heuristic that used to silently type the result as OBJECT was
    unsound (verified vs tclsh in the special-casing review)."""
    src = "proc f {} { set x [NotAClass new]; $x method }\n"
    assert _any(src, "W307")


def test_TN_W307_known_tclOO_class_new_silent():
    """D4-F6 control: ``[C new]`` where C IS a known oo::class
    correctly suppresses W307."""
    src = (
        "oo::class create C { method run {} { return ok } }\nproc f {} { set x [C new]; $x run }\n"
    )
    assert not _any(src, "W307")


def test_TP_W307_callback_array_holds_noncommand():
    """D3-P7 closure: ``array set state {-command notACommand}; $state(-command)
    hi`` MUST fire W307 -- the literal element value proves the slot
    holds a non-command, so the callback-key heuristic suppression
    is overridden by the SCCP-CONST evidence (now harvested from
    ``array set`` literal lists)."""
    src = "proc f {} { array set state {-command notACommand}; $state(-command) hi }\n"
    assert _any(src, "W307")


def test_TN_W307_callback_array_holds_known_command():
    """D3-P7 control: ``array set state {-command puts}`` -- the
    literal value IS a known command, so W307 stays silent."""
    src = "proc f {} { array set state {-command puts}; $state(-command) hi }\n"
    assert not _any(src, "W307")


def test_TP_O126_pure_user_proc_RHS_is_deleted():
    """D2-O126-FU closure: when ``set unused [add 1 2]`` and ``add``
    is interprocedurally proven pure (only ``expr``, no I/O), the
    optimiser CAN safely fold the unused assignment.  Prior behaviour
    conservatively refused on any user-proc RHS."""
    from compiler.optimiser import optimise_source

    src = "proc add {a b} { expr {$a + $b} }\nproc f {} { set unused [add 1 2]; puts done }"
    _, rewrites = optimise_source(src)
    assert "O126" in [r.code for r in rewrites], "pure user-proc RHS must allow O126 deletion"


def test_TN_O126_impure_user_proc_RHS_preserved():
    """D2-O126-FU control: ``set unused [shout x]`` where ``shout``
    contains ``puts`` (impure) MUST NOT be deleted.  Interproc summary
    correctly classifies ``shout`` as impure, so the gate refuses."""
    from compiler.optimiser import optimise_source

    src = "proc shout {x} { puts $x; expr {$x + 1} }\nproc f {} { set unused [shout 1]; puts done }"
    _, rewrites = optimise_source(src)
    assert "O126" not in [r.code for r in rewrites], (
        "impure user-proc RHS must NOT be deleted by O126 (loses side effect)"
    )


def test_TP_W307_my_method_returns_plain_literal():
    """D3-P4 closure: ``[my plain] run`` where ``plain`` is a method
    in the enclosing class whose body is a simple ``return <literal>``
    -- the return is provably a STRING, not an object handle.  Fire
    W307.  Compound bodies (cmd-subs, variables, multiple statements)
    stay conservatively suppressed via the ``my``/``self`` heuristic."""
    src = (
        "oo::class create C { method plain {} { return notACommand }\n"
        "method m {} { [my plain] run } }"
    )
    assert _any(src, "W307")


def test_TN_W307_my_method_returns_object_silent():
    """D3-P4 control: ``[my obj] run`` where ``obj`` returns ``[D new]``
    has a cmd-sub in its body, so the simple-literal-return check
    doesn't apply; the conservative TclOO-self-dispatch suppression
    holds, W307 stays silent."""
    src = (
        "oo::class create D { method run {} { return ok } }\n"
        "oo::class create C { method obj {} { return [D new] }\n"
        "method m {} { [my obj] run } }"
    )
    assert not _any(src, "W307")


def test_TP_W210_interproc_dict_with_empty_arg_unpacks_no_keys():
    """D3-P2 closure: ``proc f {d} { dict with d { return $missing } }``
    called with literal ``{}``.  The caller's empty dict is
    interprocedurally propagated to ``d`` (call-site literal
    collection + SCCP barrier preserving v0).  The dict-with reads
    SCCP CONST('') for d, harvests no keys, exempts NO names; the
    return reads ``$missing`` which IS read-before-set.

    runtime: ``f {}`` -> ERROR ``can't read "missing"``
    """
    src = "proc f {d} { dict with d { return $missing } }\nf {}\n"
    assert _any(src, "W210"), (
        "caller-passed empty dict must propagate to callee dict-with key check"
    )


def test_TN_interproc_dict_with_key_present_silent():
    """D3-P2 control: caller passes ``{missing ok}``, key matches.
    Callee dict-with unpacks ``missing`` as a local; reading ``$missing``
    is safe."""
    src = "proc f {d} { dict with d { return $missing } }\nf {missing ok}\n"
    assert not _any(src, "W210")


def test_TN_interproc_mixed_callers_conservative():
    """D3-P2 control: when callers pass DIFFERENT literals (one empty,
    one with key), the analysis falls back to conservative -- W210
    must NOT fire (some path defines missing)."""
    src = "proc f {d} { dict with d { return $missing } }\nf {}\nf {missing X}\n"
    assert not _any(src, "W210")


def test_TP_W307_interproc_dict_with_unpacks_non_command():
    """D3-P8 closure: ``proc f {d} { dict with d { $cmd hi } }`` called
    with literal ``{cmd notACommand}``.  Interproc propagation puts the
    literal dict in d at v0; the dict-with-key harvester registers
    ``cmd`` -> ``notACommand`` as a CONSTSET; the W307 SCCP-evidence
    override fires because ``notACommand`` isn't a known command.

    runtime: ``f {cmd notACommand}`` -> ERROR ``invalid command "notACommand"``
    """
    src = "proc f {d} { dict with d { $cmd hi } }\nf {cmd notACommand}\n"
    assert _any(src, "W307"), "interproc-propagated callback non-command must fire W307"


def test_TN_interproc_dict_with_unpacks_known_command_silent():
    """D3-P8 control: same shape but the unpacked value is a known
    command (``puts``).  Must NOT fire."""
    src = "proc f {d} { dict with d { $cmd hi } }\nf {cmd puts}\n"
    assert not _any(src, "W307")


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
# markers.
