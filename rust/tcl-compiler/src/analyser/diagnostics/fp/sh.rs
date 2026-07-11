// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! SH family — shimmer (S100/S101/S102) FP/TP catalogue.
//! Pairs to `tests/test_fp_sh.py` and the §SH entries in FP.md.

use super::{D, codes, fires};

// ---------------------------------------------------------------------------
// FP-SH-01 — OVERDEFINED values do not trigger shimmer
// ---------------------------------------------------------------------------

/// FP-SH-01: OVERDEFINED value (cmd return) in arithmetic must NOT fire any
/// shimmer code — the type is unknown so any verdict would be unsound.
#[test]
fn fp_sh_01_overdefined_silent() {
    let src = "\
# x has unknown type (cmd return) -> OVERDEFINED -> no shimmer warning.
set x [unknownCmd]
set y [expr {$x + 1}]
return $y
";
    let got = codes(src, D);
    assert!(
        !got.iter()
            .any(|c| c == "S100" || c == "S101" || c == "S102"),
        "FP-SH-01: OVERDEFINED value should not produce a shimmer warning; got {got:?}"
    );
}

/// FP-SH-01 TP control: a *known* STRING value in arithmetic must still fire
/// S100 — proves the OVERDEFINED suppression isn't blanket-silencing all shimmer.
#[test]
fn fp_sh_01_string_arith_still_fires() {
    let src = "set s hello\nset y [expr {$s + 1}]";
    assert!(
        fires(src, D, "S100"),
        "FP-SH-01 TP: known STRING in arithmetic must still fire S100; got {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-SH-02 — scope-alias declarations typed OVERDEFINED (not STRING)
// ---------------------------------------------------------------------------

/// FP-SH-02: `variable v` is a scope-alias — its intrep is externally
/// determined so it cannot be confidently typed STRING.  S100 here would be
/// the bug fixed by commit adfc6d84.
#[test]
fn fp_sh_02_variable_alias_no_shimmer() {
    let src = "\
proc f {} {
    # `variable v` declares an alias — type is unknown (OVERDEFINED),
    # NOT STRING, so `expr {$v + 1}` must NOT fire S100.
    variable v
    return [expr {$v + 1}]
}
";
    let got = codes(src, D);
    assert!(
        !got.iter()
            .any(|c| c == "S100" || c == "S101" || c == "S102"),
        "FP-SH-02: scope-alias declared with `variable` must be typed OVERDEFINED, not STRING; got {got:?}"
    );
}

/// FP-SH-02: the same principle applies to `global` and `upvar` aliases.
#[test]
fn fp_sh_02_global_alias_no_shimmer() {
    let src_global = "proc f {} { global g\n return [expr {$g + 1}] }";
    let got_g = codes(src_global, D);
    assert!(
        !got_g
            .iter()
            .any(|c| c == "S100" || c == "S101" || c == "S102"),
        "FP-SH-02: `global` alias must be OVERDEFINED too; got {got_g:?}"
    );

    let src_upvar = "proc f {} { upvar 1 src dst\n return [expr {$dst + 1}] }";
    let got_u = codes(src_upvar, D);
    assert!(
        !got_u
            .iter()
            .any(|c| c == "S100" || c == "S101" || c == "S102"),
        "FP-SH-02: `upvar` alias must be OVERDEFINED too; got {got_u:?}"
    );
}

// ---------------------------------------------------------------------------
// FP-SH-03 — phi joins are hash-seed-independent (determinism property)
// ---------------------------------------------------------------------------

/// FP-SH-03: a phi-merged value where both incoming branches are INT must
/// come out INT — no S101 from an unsound STRING join.
#[test]
fn fp_sh_03_phi_join_deterministic() {
    let src = "\
proc f {n} {
    # x is joined at the loop header from two INT branches; the join
    # must come out INT every run (no flake) -> no S101.
    set x 0
    for {set i 0} {$i < $n} {incr i} {
        if {$i > 5} { set x 1 } else { set x 2 }
    }
    return [expr {$x + 1}]
}
";
    let got = codes(src, D);
    assert!(
        !got.iter()
            .any(|c| c == "S100" || c == "S101" || c == "S102"),
        "FP-SH-03: INT-INT phi join must come out INT; no shimmer here; got {got:?}"
    );
}

/// FP-SH-03 TP control: a loop that reassigns `x` to a STRING on one arm
/// and an INT on the other must still fire shimmer.
#[test]
fn fp_sh_03_genuine_phi_string_int_still_fires() {
    let src = concat!(
        "proc f {n} {\n",
        "    set x 0\n",
        "    for {set i 0} {$i < $n} {incr i} {\n",
        "        if {$i > 5} { set x \"hello\" } else { set x 2 }\n",
        "    }\n",
        "    return [expr {$x + 1}]\n",
        "}\n"
    );
    let got = codes(src, D);
    assert!(
        got.iter()
            .any(|c| c == "S100" || c == "S101" || c == "S102"),
        "FP-SH-03 TP: genuine per-iteration loop shimmer must still fire; got {got:?}"
    );
}

// ---------------------------------------------------------------------------
// FP-SH-04 — hex/binary integer literals typed as INT (not STRING)
// ---------------------------------------------------------------------------

/// FP-SH-04: hex literal `0x80` is recognised as INT by `Tcl_GetIntFromObj`;
/// the analyser must also recognise it as INT so incr on a hex-init variable
/// doesn't fire spurious S100/S101.
#[test]
fn fp_sh_04_hex_literal_increment_no_shimmer() {
    let src = "\
proc f {} {
    # 0x80 is a Tcl hex integer literal -- typed INT, not STRING.
    # incr on $n must NOT fire S100/S101.
    set n 0x80
    for {set i 0} {$i < 10} {incr i} {
        incr n
    }
    return $n
}
";
    let got = codes(src, D);
    assert!(
        !got.iter()
            .any(|c| c == "S100" || c == "S101" || c == "S102"),
        "FP-SH-04: hex literal 0x80 must be typed INT; incr loop here is clean; got {got:?}"
    );
}

