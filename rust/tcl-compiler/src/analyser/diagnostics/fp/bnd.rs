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

//! BND family — bounds / intervals (W230/W231/W232/W233), driven by the
//! Phase-3 interval domain.

use super::D;
use crate::analyser::Analyser;
use crate::compilation_unit::CompilationUnit;
use crate::compiler_checks::run_all_checks;
use tcl_registry::model::ingress::static_context_for;

/// Full `(code, message)` diagnostics for `src`, mirroring `tcl diag` (analyser
/// plus `run_all_checks`, optimisation codes excluded). Bounds codes (W23x)
/// flow through these passes.
fn diags(src: &str, dialect: &str) -> Vec<(String, String)> {
    let registry = static_context_for(dialect).commands();
    let cu = CompilationUnit::build_for(src, registry, false);
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    let mut v: Vec<(String, String)> = Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|x| (x.code.to_string(), x.message.clone()))
        .collect();
    for x in run_all_checks(&cu, registry, d) {
        if x.code.is_optimisation() {
            continue;
        }
        v.push((x.code.to_string(), x.message.clone()));
    }
    v
}

fn fires(src: &str, code: &str) -> bool {
    diags(src, D).iter().any(|(c, _)| c == code)
}

fn fires_with_msg(src: &str, code: &str, needle: &str) -> bool {
    diags(src, D)
        .iter()
        .any(|(c, m)| c == code && m.contains(needle))
}

// FP-BND-01 — W231 dynamic loop-index past append-slot fires.

const FP_BND_01_REPRO: &str = "\
proc f {v} {
    # j is bounded [4, 8]; list length 3 -> every iteration is OOR.
    set l {a b c}
    for {set j 4} {$j < 9} {incr j} { lset l $j $v }
}
";

#[test]
fn fp_bnd_01_loop_index_past_append_slot_fires() {
    // TP: j ∈ [4, 8], length 3 -> every iteration out-of-range; W231 fires,
    // and the message names the dynamic index `$j`.
    assert!(
        fires_with_msg(FP_BND_01_REPRO, "W231", "$j"),
        "FP-BND-01 TP: loop index past append slot must fire W231 naming $j; emitted {:?}",
        diags(FP_BND_01_REPRO, D)
    );
}

// FP-BND-02 — W231 append-slot ($j == length) IS silent.

const FP_BND_02_REPRO: &str = "\
proc f {v} {
    # j == length -> APPEND slot; must NOT fire W231.
    set l {a b c}
    set j 3
    lset l $j $v
}
";

#[test]
fn fp_bnd_02_dynamic_append_slot_silent() {
    // FP: dynamic append slot (j == length) must NOT fire (uses `> length`).
    assert!(
        !fires(FP_BND_02_REPRO, "W231"),
        "FP-BND-02: dynamic append slot must NOT fire W231; emitted {:?}",
        diags(FP_BND_02_REPRO, D)
    );
}

#[test]
fn fp_bnd_02_in_range_dynamic_silent() {
    // FP control: a clearly-in-range dynamic index must NOT fire.
    let src = "proc f {v} { set l {a b c}\n set j 1\n lset l $j $v }";
    assert!(
        !fires(src, "W231"),
        "FP-BND-02: in-range dynamic index fired W231; {:?}",
        diags(src, D)
    );
}

// FP-BND-03 — W232 string index past end fires.

const FP_BND_03_REPRO: &str = "\
proc f {} {
    # i (10) > string length (5) -> tclsh returns \"\"; W232 smell.
    set s \"hello\"
    set i 10
    return [string index $s $i]
}
";

#[test]
fn fp_bnd_03_string_index_past_end_fires() {
    // TP: `string index $s 10` on a 5-char string is past-end (smell-tier W232).
    assert!(
        fires(FP_BND_03_REPRO, "W232"),
        "FP-BND-03 TP: past-end string index must fire W232; {:?}",
        diags(FP_BND_03_REPRO, D)
    );
}

#[test]
fn fp_bnd_03_idx_equals_length_fires() {
    // TP: idx == length is also out-of-range for `string index` (returns "").
    let src = "proc f {} { set s \"hello\"\n set i 5\n return [string index $s $i] }";
    assert!(
        fires(src, "W232"),
        "FP-BND-03 TP: idx==length must fire W232; {:?}",
        diags(src, D)
    );
}

#[test]
fn fp_bnd_03_in_range_silent() {
    // FP control: an in-range string index must NOT fire.
    let src = "proc f {} { set s \"hello\"\n set i 2\n return [string index $s $i] }";
    assert!(
        !fires(src, "W232"),
        "FP-BND-03: in-range string index fired W232; {:?}",
        diags(src, D)
    );
}

#[test]
fn fp_bnd_03_unknown_string_silent() {
    // FP control: unknown string + unknown index is not provable OOR.
    let src = "proc f {s i} { return [string index $s $i] }";
    assert!(
        !fires(src, "W232"),
        "FP-BND-03: unknown string/index fired W232; {:?}",
        diags(src, D)
    );
}

// FP-BND-04 — W233 division by provably-zero divisor fires.

const FP_BND_04_REPRO: &str = "\
proc f {} {
    # $d == 0 (CONST) -> tclsh divide-by-zero; W233 fires.
    set d 0
    return [expr {10 / $d}]
}
";

