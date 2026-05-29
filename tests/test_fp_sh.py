"""TP/FP regression tests for the SH (shimmer) family.

Each test pairs to an ``FP-SH-NN`` entry in
``docs/design/compiler/FP.md``.  The Tcl reproducer string is **copied verbatim**
from the doc — if either side drifts the test will visibly catch it.

SH family covers S100/S101/S102 — shimmer warnings for "value of one Tcl type
flowing into an operator that wants another" (STRING → arithmetic, INT →
string-compare, etc.).  Each entry locks in a conservative-suppression
verdict (FP) or a hash-seed-independent determinism property (control).

Ground truth: real C tclsh 9.0.3.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from server.features.diagnostics import get_diagnostics


def _codes(source: str, codes: list[str]):
    """Diagnostics matching any code in *codes*."""
    return [d for d in get_diagnostics(source) if d.code in codes]


SHIMMER_CODES = ["S100", "S101", "S102"]


# FP-SH-01 — OVERDEFINED values do not trigger shimmer


FP_SH_01_REPRO = """\
# x has unknown type (cmd return) -> OVERDEFINED -> no shimmer warning.
set x [unknownCmd]
set y [expr {$x + 1}]
return $y
"""


def test_FP_SH_01_overdefined_silent():
    """FP guard: OVERDEFINED in arithmetic must NOT fire S100 — the type is
    unknown, so any verdict would be unsound.  See analyser/checks/_shimmer.py."""
    assert _codes(FP_SH_01_REPRO, SHIMMER_CODES) == [], (
        "OVERDEFINED value should not produce a shimmer warning"
    )


def test_FP_SH_01_string_arith_still_fires():
    """TP control: a *KNOWN* STRING value in arithmetic must still fire S100.
    Proves the OVERDEFINED suppression isn't blanket-silencing all shimmer."""
    src = "set s hello\nset y [expr {$s + 1}]"
    assert _codes(src, ["S100"]), "known STRING in arithmetic must still fire S100"


# FP-SH-02 — scope-alias declarations typed OVERDEFINED (not STRING)


FP_SH_02_REPRO = """\
proc f {} {
    # `variable v` declares an alias — type is unknown (OVERDEFINED),
    # NOT STRING, so `expr {$v + 1}` must NOT fire S100.
    variable v
    return [expr {$v + 1}]
}
"""


def test_FP_SH_02_variable_alias_no_shimmer():
    """FP: `variable v` is a scope-alias — its intrep is externally determined
    so it cannot be confidently typed STRING.  S100 here would be the bug fixed
    by commit adfc6d84."""
    assert _codes(FP_SH_02_REPRO, SHIMMER_CODES) == [], (
        "scope-alias declared with `variable` must be typed OVERDEFINED, not STRING"
    )


def test_FP_SH_02_global_alias_no_shimmer():
    """The same principle applies to `global` and `upvar` aliases."""
    src_global = "proc f {} { global g\n return [expr {$g + 1}] }"
    assert _codes(src_global, SHIMMER_CODES) == [], "`global` alias must be OVERDEFINED too"

    src_upvar = "proc f {} { upvar 1 src dst\n return [expr {$dst + 1}] }"
    assert _codes(src_upvar, SHIMMER_CODES) == [], "`upvar` alias must be OVERDEFINED too"


# FP-SH-03 — phi joins are hash-seed-independent (determinism property)


FP_SH_03_REPRO = """\
proc f {n} {
    # x is joined at the loop header from two INT branches; the join
    # must come out INT every run (no flake) -> no S101.
    set x 0
    for {set i 0} {$i < $n} {incr i} {
        if {$i > 5} { set x 1 } else { set x 2 }
    }
    return [expr {$x + 1}]
}
"""


def test_FP_SH_03_phi_join_deterministic():
    """FP / determinism: a phi-merged value where both incoming branches are
    INT must come out INT — no S101 from an unsound STRING join.  The pre-fix
    join was set-iteration-order-dependent (PYTHONHASHSEED-flaky)."""
    assert _codes(FP_SH_03_REPRO, SHIMMER_CODES) == [], (
        "INT-INT phi join must come out INT; no shimmer here"
    )


def test_FP_SH_03_genuine_phi_string_int_still_fires():
    """TP control: a loop that reassigns `x` to a STRING on one arm and an INT
    on the other genuinely thrashes its internal rep per iteration, then feeds
    `$x` into `expr` — exactly the per-iteration string↔int shimmer the loop
    detectors exist to flag.  This pins that the accumulator-FP suppression
    (FP-SH-01) did NOT over-suppress a real oscillation."""
    src = (
        "proc f {n} {\n"
        "    set x 0\n"
        "    for {set i 0} {$i < $n} {incr i} {\n"
        '        if {$i > 5} { set x "hello" } else { set x 2 }\n'
        "    }\n"
        "    return [expr {$x + 1}]\n"
        "}\n"
    )
    # A genuine per-iteration loop shimmer (S101/S102) must still fire here.
    codes = {d.code for d in get_diagnostics(src)}
    assert codes & {"S100", "S101", "S102"}, codes