/// FP-SH-04: binary literal `0b1010` is also a Tcl integer literal.
#[test]
fn fp_sh_04_binary_literal_increment_no_shimmer() {
    let src = "proc f {} { set n 0b1010; incr n; return $n }";
    let got = codes(src, D);
    assert!(
        !got.iter()
            .any(|c| c == "S100" || c == "S101" || c == "S102"),
        "FP-SH-04: binary literal 0b1010 must be typed INT; got {got:?}"
    );
}

/// FP-SH-04 TP control: an actual string in incr must still fire shimmer.
#[test]
fn fp_sh_04_genuine_string_increment_still_fires() {
    let src = "proc f {} { set n hello; incr n; return $n }";
    let got = codes(src, D);
    assert!(
        got.iter().any(|c| c == "S100" || c == "S101"),
        "FP-SH-04 TP: STRING in incr must still fire; got {got:?}"
    );
}

// ---------------------------------------------------------------------------
// FP-SH-05 — destructure foreach (`foreach VARS LIST break`) excluded from
// loop body types
// ---------------------------------------------------------------------------

/// FP-SH-05: `foreach VARS LIST break` is the pre-`lassign` destructure
/// idiom — the binding runs once, not per iteration.  S102 on $sv must NOT
/// fire.
#[test]
fn fp_sh_05_destructure_foreach_no_s102() {
    let src = "\
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
";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S102"),
        "FP-SH-05: destructure-foreach must not pollute S102; got {got:?}"
    );
}

/// FP-SH-05 TP control: a real multi-iteration foreach whose body genuinely
/// oscillates types must still fire S102.
#[test]
fn fp_sh_05_real_iter_foreach_still_fires() {
    let src = concat!(
        "proc f {items} {\n",
        "    foreach x $items {\n",
        "        if {$x > 0} { set v \"string\" } else { set v [list 1 2] }\n",
        "        puts $v\n",
        "    }\n",
        "}\n"
    );
    let got = codes(src, D);
    assert!(
        got.iter().any(|c| c == "S101" || c == "S102"),
        "FP-SH-05 TP: real per-iter oscillation must still fire; got {got:?}"
    );
}

// ---------------------------------------------------------------------------
// FP-SH-06 — per-loop body_types (sibling loops do not pollute each other)
// ---------------------------------------------------------------------------

/// FP-SH-06: sibling loops in the same proc must not pollute each other's
/// S102 `body_types` map.
#[test]
fn fp_sh_06_sibling_loops_no_s102() {
    let src = "\
proc f {items} {
    # Loop A and loop B are SIBLINGS -- they don't nest.  Each loop
    # alone is monomorphic in $x (loop A: STRING only; loop B: LIST
    # only).  Neither loop oscillates, so S102 must not fire on either.
    foreach a $items {
        set x \"value\"
    }
    foreach b $items {
        set x [list 1 2]
    }
}
";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S102"),
        "FP-SH-06: sibling-loop S102 should not fire; got {got:?}"
    );
}

/// FP-SH-06 TP control: when a SINGLE loop's body genuinely oscillates $x's
/// type per iteration, S102 must still fire.
#[test]
fn fp_sh_06_real_oscillation_within_one_loop_still_fires() {
    let src = concat!(
        "proc f {items} {\n",
        "    foreach a $items {\n",
        "        if {$a > 0} { set x \"value\" } else { set x [list 1 2] }\n",
        "        puts $x\n",
        "    }\n",
        "}\n"
    );
    let got = codes(src, D);
    assert!(
        got.iter().any(|c| c == "S101" || c == "S102"),
        "FP-SH-06 TP: real intra-loop oscillation must still fire; got {got:?}"
    );
}

// ---------------------------------------------------------------------------
// FP-SH-07 — `_find_expr_shimmers` covers standalone expr/if/while/for
// expr contexts (D5-SH-EXPR)
// ---------------------------------------------------------------------------

/// FP-SH-07 TP: `if {$s + 1}` — expr in if-cond promotes $s from STRING to
/// INT.  Pre-fix `_find_expr_shimmers` only walked `IRAssignExpr`, missing the
/// CFGBranch.condition expr.
#[test]
fn fp_sh_07_if_condition_shimmer_fires() {
    let src = r#"proc f {} { set s [string trim "5"]; if {$s + 1} { puts yes } }"#;
    assert!(
        fires(src, D, "S100"),
        "FP-SH-07 TP: if-condition shimmer must fire; got {:?}",
        codes(src, D)
    );
}

/// FP-SH-07 TP: `while {$s + 1}` — expr in while-cond promotes $s.
#[test]
fn fp_sh_07_while_condition_shimmer_fires() {
    let src = r#"proc f {} { set s [string trim "5"]; while {$s + 1} { break } }"#;
    let got = codes(src, D);
    assert!(
        got.iter().any(|c| c == "S101" || c == "S100"),
        "FP-SH-07 TP: while-condition shimmer must fire; got {got:?}"
    );
}

/// FP-SH-07 TP: standalone `expr {$s + 1}` (result dropped) is a real
/// lex-promotion site.
#[test]
fn fp_sh_07_standalone_expr_shimmer_fires() {
    let src = r#"proc f {} { set s [string trim "5"]; expr {$s + 1} }"#;
    assert!(
        fires(src, D, "S100"),
        "FP-SH-07 TP: standalone-expr shimmer must fire; got {:?}",
        codes(src, D)
    );
}

