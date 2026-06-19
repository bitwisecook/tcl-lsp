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


def _irule_codes(source: str, codes: list[str]):
    """Diagnostics matching any code in *codes*, analysed in the f5-irules
    dialect (``*::payload`` source/sink recognition is dialect-gated)."""
    from compiler.registry.dialect import dialect_scope

    with dialect_scope("f5-irules"):
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


# FP-SH-04 — hex/binary integer literals typed as INT (not STRING)


FP_SH_04_REPRO = """\
proc f {} {
    # 0x80 is a Tcl hex integer literal -- typed INT, not STRING.
    # incr on $n must NOT fire S100/S101.
    set n 0x80
    for {set i 0} {$i < 10} {incr i} {
        incr n
    }
    return $n
}
"""


def test_FP_SH_04_hex_literal_increment_no_shimmer():
    """FP: hex literal `0x80` is recognised as INT by Tcl_GetIntFromObj;
    the analyser's `_literal_type` must also recognise it as INT so
    incr on a hex-init variable doesn't fire spurious S100/S101."""
    assert _codes(FP_SH_04_REPRO, SHIMMER_CODES) == [], (
        "hex literal 0x80 must be typed INT; incr loop here is clean"
    )


def test_FP_SH_04_binary_literal_increment_no_shimmer():
    """FP: binary literal `0b1010` is also a Tcl integer literal."""
    src = "proc f {} { set n 0b1010; incr n; return $n }"
    assert _codes(src, SHIMMER_CODES) == [], "binary literal 0b1010 must be typed INT"


def test_FP_SH_04_genuine_string_increment_still_fires():
    """TP control: an actual string in incr must still fire shimmer.
    Proves the hex/binary recognition didn't blanket-silence STRING-in-incr."""
    src = "proc f {} { set n hello; incr n; return $n }"
    # The genuine STRING-in-incr case must still produce S100/S101.
    codes = {d.code for d in get_diagnostics(src)}
    assert codes & {"S100", "S101"}, f"STRING in incr must still fire; got {codes}"


# FP-SH-05 — destructure foreach (`foreach VARS LIST break`) excluded from loop body types


FP_SH_05_REPRO = """\
proc f {state} {
    # foreach + break is the pre-8.5 lassign equivalent: a
    # single-iteration foreach that destructures a list.  The var
    # bindings are one-time, not per-iteration body types -- they
    # must NOT pollute a sibling real loop's S102 oscillation check.
    foreach {a b c sv} $state break
    foreach inst {1 2 3} {
        set sv [list 1 2 3]
    }
    return $sv
}
"""


def test_FP_SH_05_destructure_foreach_no_s102():
    """FP: `foreach VARS LIST break` is the pre-`lassign` destructure
    idiom -- the binding runs once, not per iteration.  The main loop's
    `set sv [list ...]` is monomorphic in LIST; with the destructure
    excluded from `loop_body_types`, S102 on $sv must NOT fire."""
    codes = [d.code for d in get_diagnostics(FP_SH_05_REPRO)]
    assert "S102" not in codes, f"destructure-foreach must not pollute S102: {codes}"


def test_FP_SH_05_real_iter_foreach_still_fires():
    """TP control: a real multi-iteration foreach whose body genuinely
    oscillates types must still fire S102.  The destructure detection
    keys on body=`break`, not foreach-as-such, so this stays caught."""
    src = (
        "proc f {items} {\n"
        "    foreach x $items {\n"
        '        if {$x > 0} { set v "string" } else { set v [list 1 2] }\n'
        "        puts $v\n"
        "    }\n"
        "}\n"
    )
    codes = {d.code for d in get_diagnostics(src)}
    assert codes & {"S101", "S102"}, f"real per-iter oscillation must still fire; got {codes}"


# FP-SH-06 — per-loop body_types (sibling loops do not pollute each other)


FP_SH_06_REPRO = """\
proc f {items} {
    # Loop A and loop B are SIBLINGS -- they don't nest.  Each loop
    # alone is monomorphic in $x (loop A: STRING only; loop B: LIST
    # only).  Neither loop oscillates, so S102 must not fire on either.
    foreach a $items {
        set x "value"
    }
    foreach b $items {
        set x [list 1 2]
    }
}
"""