#[test]
fn fp_bnd_04_const_zero_divisor_fires() {
    assert!(
        fires(FP_BND_04_REPRO, "W233"),
        "FP-BND-04 TP: const-zero divisor must fire W233; {:?}",
        diags(FP_BND_04_REPRO, D)
    );
}

#[test]
fn fp_bnd_04_literal_div_zero_fires() {
    assert!(
        fires("proc f {} { return [expr {1 / 0}] }", "W233"),
        "FP-BND-04 TP: literal 1/0 must fire W233"
    );
}

#[test]
fn fp_bnd_04_literal_mod_zero_fires() {
    assert!(
        fires("proc f {} { return [expr {5 % 0}] }", "W233"),
        "FP-BND-04 TP: literal 5%0 must fire W233"
    );
}

#[test]
fn fp_bnd_04_nonzero_divisor_silent() {
    let src = "proc f {} { set d 3\n return [expr {10 / $d}] }";
    assert!(
        !fires(src, "W233"),
        "FP-BND-04: non-zero const divisor fired W233; {:?}",
        diags(src, D)
    );
}

#[test]
fn fp_bnd_04_unknown_divisor_silent() {
    let src = "proc f {n} { return [expr {10 / $n}] }";
    assert!(
        !fires(src, "W233"),
        "FP-BND-04: unknown divisor fired W233; {:?}",
        diags(src, D)
    );
}

// FP-BND-05 — W233 short-circuit / dead arm is silent.

#[test]
fn fp_bnd_05_dead_ternary_arm_silent() {
    // FP: `0 ? 1/0 : 7` -> dead arm; tclsh returns 7.
    let src = "proc f {} { return [expr {0 ? 1/0 : 7}] }";
    assert!(
        !fires(src, "W233"),
        "FP-BND-05: dead ternary arm fired W233; {:?}",
        diags(src, D)
    );
}

#[test]
fn fp_bnd_05_short_circuit_or_silent() {
    let src = "proc f {} { return [expr {1 || 1/0}] }";
    assert!(
        !fires(src, "W233"),
        "FP-BND-05: `||` short-circuit fired W233; {:?}",
        diags(src, D)
    );
}

#[test]
fn fp_bnd_05_short_circuit_and_silent() {
    let src = "proc f {} { return [expr {0 && 1/0}] }";
    assert!(
        !fires(src, "W233"),
        "FP-BND-05: `&&` short-circuit fired W233; {:?}",
        diags(src, D)
    );
}

#[test]
fn fp_bnd_05_guard_excludes_zero_silent() {
    // FP control: `if {$d != 0}` guard makes the division unreachable when d==0.
    let src = "proc f {} { set d 0\n if {$d != 0} { return [expr {1 / $d}] }\n return safe }";
    assert!(
        !fires(src, "W233"),
        "FP-BND-05: guarded division fired W233; {:?}",
        diags(src, D)
    );
}

// FP-BND-06 — W233 fires when a NON-INTEGER constant guard forces the lazy arm.

#[test]
fn fp_bnd_06_float_guard_forces_arm_fires() {
    assert!(
        fires("proc f {} { return [expr {1.0 && 1/0}] }", "W233"),
        "FP-BND-06 TP: 1.0 && 1/0 must fire W233"
    );
    assert!(
        fires("proc f {} { return [expr {0.0 || 1/0}] }", "W233"),
        "FP-BND-06 TP: 0.0 || 1/0 must fire W233"
    );
    assert!(
        fires("proc f {} { return [expr {1.5 ? 1/0 : 7}] }", "W233"),
        "FP-BND-06 TP: 1.5 ? 1/0 : 7 must fire W233"
    );
}

#[test]
fn fp_bnd_06_uppercase_bool_guard_forces_arm_fires() {
    assert!(
        fires("proc f {} { return [expr {True && 1/0}] }", "W233"),
        "FP-BND-06 TP: True && 1/0 must fire W233"
    );
    assert!(
        fires("proc f {} { return [expr {TRUE && 1/0}] }", "W233"),
        "FP-BND-06 TP: TRUE && 1/0 must fire W233"
    );
    assert!(
        fires("proc f {} { return [expr {No || 1/0}] }", "W233"),
        "FP-BND-06 TP: No || 1/0 must fire W233"
    );
}

#[test]
fn fp_bnd_06_unary_constant_guard_forces_arm_fires() {
    assert!(
        fires("proc f {} { return [expr {-1 && 1/0}] }", "W233"),
        "FP-BND-06 TP: -1 && 1/0 must fire W233"
    );
    assert!(
        fires("proc f {} { return [expr {!0 && 1/0}] }", "W233"),
        "FP-BND-06 TP: !0 && 1/0 must fire W233"
    );
}

#[test]
fn fp_bnd_06_false_constant_guard_short_circuits_silent() {
    assert!(
        !fires("proc f {} { return [expr {False && 1/0}] }", "W233"),
        "FP-BND-06: False && short-circuits"
    );
    assert!(
        !fires("proc f {} { return [expr {!1 && 1/0}] }", "W233"),
        "FP-BND-06: !1 && short-circuits"
    );
    assert!(
        !fires("proc f {} { return [expr {0.0 && 1/0}] }", "W233"),
        "FP-BND-06: 0.0 && short-circuits"
    );
}

#[test]
fn fp_bnd_06_nonconstant_guard_silent() {
    let src = "proc f {c} { return [expr {$c && 1/0}] }";
    assert!(
        !fires(src, "W233"),
        "FP-BND-06: non-constant guard fired W233; {:?}",
        diags(src, D)
    );
}