/// FP-SH-07 TN: `if {1 + 1}` — both operands are numeric literals, no var
/// is involved, no shimmer.
#[test]
fn fp_sh_07_pure_numeric_if_no_shimmer() {
    let src = "proc f {} { if {1 + 1} { puts yes } }";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S100" || c == "S101"),
        "FP-SH-07: pure-numeric if must not fire shimmer; got {got:?}"
    );
}

// ---------------------------------------------------------------------------
// FP-SH-08 — `==`/`!=` falsely flagged as numeric shimmer when both
// operands are provably non-numeric (D5-SH-EQ)
// ---------------------------------------------------------------------------

/// FP-SH-08: `expr {$s == "hello"}` with `s = "hello"` — both operands are
/// non-numeric text.  tclsh takes the STRING compare path (no coercion
/// attempt), so no shimmer occurs.
#[test]
fn fp_sh_08_eq_both_non_numeric_no_shimmer() {
    let src = r#"proc f {} { set s [string trim hello]; set y [expr {$s == "world"}]; puts $y }"#;
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S100" || c == "S101"),
        "FP-SH-08: both-non-numeric == must not fire shimmer; got {got:?}"
    );
}

/// FP-SH-08 TP control: `expr {$s == "5"}` with `s` STRING-typed — the
/// other operand parses as numeric, so tclsh attempts the numeric path and
/// coerces $s.  Genuine shimmer.
#[test]
fn fp_sh_08_eq_with_numeric_literal_still_fires() {
    let src = r#"proc f {} { set s [string trim hello]; set y [expr {$s == "5"}]; puts $y }"#;
    let got = codes(src, D);
    assert!(
        got.iter().any(|c| c == "S100" || c == "S101"),
        "FP-SH-08 TP: ==/numeric-literal mix must still fire shimmer; got {got:?}"
    );
}

/// FP-SH-08 TP control: `expr {$s + 0}` always takes numeric path regardless
/// of operand types.  ADD is in the unconditional `_NUMERIC_OPS`.
#[test]
fn fp_sh_08_add_still_fires() {
    let src = r#"proc f {} { set s [string trim "5"]; set y [expr {$s + 0}]; puts $y }"#;
    let got = codes(src, D);
    assert!(
        got.iter().any(|c| c == "S100" || c == "S101"),
        "FP-SH-08 TP: $s + 0 must still fire shimmer; got {got:?}"
    );
}

// ---------------------------------------------------------------------------
// FP-SH-12 — a `trace add variable … write …` callback can rewrite a value's
// type on every access, so a traced variable's `set`-only literal types must
// not drive S102
// ---------------------------------------------------------------------------

/// FP-SH-12: a variable under a write trace must not fire S102 from its
/// visible `set` literals alone — the trace callback can rewrite the value
/// (and therefore its intrep) after every write, so the compiler cannot
/// prove the two literal types are what the variable actually holds at loop
/// re-entry.  [`tcl_compiler::type_infer::propagate_types`] forces every def
/// of a traced name to OVERDEFINED via the module-wide
/// `var_observability::ModuleVariableTraces` fact — the same one SCCP's own
/// (separate) constant-folding lattice already consumes.
#[test]
fn fp_sh_12_traced_variable_no_s102() {
    let src = "\
proc f {} {
    # x is under a write trace -- its value after any `set` is not
    # provably what the literal says, so the loop below must NOT fire
    # S102 even though the visible literals alternate int/string.
    trace add variable x write {apply {{n1 n2 op} {}}}
    set x 0
    while {1} {
        set x [expr {$x + 1}]
        set x [string range $x 0 end]
    }
}
";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S102"),
        "FP-SH-12: traced variable must not fire S102; got {got:?}"
    );
}

/// FP-SH-12 TP control: the identical loop shape, untraced, must still fire
/// S102 — proves the trace check isn't blanket-silencing the whole family.
#[test]
fn fp_sh_12_untraced_control_still_fires() {
    let src = "\
proc f {} {
    set x 0
    while {1} {
        set x [expr {$x + 1}]
        set x [string range $x 0 end]
    }
}
";
    assert!(
        fires(src, D, "S102"),
        "FP-SH-12 TP: untraced control must still fire S102; got {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-SH-13 — array-element writes collapse onto one SSA symbol per array;
// two individually-stable-but-different elements must not be reported as
// one variable oscillating
// ---------------------------------------------------------------------------

/// FP-SH-13: `normalise_var_name` strips the `(key)` suffix before SSA
/// interning, so every element of an array shares one symbol / version
/// chain.  A loop that keeps writing different-but-per-element-stable types
/// to different array elements must not fire S102 — the elements are
/// independent runtime slots, not the same value shimmering back and forth.
#[test]
fn fp_sh_13_array_element_conflation_no_s102() {
    let src = "\
proc f {} {
    set arr(x) 0
    while {1} {
        set arr(x) [expr {$arr(x) + 1}]
        set arr(x) [string range $arr(x) 0 end]
    }
}
";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S102"),
        "FP-SH-13: array-element writes must not fire S102; got {got:?}"
    );
}

/// FP-SH-13 TP control: the identical oscillation on a plain scalar (not an
/// array element) must still fire S102.
#[test]
fn fp_sh_13_scalar_control_still_fires() {
    let src = "\
proc f {} {
    set x 0
    while {1} {
        set x [expr {$x + 1}]
        set x [string range $x 0 end]
    }
}
";
    assert!(
        fires(src, D, "S102"),
        "FP-SH-13 TP: plain scalar oscillation must still fire S102; got {:?}",
        codes(src, D)
    );
}

