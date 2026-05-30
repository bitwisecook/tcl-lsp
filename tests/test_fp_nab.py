"""TP/FP regression tests for the NAB (not-a-bug / confirm-correct) family.

Each test pairs to an `FP-NAB-NN` entry in
``docs/design/compiler/FP.md``.  The Tcl reproducer string is **copied verbatim**
from the doc — if either side drifts the test will visibly catch it.

Ground truth: real C tclsh 9.0.3 (run via ``tclsh9.0`` during planning).
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser import analyse
from compiler.compilation_unit import compile_source
from server.features.diagnostics import get_diagnostics


def _analyser_codes(source: str, code: str):
    """Analyser-tier diagnostics matching *code* — mirrors test_checks._diag_with_code."""
    return [d for d in analyse(source).diagnostics if d.code == code]


def _all_codes(source: str, code: str):
    """Full pipeline diagnostics (analyser + bounds + interval + optimiser) matching *code*."""
    return [d for d in get_diagnostics(source) if d.code == code]


# FP-NAB-01 — lset append-slot (index == length) is legal, NOT W231


FP_NAB_01_REPRO = """\
# tclsh contract: lset at index == length APPENDS (legal, not an error).
set l {a b c}      ;# llength=3
lset l 3 X         ;# 3 == llength $l -> APPENDS X (NOT an error)
puts $l            ;# use post-lset binding (silences W211 on l#2)
"""


def test_FP_NAB_01_append_slot_silent():
    """FP guard: lset at index==length must NOT fire W231 (the append slot is
    legal in tclsh; see docs/design/compiler/FP.md#fp-nab-01).

    Uses a literal 3-element list so the bounds check has a *statically
    known* length to compare against — index 3 then exercises the precise
    `index == length` append slot.  A parameter form (l as a proc arg)
    would have unknown length and the verdict would be vacuous: the test
    would still pass if the analyser regressed `>` to `>=` because
    unknown-length lists never fire the bounds check at all.
    """
    assert _all_codes(FP_NAB_01_REPRO, "W231") == []


def test_FP_NAB_01_real_out_of_range_fires():
    """TP control: a literal lset index > length IS a tclsh error
    (`index "4" out of range`) and SHOULD fire W231.  Catches a regression
    where the append-slot fix accidentally suppresses real out-of-range cases
    too.  Uses the `set var; lset var ...` pattern that W231's recent-set
    length inference walks back to."""
    real_oor = """\
set l {a b c}      ;# llength=3
lset l 4 X         ;# 4 > 3 -> tclsh: index "4" out of range
"""
    diags = get_diagnostics(real_oor)
    w231 = [d for d in diags if d.code == "W231" and "out of range" in (d.message or "").lower()]
    assert w231, "lset l 4 X (preceded by `set l {a b c}`) must fire W231; current: " + ", ".join(
        f"{d.code}:{d.message}" for d in diags
    )


# FP-NAB-02 — lindex out-of-range returns "" — smell (W230), not an error


FP_NAB_02_REPRO = """\
# Top-level lindex with literal list + literal out-of-range index.
# tclsh returns "" silently — likely-bug, not an error.
set x [lindex {a b c} 9]
return $x
"""


def test_FP_NAB_02_lindex_oor_smell_fires():
    """TP: literal-arg lindex past the end fires W230 (smell), reflecting that
    tclsh returns the empty string (the code RUNS, but is probably buggy)."""
    assert _all_codes(FP_NAB_02_REPRO, "W230"), (
        "Out-of-range literal lindex should fire W230 (smell). "
        "If this regresses, the bounds check in analyser/checks/_bounds.py "
        "lost its literal-index path."
    )


def test_FP_NAB_02_lindex_oor_not_w231():
    """FP guard: an out-of-range lindex must NOT escalate to the error-tier
    W231 (which is reserved for lset, where tclsh actually errors)."""
    assert _all_codes(FP_NAB_02_REPRO, "W231") == [], (
        "lindex out-of-range is smell-only (W230); escalating to W231 would "
        "be a regression of the dialect-aware severity split."
    )


def test_FP_NAB_02_lset_same_index_does_w231():
    """TP control: the matching lset at the same out-of-range index DOES error
    in tclsh (`index "9" out of range`) and SHOULD trigger W231 (error-tier).
    Proves the W230 (smell) vs W231 (error) asymmetry encodes a real dialect
    difference between lindex and lset."""
    lset_oor = """\
set l {a b c}
lset l 9 X         ;# 9 > 3 -> tclsh: index "9" out of range
"""
    diags = get_diagnostics(lset_oor)
    w231 = [d for d in diags if d.code == "W231" and "out of range" in (d.message or "").lower()]
    assert w231, "lset l 9 X must fire W231; current: " + ", ".join(
        f"{d.code}:{d.message}" for d in diags
    )


# FP-NAB-03 — recursive procs ARE detected pure (Phase-4 SCC not needed)


FP_NAB_03_REPRO = """\
proc fact {n} {
    if {$n <= 1} { return 1 }
    return [expr {$n * [fact [expr {$n - 1}]]}]
}
"""


def test_FP_NAB_03_recursive_proc_detected_pure():
    """TP: a self-recursive arithmetic proc must come out `pure=True` from the
    interproc fix-point (the original plan-doc claim that recursion was
    conservatively impure was wrong; the fix-point is greatest, not least)."""
    cu = compile_source(FP_NAB_03_REPRO)
    fact = cu.interproc.procedures.get("::fact")
    assert fact is not None, "::fact missing from interproc summary"
    assert fact.pure is True, (
        f"recursive arithmetic proc must be pure; got pure={fact.pure}.  "
        "If this regresses, analyse_interprocedural_ir lost its "
        "greatest-fix-point initialisation."
    )


def test_FP_NAB_03_impure_proc_still_detected():
    """Control: an impure proc using puts must come out pure=False.  Proves the
    test isn't trivially asserting all procs pure."""
    impure = """\
proc logit {msg} {
    puts $msg            ;# I/O side-effect — must NOT be detected pure
    return ok
}
"""
    cu = compile_source(impure)
    logit = cu.interproc.procedures.get("::logit")
    assert logit is not None, "::logit missing from interproc summary"
    assert logit.pure is False, f"a proc that calls puts must be impure; got pure={logit.pure}"