def test_FP_SH_06_sibling_loops_no_s102():
    """FP: sibling loops in the same proc must not pollute each other's
    S102 body_types map.  Loop A binds $x to STRING only; loop B binds
    $x to LIST only -- per-loop body_types is monomorphic in each, so
    no oscillation, no S102.  Pre-fix the function-wide union triggered
    S102 on both loops' phi for $x."""
    codes = [d.code for d in get_diagnostics(FP_SH_06_REPRO)]
    assert "S102" not in codes, f"sibling-loop S102 should not fire: {codes}"


def test_FP_SH_06_real_oscillation_within_one_loop_still_fires():
    """TP control: when a SINGLE loop's body genuinely oscillates $x's
    type per iteration, S102 must still fire.  Proves the per-loop
    body_types didn't over-suppress real intra-loop oscillation."""
    src = (
        "proc f {items} {\n"
        "    foreach a $items {\n"
        '        if {$a > 0} { set x "value" } else { set x [list 1 2] }\n'
        "        puts $x\n"
        "    }\n"
        "}\n"
    )
    codes = {d.code for d in get_diagnostics(src)}
    assert codes & {"S101", "S102"}, f"real intra-loop oscillation must still fire; got {codes}"


# FP-SH-08 — ``==``/``!=`` falsely flagged as numeric shimmer when both
# operands are provably non-numeric (D5-SH-EQ).


def test_FP_SH_08_eq_both_non_numeric_no_shimmer():
    """FP: ``expr {$s == "hello"}`` with ``s = "hello"`` -- both operands
    are non-numeric text.  tclsh takes the STRING compare path (no
    coercion attempt), so no shimmer occurs.

    tclsh 9.0.3::

        % set s hello
        % expr {$s == "hello"}
        1
        % # Both operands non-numeric -> string path -- no intrep change.

    Pre-fix S100 fired because ``==`` was in ``_NUMERIC_OPS``.  Post-fix
    it sits in ``_CONDITIONAL_NUMERIC_OPS`` and the predicate requires
    at least one operand to be provably numeric-looking.
    """
    src = 'proc f {} { set s [string trim hello]; set y [expr {$s == "world"}]; puts $y }'
    codes = [d.code for d in get_diagnostics(src)]
    assert "S100" not in codes and "S101" not in codes, (
        f"both-non-numeric == must not fire shimmer; got {codes}"
    )


def test_FP_SH_08_eq_with_numeric_literal_still_fires():
    """TP control: ``expr {$s == "5"}`` with ``s`` STRING-typed -- the
    other operand parses as numeric, so tclsh attempts the numeric path
    and coerces $s.  Genuine shimmer."""
    src = 'proc f {} { set s [string trim hello]; set y [expr {$s == "5"}]; puts $y }'
    codes = [d.code for d in get_diagnostics(src)]
    assert "S100" in codes or "S101" in codes, (
        f"==/numeric-literal mix must still fire shimmer; got {codes}"
    )


def test_FP_SH_08_add_still_fires():
    """TP control: ``expr {$s + 0}`` always takes numeric path, regardless
    of operand types.  ADD is in the unconditional ``_NUMERIC_OPS``."""
    src = 'proc f {} { set s [string trim "5"]; set y [expr {$s + 0}]; puts $y }'
    codes = [d.code for d in get_diagnostics(src)]
    assert "S100" in codes or "S101" in codes, f"$s + 0 must still fire shimmer; got {codes}"


# FP-SH-07 — `_find_expr_shimmers` covers standalone `expr`, `if {…}`,
# `while {…}`, `for ... {...} ...` expr contexts (D5-SH-EXPR).


def test_FP_SH_07_if_condition_shimmer_fires():
    """TP: ``if {$s + 1}`` -- expr in if-cond promotes $s from STRING to
    INT.  Pre-fix `_find_expr_shimmers` only walked IRAssignExpr, missing
    the CFGBranch.condition expr.

    tclsh 9.0.3::

        % set s [string trim "5"]
        % tcl::unsupported::representation $s
        value is a pure string ...
        % if {$s + 1} { puts yes }
        yes
        % tcl::unsupported::representation $s
        value is a int ...
    """
    src = 'proc f {} { set s [string trim "5"]; if {$s + 1} { puts yes } }'
    codes = [d.code for d in get_diagnostics(src)]
    assert "S100" in codes, f"if-condition shimmer must fire; got {codes}"