/// FP-SH-13 (extended to S100): the S102 array-element guard now also covers
/// the use-site pass. `set arr(n) 5; set arr(label) "text"; incr arr(n)` reads
/// the conflated `arr` symbol as STRING (the last element written), but
/// `arr(n)` is always INT — an independent slot, not a shimmer. No S100.
#[test]
fn fp_sh_13_array_element_use_site_no_s100() {
    let src = "\
proc f {} {
    set arr(n) 5
    set arr(label) \"text\"
    incr arr(n)
}
";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S100" || c == "S101"),
        "FP-SH-13: array-element use-site must not fire S100/S101; got {got:?}"
    );
}

/// FP-SH-13 (extended to S101): the same conflation inside a loop must not
/// fire the per-iteration S101 either.
#[test]
fn fp_sh_13_array_element_loop_use_site_no_s101() {
    let src = "\
proc f {items} {
    set arr(n) 5
    foreach i $items {
        set arr(label) \"text\"
        incr arr(n)
    }
}
";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S100" || c == "S101"),
        "FP-SH-13: array-element loop use-site must not fire S101; got {got:?}"
    );
}

/// FP-SH-13 (extended to the phi pass): a branch merge of two different array
/// elements must not phi-shimmer the conflated base symbol.
#[test]
fn fp_sh_13_array_element_phi_merge_no_s100() {
    let src = "\
proc f {c} {
    if {$c} {
        set arr(n) 5
    } else {
        set arr(s) \"hi\"
    }
    return [string length $arr(n)]
}
";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S100" || c == "S101"),
        "FP-SH-13: array-element phi merge must not fire S100/S101; got {got:?}"
    );
}

/// FP-SH-13 TP control for the use-site guard: a plain scalar with the same
/// int→string→incr shape still fires — the array guard is keyed on array-base
/// symbols only, not a blanket use-site suppression.
#[test]
fn fp_sh_13_scalar_use_site_still_fires() {
    let src = "\
proc f {} {
    set n hello
    incr n
}
";
    assert!(
        fires(src, D, "S100"),
        "FP-SH-13 TP: plain scalar use-site shimmer must still fire S100; got {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-SH-14 — a self-referential variable that oscillates through an
// intermediate branch merge (not a direct loop-header incoming edge) is
// still genuine thunking; a non-self-referential reset through the same
// branch shape is not
// ---------------------------------------------------------------------------

/// FP-SH-14 TP: `if {...} {set x [string range $x 0 end]} else {set x [list
/// 1 2]}` inside a loop — one branch reads `$x`'s own prior value, so the
/// variable is self-referential and genuinely oscillates between whichever
/// type survived the previous pass and whichever branch runs next.  The
/// direct loop-internal predecessor into the header phi is itself a
/// SHIMMERED merge (the if/else join), not a single KNOWN type — this must
/// still fire S102.
#[test]
fn fp_sh_14_self_referential_branchy_oscillation_fires() {
    let src = "\
proc f {n} {
    set x \"seed\"
    while {$n} {
        if {$n % 2} {
            set x [string range $x 0 end]
        } else {
            set x [list 1 2]
        }
        incr n -1
    }
    return $x
}
";
    assert!(
        fires(src, D, "S102"),
        "FP-SH-14 TP: self-referential branchy oscillation must fire S102; got {:?}",
        codes(src, D)
    );
}

/// FP-SH-14 FP guard: the same branch shape, but *neither* arm reads `$x` —
/// both assign an unrelated fresh literal.  The variable is not
/// self-referential: each pass creates a brand-new, unrelated value with
/// nothing to reinterpret, so this must NOT fire S102 even though the
/// header phi is still SHIMMERED and the direct predecessor is still a
/// merge.
#[test]
fn fp_sh_14_non_self_referential_branchy_reset_no_s102() {
    let src = "\
proc f {n} {
    set x \"seed\"
    while {$n} {
        if {$n % 2} {
            set x \"value\"
        } else {
            set x [list 1 2]
        }
        incr n -1
    }
    return $x
}
";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S102"),
        "FP-SH-14: non-self-referential branchy reset must not fire S102; got {got:?}"
    );
}

// ---------------------------------------------------------------------------
// FP-SH-15 — tricky command/variable indirection stays silent (conservative,
// no false positives): `rename`, `interp alias`, a safe sub-interpreter's
// `eval`, TclOO instance variables, and `args`/positional parameters as the
// oscillation seed
// ---------------------------------------------------------------------------

/// FP-SH-15 (detection gap now closed): a command renamed onto `set`
/// (`rename set myset`) resolves back to the registry's `set` spec. The
/// lowerer records the rename in the same binding snapshot it uses for
/// `interp alias` (so the store carries `canonical_command = set`), and
/// `type_infer` types the def from its value word while `ssa::uses_of` scans
/// that value like the un-aliased `set` — so the loop-carried int/string
/// oscillation forms a header phi and fires S102. Genuine thunk, not a false
/// positive: `myset` *is* `set`, so the per-iteration re-conversion is real
/// (verified against tclsh — the renamed store behaves identically to `set`).
#[test]
fn fp_sh_15_rename_indirection_fires_s102() {
    let src = "\
proc f {} {
    rename set myset
    myset x 0
    while {1} {
        myset x [expr {$x + 1}]
        myset x [string range $x 0 end]
    }
}
";
    assert!(
        fires(src, D, "S102"),
        "FP-SH-15: rename-onto-set oscillation must resolve to `set` and fire S102; got {:?}",
        codes(src, D)
    );
}

