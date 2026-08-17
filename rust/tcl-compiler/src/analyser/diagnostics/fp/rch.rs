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

//! RCH family — reachability (O107) + the read-before-set (W210) fallout of
//! handler bodies that used to be CFG islands.

use super::{D, fires};
use crate::analyser::Analyser;
use crate::compilation_unit::CompilationUnit;
use crate::compiler_checks::run_all_checks;
use crate::optimiser::manager::optimise_with_dialect;
use tcl_core_types::DiagCode;
use tcl_registry::registry_for_dialect;

/// Full diagnostic codes for `src`, INCLUDING the optimiser's suggestions.
/// `O107` (unreachable dead code) is produced by the optimiser pass, not by the
/// analyser or `run_all_checks`; the RCH catalogue includes optimiser
/// reachability suggestions, so this helper unions all three sources.
fn all_codes(src: &str, dialect: &str) -> Vec<String> {
    let registry = registry_for_dialect(dialect);
    let cu = CompilationUnit::build_for(src, registry, false);
    let d = (!dialect.is_empty()).then(|| tcl_dialect::DialectProfile::by_name(dialect));
    let mut v: Vec<String> = Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|x| x.code.to_string())
        .collect();
    v.extend(
        run_all_checks(&cu, registry, d)
            .into_iter()
            .map(|x| x.code.to_string()),
    );
    v.extend(
        optimise_with_dialect(src, registry, d)
            .into_iter()
            .map(|o| o.code.to_string()),
    );
    v
}

fn o107_fires(src: &str, dialect: &str) -> bool {
    let registry = registry_for_dialect(dialect);
    let d = (!dialect.is_empty()).then(|| tcl_dialect::DialectProfile::by_name(dialect));
    optimise_with_dialect(src, registry, d)
        .iter()
        .any(|o| o.code == DiagCode::O107)
}

// ---------------------------------------------------------------------------
// FP-RCH-01 — `while 1 { ... break }`: the post-loop block is reachable.
// ---------------------------------------------------------------------------

const FP_RCH_01_REPRO: &str = "\
proc f {c} {
    # while 1 with a conditional `break` -> `puts after` IS reachable.
    while 1 { if {$c} break }
    puts after
}
";

#[test]
fn fp_rch_01_while1_break_after_reachable() {
    // FP-RCH-01: a conditional break inside `while 1` makes the post-loop block
    // reachable — `puts after` must NOT fire O107.
    assert!(
        !o107_fires(FP_RCH_01_REPRO, D),
        "FP-RCH-01: post-loop block is reachable; must not fire O107; emitted {:?}",
        all_codes(FP_RCH_01_REPRO, D)
    );
}

#[test]
fn fp_rch_01_for_true_break_reachable() {
    // FP-RCH-01: same fix applies to `for {…} true {…}` constant-true forms.
    let src = "proc f {c} { for {set i 0} true {incr i} { if {$c} break }\n puts after }";
    assert!(
        !o107_fires(src, D),
        "FP-RCH-01: for-true with break is reachable; emitted {:?}",
        all_codes(src, D)
    );
}

#[test]
fn fp_rch_01_nested_loop_break_reachable() {
    // FP-RCH-01: nested loops with break each feed their own loop-exit edge.
    let src =
        "proc f {c} { while 1 { foreach x {1 2} { if {$c} break }\n if {$c} break }\n puts after }";
    assert!(
        !o107_fires(src, D),
        "FP-RCH-01: nested-loop post block reachable; emitted {:?}",
        all_codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-RCH-02 — try handler body is reachable.
// ---------------------------------------------------------------------------

const FP_RCH_02_REPRO: &str = "\
proc f {} {
    # `on error` handler body is reachable; no O107 on `set y 1`.
    try {
        set x [doThing]
    } on error {e opts} {
        set y 1
        puts $y
    }
}
";

#[test]
fn fp_rch_02_handler_body_reachable() {
    // FP-RCH-02: try handler body must not be flagged unreachable.
    assert!(
        !o107_fires(FP_RCH_02_REPRO, D),
        "FP-RCH-02: handler body is reachable; emitted {:?}",
        all_codes(FP_RCH_02_REPRO, D)
    );
}

#[test]
fn fp_rch_02_handler_var_not_unset() {
    // FP-RCH-02 control: the handler-bound var `e` is defined by the handler
    // clause itself — must NOT be W210 read-before-set.
    let src =
        "proc f {} {\n    try {\n        risky\n    } on error {e} {\n        puts $e\n    }\n}";
    assert!(
        !fires(src, D, "W210"),
        "FP-RCH-02: handler-bound `e` is defined by the clause; emitted {:?}",
        all_codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-RCH-03 — `on ok` inherits body-defined SSA versions.
// ---------------------------------------------------------------------------

const FP_RCH_03_REPRO: &str = "\
proc f {} {
    # `on ok` runs after the body completes; $vdata IS defined.
    try {
        set vdata [getData]
    } on ok {} {
        return $vdata
    }
}
";

#[test]
fn fp_rch_03_on_ok_reads_body_var() {
    // FP-RCH-03: `on ok` runs after the body completes normally, so the body's
    // SSA versions feed the handler — `$vdata` is defined.
    assert!(
        !fires(FP_RCH_03_REPRO, D, "W210"),
        "FP-RCH-03: on-ok reads body-defined var; emitted {:?}",
        all_codes(FP_RCH_03_REPRO, D)
    );
}

#[test]
fn fp_rch_03_on_ok_unset_var_still_fires() {
    // FP-RCH-03 TP control: a `$vneversetbeforetry` read in `on ok` IS
    // read-before-set — the SSA-inheritance fix must not blanket-suppress it.
    let src = "\
proc f {} {
    try {
        set vdata [getData]
    } on ok {} {
        return $vneversetbeforetry
    }
}
";
    assert!(
        fires(src, D, "W210"),
        "FP-RCH-03 TP: genuine read-before-set in handler must still fire W210; emitted {:?}",
        all_codes(src, D)
    );
}

// ---------------------------------------------------------------------------
// FP-RCH-04 — genuine infinite loop (no break) → O107 IS reported.
// ---------------------------------------------------------------------------

const FP_RCH_04_REPRO: &str = "\
proc f {} {
    # No break / return -> `puts after` IS dead code.
    while 1 { puts x }
    puts after
}
";