def test_FP_SH_07_while_condition_shimmer_fires():
    """TP: ``while {$s + 1}`` -- expr in while-cond promotes $s.  In a
    loop context fires S101 (loop shimmer)."""
    src = 'proc f {} { set s [string trim "5"]; while {$s + 1} { break } }'
    codes = [d.code for d in get_diagnostics(src)]
    assert "S101" in codes or "S100" in codes, f"while-condition shimmer must fire; got {codes}"


def test_FP_SH_07_standalone_expr_shimmer_fires():
    """TP: standalone ``expr {$s + 1}`` (result dropped) is also a real
    lex-promotion site -- the expr command still evaluates and coerces."""
    src = 'proc f {} { set s [string trim "5"]; expr {$s + 1} }'
    codes = [d.code for d in get_diagnostics(src)]
    assert "S100" in codes, f"standalone-expr shimmer must fire; got {codes}"


def test_FP_SH_07_pure_numeric_if_no_shimmer():
    """TN: ``if {1 + 1}`` -- both operands are numeric literals, no var
    is involved, no shimmer."""
    src = "proc f {} { if {1 + 1} { puts yes } }"
    codes = [d.code for d in get_diagnostics(src)]
    assert "S100" not in codes and "S101" not in codes, (
        f"pure-numeric if must not fire shimmer; got {codes}"
    )


# FP-SH-09 — byte array case-folded / re-encoded by a string op corrupts high bytes (S110)


FP_SH_09_REPRO = """\
# binary format -> string toupper -> corrupted bytes (S110).
set ba [binary format c* {128 195 255}]
set up [string toupper $ba]
"""


def test_FP_SH_09_toupper_byte_array_fires():
    """TP: ``string toupper`` on a ``binary format`` byte array reinterprets
    bytes as Unicode and mangles every byte >= 0x80 — a correctness shimmer
    (S110, Warning).  Ground truth: tclsh 8.6 silently corrupts 80c3ff->80c378,
    tclsh 9.0 raises on the byte scan-back."""
    assert _codes(FP_SH_09_REPRO, ["S110"]), (
        "byte array case-folded by string toupper must fire S110"
    )


def test_FP_SH_09_toupper_plain_string_silent():
    """FP control: ``string toupper`` on a plain string has no byte source, so
    S110 must stay silent (proves S110 is provenance-gated, not blanket)."""
    src = 'set s "hello world"\nset u [string toupper $s]\n'
    assert _codes(src, ["S110"]) == [], "string op on a non-binary value must not fire S110"


# FP-SH-10 — *::payload round-trip: string-coerced binary written back corrupts it (S110)


FP_SH_10_REPRO = """\
when HTTP_REQUEST_DATA {
    set original_data [HTTP::payload]
    set new_data "$original_data MODIFIED"
    HTTP::payload replace 0 100 $new_data
}
"""


def test_FP_SH_10_payload_roundtrip_fires():
    """TP: HTTP::payload read, string-interpolated, then written back with
    HTTP::payload replace — the canonical F5 KB K22406348 double-encode bug."""
    assert _irule_codes(FP_SH_10_REPRO, ["S110"]), "payload string round-trip must fire S110"


def test_FP_SH_10_clean_payload_writeback_silent():
    """FP control: writing the byte array straight back (no string coercion)
    is binary-safe — S110 must stay silent."""
    src = (
        "when HTTP_REQUEST_DATA {\n"
        "    set original_data [HTTP::payload]\n"
        "    HTTP::payload replace 0 100 $original_data\n"
        "}\n"
    )
    assert _irule_codes(src, ["S110"]) == [], "clean byte-array writeback must not fire S110"


def test_FP_SH_10_binary_scan_fix_silent():
    """FP control: the documented fix — ``binary scan $v c* -`` re-binarifies
    the value before the sink — must clear S110."""
    src = (
        "when HTTP_REQUEST_DATA {\n"
        "    set original_data [HTTP::payload]\n"
        '    set new_data "$original_data MODIFIED"\n'
        "    binary scan $new_data c* -\n"
        "    HTTP::payload replace 0 100 $new_data\n"
        "}\n"
    )
    assert _irule_codes(src, ["S110"]) == [], "documented binary-scan fix must not fire S110"