/// FP-SH-15: a `rename` inside `namespace eval ::ns` binds NEW in *that*
/// namespace (`::ns::myset`), not globally — so the namespace-qualified call
/// resolves to `set` and fires S102. Regression for the Codex review point
/// that qualifying NEW with `normalise_qualified_name` alone recorded a global
/// `::myset` (missing the real `::ns::myset` calls, and mis-analysing a global
/// `myset` that Tcl considers invalid).
#[test]
fn fp_sh_15_namespaced_rename_indirection_fires_s102() {
    let src = "\
namespace eval ::ns {
    rename set myset
    proc f {} {
        myset x 0
        while {1} {
            myset x [expr {$x + 1}]
            myset x [string range $x 0 end]
        }
    }
}
";
    assert!(
        fires(src, D, "S102"),
        "FP-SH-15: a namespace-relative rename must bind ::ns::myset and fire S102; got {:?}",
        codes(src, D)
    );
}

/// FP-SH-15 (detection gap now closed): `interp alias {} myset {} set` is the
/// same indirection through the interpreter's alias table rather than
/// `rename` — the lowerer already records it, so threading
/// `canonical_command` through `type_infer` / `ssa` closes it identically to
/// the rename case. S102 fires on the genuine oscillation.
#[test]
fn fp_sh_15_interp_alias_indirection_fires_s102() {
    let src = "\
interp alias {} myset {} set
proc f {} {
    myset x 0
    while {1} {
        myset x [expr {$x + 1}]
        myset x [string range $x 0 end]
    }
}
";
    assert!(
        fires(src, D, "S102"),
        "FP-SH-15: interp-alias-onto-set oscillation must resolve to `set` and fire S102; got {:?}",
        codes(src, D)
    );
}

/// FP-SH-15 TN control: an `interp alias` / `rename` onto a command that is
/// *not* a value-passthrough store must not gain a spurious oscillation. Here
/// `myputs` aliases `puts` (no def, no value passthrough), so the loop body
/// defines nothing and S102 cannot fire — proves the canonical-command
/// resolution is keyed on the real command's spec, not a blanket
/// "aliased command oscillates".
#[test]
fn fp_sh_15_alias_onto_non_store_no_s102() {
    let src = "\
interp alias {} myputs {} puts
proc f {} {
    set x 0
    while {1} {
        myputs [expr {$x + 1}]
        myputs [string range $x 0 end]
    }
}
";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S102"),
        "FP-SH-15: alias onto a non-store command must not fabricate S102; got {got:?}"
    );
}

/// FP-SH-15: a safe sub-interpreter's `$slave eval {...}` body is an opaque
/// string argument, not statically-analysed Tcl source — no S102 from
/// inside it (and no crash walking into it).
#[test]
fn fp_sh_15_safe_sub_interpreter_eval_no_s102() {
    let src = "\
proc f {} {
    set slave [interp create -safe]
    $slave eval {
        set x 0
        while {1} {
            set x [expr {$x + 1}]
            set x [string range $x 0 end]
        }
    }
}
";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S102"),
        "FP-SH-15: a safe sub-interpreter's eval body must not fire S102; got {got:?}"
    );
}

/// FP-SH-15: a `TclOO` instance variable declared with a bare `variable` in a
/// method body is a scope-alias declaration, like `global`/top-level
/// `variable` — its entry type is unknown, so an oscillating loop over it
/// must not fire S102.
#[test]
fn fp_sh_15_tcloo_instance_variable_no_s102() {
    let src = "\
oo::class create C {
    variable x
    constructor {} { set x 0 }
    method run {} {
        while {1} {
            set x [expr {$x + 1}]
            set x [string range $x 0 end]
        }
    }
}
";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S102"),
        "FP-SH-15: TclOO instance variable must not fire S102; got {got:?}"
    );
}

/// FP-SH-15: `my variable x` (the `TclOO` idiom for binding an instance
/// variable inside a method) is recognised as a scope-alias declaration —
/// `oo_my.rs`'s `variable` subcommand carries `creates_scope_alias: true`,
/// the same signal `global`/top-level `variable`/`upvar` use — so `$x`'s
/// entry type widens to `Overdefined`, not a confident type from either
/// arm. No S102 (and no crash).
#[test]
fn fp_sh_15_my_variable_idiom_no_s102() {
    let src = "\
oo::class create C {
    constructor {} { my variable x; set x 0 }
    method run {} {
        my variable x
        while {1} {
            set x [expr {$x + 1}]
            set x [string range $x 0 end]
        }
    }
}
";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S102"),
        "FP-SH-15: 'my variable' idiom must not fire S102; got {got:?}"
    );
}

/// FP-SH-15 (blind-spot fix, the case that was a genuine false positive):
/// `my variable x` *followed by a local `set x 0`* used to give the loop a
/// versioned `Known(Int)` entry and fire a spurious S102 — the instance
/// variable's intrep is externally determined (the constructor / other
/// methods can set it), so this must stay silent, the same protection a bare
/// `variable x; set x 0` already had (FP-SH-16).
#[test]
fn fp_sh_15_my_variable_locally_initialised_no_s102() {
    let src = "\
oo::class create C {
    method run {} {
        my variable x
        set x 0
        while {1} {
            set x [expr {$x + 1}]
            set x [string range $x 0 end]
        }
    }
}
";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S102"),
        "FP-SH-15: locally-initialised 'my variable' instance var must not fire S102; got {got:?}"
    );
}