#[test]
fn fp_rch_04_infinite_loop_dead_code_fires() {
    // FP-RCH-04 TP: a `while 1` body with no break/return/throw really does make
    // the post-loop block unreachable — O107 must still fire.
    assert!(
        o107_fires(FP_RCH_04_REPRO, D),
        "FP-RCH-04 TP: infinite loop with no break must fire O107; emitted {:?}",
        all_codes(FP_RCH_04_REPRO, D)
    );
}

// ---------------------------------------------------------------------------
// FP-RCH-05 — a dynamic-name write / destroy weakens the existence fold
// (issue #923 audit idx 1)
// ---------------------------------------------------------------------------
//
// `set $switch {}` defines whatever variable `$switch` names, so
// `[info exists mixed]` cannot be folded to a constant `false` and neither
// I230 (analyser) nor O101 (optimiser fold / DCE) may claim the guarded arm
// is unreachable.  `unset $n` is the mirror image: it can remove a parameter,
// so even the "a parameter always exists" fold has to abstain.
//
// Oracle (tclsh 9.0.4 and 8.6.14, identical):
//   proc f {} { set x foo; set $x bar; if {[info exists foo]} { return yes }
//               return no }
//   f                                                       → yes
//   proc f {p n} { unset $n; if {[info exists p]} { return yes }; return no }
//   f hello p                                               → no
//   f hello n                                               → yes

const FP_RCH_05_REPRO: &str = "\
proc f {} {
    set x foo
    # Defines the variable *named by* $x — i.e. `foo`.
    set $x bar
    if {[info exists foo]} { return yes }
    return no
}
";

#[test]
fn fp_rch_05_dynamic_write_blocks_the_absent_fold() {
    assert!(
        !fires(FP_RCH_05_REPRO, D, "I230"),
        "FP-RCH-05: `set $x bar` may define `foo`; emitted {:?}",
        all_codes(FP_RCH_05_REPRO, D)
    );
    assert!(
        !o107_fires(FP_RCH_05_REPRO, D),
        "FP-RCH-05: the reachable arm must not be optimised away; emitted {:?}",
        all_codes(FP_RCH_05_REPRO, D)
    );
}

#[test]
fn fp_rch_05_static_write_still_folds() {
    // TN control: with no dynamic write, a never-defined local really is
    // absent and the fold must survive.
    let src = "\
proc f {} {
    set x foo
    if {[info exists foo]} { return yes }
    return no
}
";
    assert!(
        fires(src, D, "I230"),
        "FP-RCH-05 TN: a never-defined local still folds to absent; emitted {:?}",
        all_codes(src, D)
    );
}

#[test]
fn fp_rch_05_dynamic_unset_blocks_the_present_fold() {
    let src = "\
proc f {p n} {
    unset $n
    if {[info exists p]} { return $p }
    return no
}
";
    assert!(
        !fires(src, D, "I230"),
        "FP-RCH-05: `unset $n` may remove the parameter; emitted {:?}",
        all_codes(src, D)
    );
}

#[test]
fn fp_rch_05_parameter_without_dynamic_unset_still_folds() {
    // TN control: a parameter with no dynamic destroy anywhere always exists.
    let src = "proc f {p} { if {[info exists p]} { return $p }\n    return no\n}\n";
    assert!(
        fires(src, D, "I230"),
        "FP-RCH-05 TN: a parameter still folds to present; emitted {:?}",
        all_codes(src, D)
    );
}

#[test]
fn fp_rch_05_dynamic_array_element_key_still_folds() {
    // TN control: `set a($k) 1` names a run-time *element* of the statically
    // named array `a` — it cannot conjure the local `foo`, so the fold stands.
    let src = "\
proc f {k} {
    set a($k) 1
    if {[info exists foo]} { return yes }
    return no
}
";
    assert!(
        fires(src, D, "I230"),
        "FP-RCH-05 TN: a dynamic element key is not a dynamic name; emitted {:?}",
        all_codes(src, D)
    );
}