/// FP-SH-15 TP control: an *unaliased* plain local in the same method, with
/// the identical oscillation, must still fire — proves the `my variable`
/// recognition is keyed on the declared instance-variable name, not a blanket
/// suppression of S102 inside method bodies.
#[test]
fn fp_sh_15_my_variable_unaliased_sibling_local_still_fires() {
    let src = "\
oo::class create C {
    method run {} {
        my variable x
        set local 0
        while {1} {
            set local [expr {$local + 1}]
            set local [string range $local 0 end]
        }
    }
}
";
    assert!(
        fires(src, D, "S102"),
        "FP-SH-15 TP: an unaliased sibling local in the method must still fire S102; got {:?}",
        codes(src, D)
    );
}

/// FP-SH-15: a proc parameter's entry type is unknown (the caller can pass
/// anything) — `propagate_types` forces a live-in (SSA version 0) to
/// OVERDEFINED, so a loop that oscillates a plain positional parameter must
/// not fire S102. Mirrors [`fp_sh_02_variable_alias_no_shimmer`]'s aliasing
/// rationale, but for an ordinary (unaliased) parameter.
#[test]
fn fp_sh_15_parameter_seeded_oscillation_no_s102() {
    let src = "\
proc f {p} {
    while {1} {
        set p [expr {$p + 1}]
        set p [string range $p 0 end]
    }
}
";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S102"),
        "FP-SH-15: parameter-seeded oscillation must not fire S102; got {got:?}"
    );
}

/// FP-SH-15: an `unknown`-command call (undefined at compile time) types
/// OVERDEFINED, exactly like the FP-SH-01 `[unknownCmd]` case — must not
/// fire S102 through the indirection.
#[test]
fn fp_sh_15_unknown_command_result_no_s102() {
    let src = "\
proc f {} {
    set x 0
    while {1} {
        set x [totallyUnknownCommand $x]
        set x [string range $x 0 end]
    }
}
";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S102"),
        "FP-SH-15: unknown-command result must not fire S102; got {got:?}"
    );
}

// ---------------------------------------------------------------------------
// FP-SH-16 — global/namespace aliasing protects a name even once it has a
// local prior version, matching the externally-mutable guard SCCP/O102
// already apply to their own lattices
// ---------------------------------------------------------------------------

/// FP-SH-16: `global x; set x 0` before the loop gives the header phi a
/// real, versioned `Known(Int)` entry type — but `x` stays externally
/// mutable throughout this function's body regardless: another procedure's
/// own `global x; set x …` (reached through a call whose relative order
/// isn't statically known) or a write trace's callback can rewrite it
/// between any two statements here, including the loop's own. Reusing
/// `crate::sccp::is_externally_mutable` — the same predicate
/// `sccp_with_extra_escaping` and O102 load-forwarding already apply to
/// their own (separate) lattices — for the type lattice closes this: no
/// literal-driven def of an aliased name is trusted, so S102 must not fire.
#[test]
fn fp_sh_16_global_alias_locally_initialised_no_s102() {
    let src = "\
proc f {} {
    global x
    set x 0
    while {1} {
        set x [expr {$x + 1}]
        set x [string range $x 0 end]
    }
}
";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S102"),
        "FP-SH-16: a locally-initialised global must not fire S102 — another \
         proc's `global x` write, or a trace, could rewrite it between any two \
         statements here; got {got:?}"
    );
}

/// FP-SH-16 TP control: an *unaliased* sibling local in the same function,
/// oscillating the same way, must still fire — proves the escaping-set
/// widening is keyed on the aliased name specifically, not a blanket
/// suppression of S102 for the whole function.
#[test]
fn fp_sh_16_unaliased_sibling_local_still_fires() {
    let src = "\
proc f {} {
    global x
    set x 0
    set y 0
    while {1} {
        set x [expr {$x + 1}]
        set x [string range $x 0 end]
        set y [expr {$y + 1}]
        set y [string range $y 0 end]
    }
}
";
    let got = codes(src, D);
    assert!(
        got.iter().any(|c| c == "S102"),
        "FP-SH-16 TP: an unaliased sibling local in the same function must still \
         fire S102; got {got:?}"
    );
}

/// FP-SH-16 TN control: a namespace variable declared with `variable x` but
/// never locally re-initialised before the loop keeps its unknown entry
/// type (SSA version 0 stays OVERDEFINED) — no S102.
#[test]
fn fp_sh_16_namespace_alias_without_local_init_no_s102() {
    let src = "\
namespace eval ::foo {
    variable x 0
    proc bump {} {
        variable x
        while {1} {
            set x [expr {$x + 1}]
            set x [string range $x 0 end]
        }
    }
}
";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S102"),
        "FP-SH-16: namespace alias without a local re-init must not fire S102; got {got:?}"
    );
}

// ---------------------------------------------------------------------------
// FP-SH-09 — a variable under a live write-trace is not confidently typed
// ---------------------------------------------------------------------------

/// FP-SH-09: `trace add variable v write cb` registers a callback that can
/// rewrite `v`'s value — and so its intrep — on every subsequent write. A
/// `set v hello` *after* the trace is installed must not be trusted as a
/// definite String: the trace callback runs on that very `set` and may
/// substitute a different value. Claiming S100 on the following `incr v`
/// would be unsound.
#[test]
fn fp_sh_09_traced_variable_write_not_confidently_typed() {
    let src = "\
proc f {} {
    trace add variable v write cb
    set v hello
    incr v
}
";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S100" || c == "S101"),
        "FP-SH-09: a write-traced variable's def must widen to OVERDEFINED; got {got:?}"
    );
}

/// FP-SH-09 TP control: the identical `set`/`incr` pair with no trace
/// anywhere in the function must still fire — the widening is trace-gated,
/// not a blanket suppression of `incr` shimmer.
#[test]
fn fp_sh_09_untraced_variable_still_fires() {
    let src = "proc f {} {\n    set v hello\n    incr v\n}\n";
    let got = codes(src, D);
    assert!(
        got.iter().any(|c| c == "S100" || c == "S101"),
        "FP-SH-09 TP: untraced incr shimmer must still fire; got {got:?}"
    );
}

/// FP-SH-09: the trace can be installed by a *different* proc — the
/// module-wide variable-trace fact must still widen the def, not just a
/// same-function `analyse_var_observability` scan.
#[test]
fn fp_sh_09_module_wide_trace_from_another_proc_suppresses_shimmer() {
    let src = "\
proc install_trace {} {
    trace add variable ::v write cb
}
proc use_it {} {
    install_trace
    set ::v hello
    incr ::v
}
";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S100" || c == "S101"),
        "FP-SH-09: a trace installed by a different proc must still widen the def; got {got:?}"
    );
}

// FP-SH-17 — destructuring writers do not broadcast their return type onto
// the variables they write (issue #867).  `lassign`/`scan`/`regexp`/`binary
// scan` write element-wise pieces, not the leftover-list / match-count they
// return, so a target must not inherit `List`/`Int`.  Driven by each command's
// registry `VarWriteTyping`, never a compiler-side def-count heuristic.

/// FP-SH-17: the reported case — `lassign $point x y z` then arithmetic on the
/// targets.  Pre-fix the targets were typed `List` (the command's return type),
/// so `expr {$x + $y + $z}` wrongly fired S100 "list intrep used in arithmetic".
#[test]
fn fp_sh_17_lassign_targets_arithmetic_no_shimmer() {
    let src = "set point [list 1 2 3]\n\
               lassign $point x y z\n\
               set offset [expr {$x + $y + $z}]";
    let got = codes(src, D);
    assert!(
        !got.iter()
            .any(|c| c == "S100" || c == "S101" || c == "S102"),
        "FP-SH-17: lassign targets are list elements (unknown intrep), not lists; \
         arithmetic on them must not fire shimmer; got {got:?}"
    );
}

/// FP-SH-17: the single-target `lassign $l x` case the old `defs.len() > 1`
/// heuristic never covered — one write still fell through to the `List` return
/// type.  Both arithmetic and string-comparison uses must stay silent.
#[test]
fn fp_sh_17_lassign_single_target_no_shimmer() {
    let arith = "set p [list 5]\nlassign $p x\nset o [expr {$x + 1}]";
    assert!(
        !codes(arith, D).iter().any(|c| c == "S100" || c == "S101"),
        "FP-SH-17: single-target lassign in arithmetic must not fire; got {:?}",
        codes(arith, D)
    );
    let strcmp = "set p [list a b]\nlassign $p x\nif {$x eq \"a\"} { puts yes }";
    assert!(
        !codes(strcmp, D).iter().any(|c| c == "S100" || c == "S101"),
        "FP-SH-17: single-target lassign in string compare must not fire; got {:?}",
        codes(strcmp, D)
    );
}

/// FP-SH-17: `regexp` capture variables hold matched *substrings*, not the
/// `Int` match count the command returns.  Pre-fix a single capture was typed
/// `Int`, so comparing it with `eq` wrongly fired S100 "numeric variable used
/// in string comparison".
#[test]
fn fp_sh_17_regexp_capture_string_compare_no_shimmer() {
    let src = "set s \"abc\"\n\
               regexp {(.)} $s letter\n\
               if {$letter eq \"a\"} { puts yes }";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S100" || c == "S101"),
        "FP-SH-17: regexp capture is a string, not the Int count; \
         string compare must not fire; got {got:?}"
    );
}

/// FP-SH-17: `scan`'s targets are format-dependent conversions, unknown
/// without parsing the format — not the `Int` count.  A `%s` target compared
/// as a string must not fire.
#[test]
fn fp_sh_17_scan_target_string_compare_no_shimmer() {
    let src = "set s \"hello\"\n\
               scan $s \"%s\" word\n\
               if {$word eq \"hello\"} { puts yes }";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S100" || c == "S101"),
        "FP-SH-17: scan target intrep is unknown, not Int; string compare must \
         not fire; got {got:?}"
    );
}

/// FP-SH-17: `regsub`'s `varName` form writes the substituted *string* while
/// returning the replacement count.  Comparing the output as a string must not
/// fire the numeric-in-string-compare shimmer.
#[test]
fn fp_sh_17_regsub_output_string_compare_no_shimmer() {
    let src = "set s \"aaa\"\n\
               regsub \"a\" $s \"b\" out\n\
               if {$out eq \"baa\"} { puts yes }";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S100" || c == "S101"),
        "FP-SH-17: regsub output is a string, not the Int count; got {got:?}"
    );
}

/// FP-SH-17 TP control: `regsub`'s output is a genuine `String` (`Fixed(String)`,
/// not `Destructured`), so using it in arithmetic is a real string→int shimmer
/// and must still fire S100 — the fix drops the bogus `Int` typing without
/// widening the value away (PR #885 review).
#[test]
fn fp_sh_17_regsub_output_in_arithmetic_still_fires() {
    let src = "set s \"aaa\"\n\
               regsub \"a\" $s \"b\" out\n\
               set n [expr {$out + 1}]";
    assert!(
        fires(src, D, "S100"),
        "FP-SH-17 TP: regsub output is a String; arithmetic on it must fire S100; got {:?}",
        codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-SH-18 — a numeric loop-body oscillation seeded by an Int entry was
// masked: `type_join`'s exact-equality Known-vs-Shimmered rule degraded
// `Known(Int) ⊔ SHIMMERED(Numeric, String)` to OVERDEFINED, so the genuine
// thunk went undetected. The numeric-refinement join keeps it SHIMMERED.
// ---------------------------------------------------------------------------

/// FP-SH-18 (detection gap now closed): a self-referential loop whose body
/// oscillates between `Numeric` (from `expr {$x + …}` on the loop-carried
/// `$x`) and `String`, seeded by an `Int` entry (`set x 0`). Pre-fix the
/// loop-header phi joined `Known(Int)` with the body's `SHIMMERED(Numeric,
/// String)` and — because `Int != Numeric` under exact equality — degraded to
/// OVERDEFINED, silently masking the thunk. `Int` is subsumed by the `Numeric`
/// side, so the join now stays SHIMMERED and S102 fires. Genuine thunk
/// (verified against tclsh: the loop pays a per-iteration numeric↔string
/// re-conversion).
#[test]
fn fp_sh_18_numeric_shimmer_masked_by_int_entry_fires_s102() {
    let src = "\
proc f {n} {
    set x 0
    while {$n > 0} {
        if {$n % 2} {
            set x [expr {$x + $n}]
        } else {
            set x \"s$x\"
        }
        incr n -1
    }
    return $x
}
";
    assert!(
        fires(src, D, "S102"),
        "FP-SH-18: numeric/string oscillation seeded by an Int entry must fire S102; got {:?}",
        codes(src, D)
    );
}

/// FP-SH-17: a call that writes several variables under the default typing
/// (`catch {body} resultVar optionsVar`) must not broadcast its `Int` status
/// return onto them — `result`/`opts` (and the body's `msg`) stay overdefined,
/// so comparing `opts` as a string draws no numeric-in-string-compare S100
/// (PR #885 review).
#[test]
fn fp_sh_17_catch_result_and_options_vars_no_shimmer() {
    let src = "catch {set msg hello} result opts\n\
               if {$opts eq \"\"} { puts $result }";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S100" || c == "S101"),
        "FP-SH-17: catch result/options vars are not the Int status; got {got:?}"
    );
}

/// FP-SH-17 TP control: the fix widens *destructured* targets, not every value
/// — a genuine `set s hello; expr {$s + 1}` STRING-in-arithmetic shimmer must
/// still fire, proving the change is not blanket-silencing.
#[test]
fn fp_sh_17_genuine_string_arithmetic_still_fires() {
    let src = "set s hello\nset y [expr {$s + 1}]";
    assert!(
        fires(src, D, "S100"),
        "FP-SH-17 TP: genuine STRING-in-arithmetic must still fire S100; got {:?}",
        codes(src, D)
    );
}

/// FP-SH-17 TP control: a `lassign` *leftover* captured with `set left
/// [lassign …]` still types `left` as the returned `List` — the return value
/// genuinely is a list, so using it where a scalar is expected is a real
/// signal.  Here the leftover flows to `llength` (a list use) — no shimmer,
/// confirming the return-value path is unchanged.
#[test]
fn fp_sh_17_lassign_leftover_capture_is_list() {
    let src = "set p [list 1 2 3]\n\
               set left [lassign $p a]\n\
               set n [llength $left]";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S100" || c == "S101"),
        "FP-SH-17: lassign leftover is a List used as a list — no shimmer; got {got:?}"
    );
}

/// FP-SH-18 TN control: a loop whose body is uniformly numeric (`Int` entry,
/// `expr`-only body) must NOT fire — the numeric refinement must not turn a
/// clean numeric loop into a spurious shimmer.
#[test]
fn fp_sh_18_uniform_numeric_loop_no_s102() {
    let src = "\
proc f {n} {
    set x 0
    while {$n > 0} {
        set x [expr {$x + $n}]
        incr n -1
    }
    return $x
}
";
    let got = codes(src, D);
    assert!(
        !got.iter().any(|c| c == "S102"),
        "FP-SH-18: a uniformly-numeric loop must not fire S102; got {got:?}"
    );
}

// ---------------------------------------------------------------------------
// FP-SH-19 — byte-array-transparent `string` ops on a payload getter
// ---------------------------------------------------------------------------

/// FP-SH-19: `string range`/`index`/`reverse`/`trim*` keep the byte-array
/// representation (verified against tclsh 8.6 and 9.0), so the canonical
/// `string range $payload …` → `*::payload replace` idiom is byte-exact and
/// must NOT fire S110. Registry-driven via `ByteArrayEffect::Transparent`.
#[test]
fn fp_sh_19_transparent_string_ops_on_payload_silent() {
    for op in [
        "string range $p 0 5",
        "string index $p 3",
        "string reverse $p",
        "string trim $p",
        "string trimleft $p",
        "string trimright $p",
    ] {
        let src = format!(
            "when CLIENT_DATA {{\n  set p [TCP::payload]\n  set q [{op}]\n  \
             TCP::payload replace 0 100 $q\n}}\n"
        );
        assert!(
            !fires(&src, "f5-irules", "S110"),
            "FP-SH-19: transparent op '{op}' must not fire S110; got {:?}",
            codes(&src, "f5-irules"),
        );
    }
}

/// FP-SH-19 TP control: the coercing `string map` form DOES corrupt the byte
/// array (it builds a character string), so it must still fire S110.
#[test]
fn fp_sh_19_coercing_string_map_on_payload_still_fires() {
    let src = "when CLIENT_DATA {\n  set p [TCP::payload]\n  set q [string map {a b} $p]\n  \
               TCP::payload replace 0 100 $q\n}\n";
    assert!(
        fires(src, "f5-irules", "S110"),
        "FP-SH-19 TP: string map on a payload must fire S110; got {:?}",
        codes(src, "f5-irules"),
    );
}
