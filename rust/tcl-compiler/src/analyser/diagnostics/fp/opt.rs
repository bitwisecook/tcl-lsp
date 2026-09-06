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

//! OPT family — optimisation / codegen quick-fix false-positive catalogue.
//!
//! Covers codes O106, O109, O110, O114, O116, O120, O126 (and related analyser
//! codes W211/W220 where they overlap with the optimiser surface).

use crate::compilation_unit::CompilationUnit;
use crate::compiler_checks::run_all_checks;
use crate::optimiser::manager::{apply_optimisations, optimise_with_dialect};
use tcl_registry::model::ingress::static_context_for;

use super::D;

// Shared helpers

/// True if any optimisation with `code` fires on `src` under `dialect`.
fn opt_fires(src: &str, dialect: &str, code: &str) -> bool {
    let registry = static_context_for(dialect).commands();
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    optimise_with_dialect(src, registry, d)
        .iter()
        .any(|o| o.code.as_str() == code)
}

/// Apply all optimisations and return the rewritten source.
fn optimised(src: &str, dialect: &str) -> String {
    let registry = static_context_for(dialect).commands();
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    apply_optimisations(src, &optimise_with_dialect(src, registry, d))
}

/// Collect `(code, replacement)` pairs from the optimiser for `src`.
fn opt_rewrites(src: &str, dialect: &str) -> Vec<(String, String)> {
    let registry = static_context_for(dialect).commands();
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    optimise_with_dialect(src, registry, d)
        .into_iter()
        .map(|o| (o.code.as_str().to_owned(), o.replacement.clone()))
        .collect()
}

/// Diagnostic codes from the compiler-checks pass (the LICM / GVN / reachability
/// surface). O106/O107 are *diagnostics*, not rewrites, so they flow through
/// `run_all_checks` rather than the optimiser rewrite manager — this helper is
/// the right probe for those codes.
fn check_codes(src: &str, dialect: &str) -> Vec<String> {
    let registry = static_context_for(dialect).commands();
    let d = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    let cu = CompilationUnit::build_for(src, registry, false);
    run_all_checks(&cu, registry, d)
        .iter()
        .map(|d| d.code.to_string())
        .collect()
}

/// True if `code` appears in the compiler-checks diagnostic set for `src`.
fn check_fires(src: &str, dialect: &str, code: &str) -> bool {
    check_codes(src, dialect).iter().any(|c| c == code)
}

// FP-OPT-01 — O110 InstCombine: whitespace-only / paren-preservation / commutative reorder

const FP_OPT_01_REPRO_WHITESPACE: &str = "set x [expr { $a + $b }]\n";
const FP_OPT_01_REPRO_PAREN: &str = "set x [expr {($a << 1) & 0xff}]\n";
const FP_OPT_01_REPRO_REORDER: &str = "set x [expr {2 + $a}]\n";

#[test]
fn fp_opt_01_whitespace_only_no_o110() {
    // FP-OPT-01: decorative whitespace in expr must NOT fire O110.
    assert!(
        !opt_fires(FP_OPT_01_REPRO_WHITESPACE, D, "O110"),
        "FP-OPT-01: whitespace-only expr must not fire O110; rewrites={:?}",
        opt_rewrites(FP_OPT_01_REPRO_WHITESPACE, D)
    );
}

#[test]
fn fp_opt_01_branch_folding_whitespace_no_o110() {
    // FP-OPT-01: branch-folding whitespace churn must NOT fire O110.
    let src = "if {$x<0} { puts negative }\n";
    assert!(
        !opt_fires(src, D, "O110"),
        "FP-OPT-01: branch-fold whitespace must not fire O110; rewrites={:?}",
        opt_rewrites(src, D)
    );
}

#[test]
fn fp_opt_01_paren_preserved_no_o110() {
    // FP-OPT-01: mixed bitwise/shift keeps parens per CERT EXP00-C; must NOT fire O110.
    assert!(
        !opt_fires(FP_OPT_01_REPRO_PAREN, D, "O110"),
        "FP-OPT-01: paren-preserved bitwise expr must not fire O110; rewrites={:?}",
        opt_rewrites(FP_OPT_01_REPRO_PAREN, D)
    );
}

#[test]
fn fp_opt_01_commutative_reorder_no_o110() {
    // FP-OPT-01: commutative reorder without a real fold must NOT fire O110.
    assert!(
        !opt_fires(FP_OPT_01_REPRO_REORDER, D, "O110"),
        "FP-OPT-01: commutative-reorder-only must not fire O110; rewrites={:?}",
        opt_rewrites(FP_OPT_01_REPRO_REORDER, D)
    );
}

#[test]
fn fp_opt_01_genuine_simplification_still_fires() {
    // TP control: $i + 0 where $i is INT via loop counter SCCP must fire O110.
    let src = "proc f {n} {\n  for {set i 0} {$i < $n} {incr i} {\n    set y [expr {$i + 0}]\n    puts $y\n  }\n}\nf 3\n";
    assert!(
        opt_fires(src, D, "O110"),
        "FP-OPT-01 TP: identity simplification on INT loop counter must fire O110; rewrites={:?}",
        opt_rewrites(src, D)
    );
}

// FP-OPT-02 — O116 fold-const-list-command: empty [list] folds to {}, not ""

const FP_OPT_02_REPRO: &str = "set x [list]\nlappend x a\nputs $x\n";

#[test]
fn fp_opt_02_empty_list_quick_fix_uses_braces() {
    // FP-OPT-02: O116 on `[list]` must propose replacement "{}" (canonical empty-list literal).
    let registry = static_context_for(D).commands();
    let d = Some(tcl_registry::model::ingress::resolve_environment(D).analyser_profile());
    let opts = optimise_with_dialect(FP_OPT_02_REPRO, registry, d);
    let o116: Vec<_> = opts.iter().filter(|o| o.code.as_str() == "O116").collect();
    assert!(
        !o116.is_empty(),
        "FP-OPT-02: O116 must fire on `set x [list]`; rewrites={:?}",
        opt_rewrites(FP_OPT_02_REPRO, D)
    );
    for o in &o116 {
        assert_eq!(
            o.replacement, "{}",
            "FP-OPT-02: O116 replacement must be canonical empty-list literal `{{}}`; got {:?}",
            o.replacement
        );
    }
}

// FP-OPT-03 — O106 LICM purity: outer-pure / inner-impure expression NOT hoistable

const FP_OPT_03_REPRO: &str = "\
proc f {} {
    for {set i 0} {$i < 10} {incr i} {
        set s [format %04d [incr testnum]]
    }
    return $s
}
";

#[test]
fn fp_opt_03_inner_impure_blocks_licm() {
    // FP-OPT-03: LICM must NOT hoist [format %04d [incr testnum]] — inner impure.
    // O106 is a *diagnostic* (find_loop_invariants), so probe the compiler-checks
    // surface, not the rewrite manager.
    assert!(
        !check_fires(FP_OPT_03_REPRO, D, "O106"),
        "FP-OPT-03: inner-impure expression must block LICM O106; codes={:?}",
        check_codes(FP_OPT_03_REPRO, D)
    );
}

#[test]
fn fp_opt_03_outer_pure_inner_pure_abstains_in_a_procedure_body() {
    // This was a true-positive control while loop-invariance was decided by the
    // legacy command-string classifier, which treated `format` as pure because
    // of its name. It is not sound: hoisting a call out of a loop changes how
    // many times it is dispatched, and a caller can install an execution trace
    // on `format` before invoking this procedure, so the hoist would change the
    // number of trace callbacks.
    //
    // Loop-invariance now consults the same dispatch-stability proof as reuse
    // (`find_loop_invariants_for_function`). A procedure body carries the
    // `UnknownWorld` entry contract, so no proof succeeds inside one and the
    // pass abstains. Restoring this finding needs an entry contract for
    // procedure bodies derived from workspace, package, and sourced-file facts
    // -- the contract issue #1364 requires -- not a return to name-based purity.
    let src = "\
proc f {} {
    set k 42
    for {set i 0} {$i < 10} {incr i} {
        set s [format %04d [expr {$k + 1}]]
    }
    return $s
}
";
    assert!(
        !check_fires(src, D, "O106"),
        "a procedure body cannot prove dispatch stability, so O106 must abstain; codes={:?}",
        check_codes(src, D)
    );
}

// FP-OPT-04 — O109/O126 dead-store / unused via call-by-name suppression

const FP_OPT_04_REPRO: &str = "\
proc asnPeekTag {data {tag tag} {type type}} {
    upvar 1 $tag tagOut $type typeOut
    set tagOut 0
    set typeOut 0
    return [string length $data]
}
proc decode {data} {
    asnPeekTag $data tag type
    return [list $tag $type]
}
";

#[test]
fn fp_opt_04_call_by_name_suppresses_dead_store() {
    // FP-OPT-04: call-by-name upvar idiom must suppress O109/O126 on tag/type in decode.
    // Check optimiser codes (O109, O126) — the analyser W211/W220 are not in opt surface.
    let registry = static_context_for(D).commands();
    let d = Some(tcl_registry::model::ingress::resolve_environment(D).analyser_profile());
    let opts = optimise_with_dialect(FP_OPT_04_REPRO, registry, d);
    let relevant: Vec<_> = opts
        .iter()
        .filter(|o| {
            let code = o.code.as_str();
            (code == "O109" || code == "O126")
                && (o.message.contains("'tag'") || o.message.contains("'type'"))
        })
        .collect();
    assert!(
        relevant.is_empty(),
        "FP-OPT-04: call-by-name suppression should silence O109/O126 on tag/type; got {relevant:?}"
    );
}

#[test]
fn fp_opt_04_genuine_dead_store_still_fires() {
    // TP control: a write with no callee using-by-name still fires O109 or O126.
    let src = "\
proc f {} {
    set x 1
    set x 2
    return $x
}
";
    let registry = static_context_for(D).commands();
    let d = Some(tcl_registry::model::ingress::resolve_environment(D).analyser_profile());
    let opts = optimise_with_dialect(src, registry, d);
    let fires = opts
        .iter()
        .any(|o| matches!(o.code.as_str(), "O109" | "O126"));
    assert!(
        fires,
        "FP-OPT-04 TP: genuine dead store must still fire O109/O126; rewrites={:?}",
        opt_rewrites(src, D)
    );
}

// FP-OPT-05 — O126 must NOT delete an RHS with observable side effects (D2-O126)

const FP_OPT_05_REPRO: &str = "proc f {} { set unused [puts side]; puts done }";

#[test]
fn fp_opt_05_o126_preserves_puts_side_effect() {
    // FP-OPT-05: O126 must NOT fire on `set unused [puts side]` — puts is impure.
    assert!(
        !opt_fires(FP_OPT_05_REPRO, D, "O126"),
        "FP-OPT-05: O126 must NOT delete a [puts X] RHS (would lose side effect); rewrites={:?}",
        opt_rewrites(FP_OPT_05_REPRO, D)
    );
}

#[test]
fn fp_opt_05_o126_pure_rhs_still_fires() {
    // TP control: pure RHS [list 1 2 3] IS safe to delete; O126 must fire.
    let src = "proc f {} { set unused [list 1 2 3]; puts done }";
    assert!(
        opt_fires(src, D, "O126"),
        "FP-OPT-05 TP: O126 must fire when RHS is pure (list); rewrites={:?}",
        opt_rewrites(src, D)
    );
}

// FP-OPT-06 — O100/O109/O127: cmd-sub writes are SSA kills (D2-O100)

const FP_OPT_06_REPRO: &str = "proc f {} { set x a; set y [append x b]; puts $x; puts $y }";

#[test]
fn fp_opt_06_o100_does_not_propagate_past_cmd_sub_write() {
    // FP-OPT-06: [append x b] mutates x; optimiser must NOT propagate stale "a" into puts $x.
    let opt_src = optimised(FP_OPT_06_REPRO, D);
    assert!(
        !opt_src.contains("puts a"),
        "FP-OPT-06: O100 must NOT propagate stale value past [append x b]; got: {:?}",
        opt_src.trim()
    );
}

// FP-OPT-07 — O126 extends to pure user-proc RHS via interproc purity (D2-O126-FU)

const FP_OPT_07_REPRO: &str =
    "proc add {a b} { expr {$a + $b} }\nproc f {} { set unused [add 1 2]; puts done }";

#[test]
fn fp_opt_07_pure_user_proc_rhs_is_deleted() {
    // TP: pure user-proc RHS in unused assignment must fire O126.
    assert!(
        opt_fires(FP_OPT_07_REPRO, D, "O126"),
        "FP-OPT-07 TP: pure user-proc RHS must allow O126 deletion; rewrites={:?}",
        opt_rewrites(FP_OPT_07_REPRO, D)
    );
}

#[test]
fn fp_opt_07_impure_user_proc_rhs_preserved() {
    // TN control: impure user proc RHS must NOT be deleted by O126.
    let src =
        "proc shout {x} { puts $x; expr {$x + 1} }\nproc f {} { set unused [shout 1]; puts done }";
    assert!(
        !opt_fires(src, D, "O126"),
        "FP-OPT-07 TN: impure user-proc RHS must NOT be deleted by O126; rewrites={:?}",
        opt_rewrites(src, D)
    );
}

// FP-OPT-08 — O109/O126 overlap filter: segment_commands + EXPR/BODY descent (D4-F10)

const FP_OPT_08_REPRO: &str = "\
proc f {} {
    set a 1
    set b 0
    if {$a} {
        if {$b} { puts X }
    }
}
";

#[test]
fn fp_opt_08_nested_if_constant_chain_does_not_delete_inner_var() {
    // FP-OPT-08: overlap filter must NOT delete `set b 0` when $b survives in EXPR context.
    let opt_src = optimised(FP_OPT_08_REPRO, D);
    // If $b is still referenced in optimised source, set b 0 must survive.
    if opt_src.contains("$b") {
        assert!(
            opt_src.contains("set b 0") || opt_src.contains("set b {0}"),
            "FP-OPT-08: O109/O126 must not delete 'set b 0' when $b survives; got: {opt_src:?}"
        );
    }
}

#[test]
fn fp_opt_08_unrelated_set_still_eligible_for_o126() {
    // TN control: unreferenced 'set unused 42' must still be eligible for O126.
    let src = "proc f {} { set unused 42; puts done }";
    assert!(
        opt_fires(src, D, "O126"),
        "FP-OPT-08 TN: unreferenced 'set unused 42' must still fire O126; rewrites={:?}",
        opt_rewrites(src, D)
    );
}

// FP-OPT-09 — D5-O110: O110 identity/annihilator rewrites must preserve coercion semantics

const FP_OPT_09_TP_REPRO: &str = "proc f {x} {\n  puts [expr {$x + 0}]\n}\nf abc\n";
const FP_OPT_09_TN_REPRO: &str = "proc f {} {\n  for {set i 0} {$i < 3} {incr i} {\n    set y [expr {$i + 0}]\n    puts $y\n  }\n}\nf\n";

#[test]
fn fp_opt_09_unknown_type_param_blocks_identity_rewrite() {
    // FP-OPT-09: $x + 0 on unknown-type param must NOT be rewritten to $x (hides coercion error).
    // Both the code check and the replacement check.
    let registry = static_context_for(D).commands();
    let d = Some(tcl_registry::model::ingress::resolve_environment(D).analyser_profile());
    let opts = optimise_with_dialect(FP_OPT_09_TP_REPRO, registry, d);
    // No O110 at all is fine; but if O110 fires it must not drop "+ 0".
    let unsound_o110: Vec<_> = opts
        .iter()
        .filter(|o| {
            o.code.as_str() == "O110"
                && !o.replacement.contains("+ 0")
                && o.replacement.contains("$x")
        })
        .collect();
    assert!(
        unsound_o110.is_empty(),
        "FP-OPT-09: unsound O110 drop of `$x + 0` on unknown-type param; got {unsound_o110:?}"
    );
    // The diagnostic surface should also be quiet on O110.
    assert!(
        !opt_fires(FP_OPT_09_TP_REPRO, D, "O110"),
        "FP-OPT-09: O110 must NOT fire on unknown-type param $x + 0; rewrites={:?}",
        opt_rewrites(FP_OPT_09_TP_REPRO, D)
    );
}

#[test]
fn fp_opt_09_provably_numeric_var_still_fires() {
    // TN: $i + 0 where $i is INT via SCCP (loop counter) IS sound; O110 must fire.
    assert!(
        opt_fires(FP_OPT_09_TN_REPRO, D, "O110"),
        "FP-OPT-09 TN: provably-numeric identity simplification must fire O110; rewrites={:?}",
        opt_rewrites(FP_OPT_09_TN_REPRO, D)
    );
}

// FP-OPT-10 — D5-O114: set x [expr {$x + N}] -> incr x N requires proof x is INT

const FP_OPT_10_TP_REPRO: &str = "proc foo {x} {\n  set x [expr {$x + 1}]\n  puts $x\n}\nfoo 1.5\n";
const FP_OPT_10_TN_REPRO: &str = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {$x + 1}]\n    puts $x\n  }\n}\nfoo 3\n";

#[test]
fn fp_opt_10_unknown_type_param_blocks_incr_rewrite() {
    // FP-OPT-10: set x [expr {$x + 1}] on unknown-type param must NOT rewrite to incr x.
    assert!(
        !opt_fires(FP_OPT_10_TP_REPRO, D, "O114"),
        "FP-OPT-10: unsound O114 rewrite on unknown-type param; rewrites={:?}",
        opt_rewrites(FP_OPT_10_TP_REPRO, D)
    );
}

#[test]
fn fp_opt_10_provably_int_var_still_fires() {
    // TN: set x [expr {$x + 1}] inside a for-loop where x is SCCP-typed INT must fire O114.
    assert!(
        opt_fires(FP_OPT_10_TN_REPRO, D, "O114"),
        "FP-OPT-10 TN: provably-INT incr-idiom rewrite must fire O114; rewrites={:?}",
        opt_rewrites(FP_OPT_10_TN_REPRO, D)
    );
}

// FP-OPT-11 — O120 ==/!= -> eq/ne requires at-least-one provably-non-numeric operand (D5-O120)

const FP_OPT_11_TP_REPRO: &str = "proc f {raw} {\n    set a [string trim $raw]\n    if {$a == \"1\"} { puts yes } else { puts no }\n}\n";
const FP_OPT_11_TN_REPRO: &str = "proc f {raw} {\n    set a [string trim $raw]\n    if {$a == \"hello\"} { puts yes } else { puts no }\n}\n";

#[test]
fn fp_opt_11_numeric_like_literal_string_typed_var_no_rewrite() {
    // FP-OPT-11: STRING-typed $a + numeric-looking literal "1" must NOT rewrite to eq.
    assert!(
        !opt_fires(FP_OPT_11_TP_REPRO, D, "O120"),
        "FP-OPT-11: unsound O120 rewrite when neither operand provably non-numeric; rewrites={:?}",
        opt_rewrites(FP_OPT_11_TP_REPRO, D)
    );
}

#[test]
fn fp_opt_11_non_numeric_literal_still_rewrites() {
    // TN: $a == "hello" — "hello" is provably non-numeric; rewrite to eq is sound.
    assert!(
        opt_fires(FP_OPT_11_TN_REPRO, D, "O120"),
        "FP-OPT-11 TN: sound O120 rewrite (non-numeric literal) must fire; rewrites={:?}",
        opt_rewrites(FP_OPT_11_TN_REPRO, D)
    );
    let opt_src = optimised(FP_OPT_11_TN_REPRO, D);
    assert!(
        opt_src.contains("$a eq \"hello\""),
        "FP-OPT-11 TN: expected eq-form in optimised source; got: {opt_src:?}"
    );
}

// FP-OPT-12 — TclOO method purity wired into O126 (SF-2 PARTIAL)

#[test]
fn fp_opt_12_pure_user_proc_via_my_dispatch_handled_at_word_level() {
    // SF-2 wiring TP: pure user-proc set unused [pure_helper] must fire O126.
    let src = "proc pure_helper {} { return 42 }\nproc m {} {\n    set unused [pure_helper]\n    puts done\n}\n";
    assert!(
        opt_fires(src, D, "O126"),
        "FP-OPT-12: pure-user-proc RHS in unused assign must fire O126; rewrites={:?}",
        opt_rewrites(src, D)
    );
}

#[test]
fn fp_opt_12_impure_user_proc_still_blocks() {
    // TN control: impure user proc RHS in unused assignment must NOT fire O126.
    let src = "proc impure_helper {} { puts side; return 0 }\nproc m {} {\n    set unused [impure_helper]\n    puts done\n}\n";
    assert!(
        !opt_fires(src, D, "O126"),
        "FP-OPT-12 TN: impure-user-proc RHS must NOT fire O126; rewrites={:?}",
        opt_rewrites(src, D)
    );
}

#[test]
fn fp_opt_12_runtime_selected_tcloo_method_rhs_not_deleted() {
    // Tcl 9.0.4 oracle: `my` is a relative command head and an object's
    // namespace may shadow it, so the optimiser cannot prove this dispatch.
    let src = "oo::class create C {\n    method pure_helper {} { ::return 42 }\n    method m {} {\n        ::set unused [my pure_helper]\n        ::puts done\n    }\n}\n";
    assert!(
        !opt_fires(src, D, "O126"),
        "FP-OPT-12: relative `my` dispatch must make O126 abstain; rewrites={:?}",
        opt_rewrites(src, D)
    );
}

#[test]
fn fp_opt_12_tcloo_impure_method_rhs_preserved() {
    // SF-2 TN: impure TclOO method RHS must NOT fire O126.
    let src = "oo::class create C {\n    method impure_helper {} { puts hi; return 0 }\n    method m {} {\n        set unused [my impure_helper]\n        puts done\n    }\n}\n";
    assert!(
        !opt_fires(src, D, "O126"),
        "FP-OPT-12 TN: impure-method RHS must NOT fire O126; rewrites={:?}",
        opt_rewrites(src, D)
    );
}

#[test]
fn fp_opt_12_class_level_instance_var_write_not_deleted() {
    // SF-2 soundness: method that writes class-level variable is not pure.
    let src = "oo::class create C {\n    variable counter\n    method bump {} { set counter [expr {$counter + 1}]; return $counter }\n    method m {} { set unused [my bump]; puts done }\n}\n";
    assert!(
        !opt_fires(src, D, "O126"),
        "FP-OPT-12: instance-var-mutating method must not be treated as pure; rewrites={:?}",
        opt_rewrites(src, D)
    );
}

#[test]
fn fp_opt_12_method_local_instance_var_write_not_deleted() {
    // SF-2 soundness: method-local variable counter write makes method impure.
    let src = "oo::class create C {\n    method bump {} { variable counter; set counter 5; return $counter }\n    method m {} { set unused [my bump]; puts done }\n}\n";
    assert!(
        !opt_fires(src, D, "O126"),
        "FP-OPT-12: method-local instance-var write must not be treated as pure; rewrites={:?}",
        opt_rewrites(src, D)
    );
}

#[test]
fn fp_opt_12_array_element_instance_var_write_not_deleted() {
    // SF-2 soundness: array-element write of instance var makes method impure.
    let src = "oo::class create C {\n    variable counter\n    method bump {} { set counter(0) 1 }\n    method m {} { set unused [my bump]; puts done }\n}\n";
    assert!(
        !opt_fires(src, D, "O126"),
        "FP-OPT-12: array-element instance-var write must not be treated as pure; rewrites={:?}",
        opt_rewrites(src, D)
    );
}

#[test]
fn fp_opt_12_runtime_selected_instance_var_reader_not_deleted() {
    // The callee body is pure, but Tcl 9.0.4 resolves the relative `my` head
    // at runtime, so that is insufficient proof for O126.
    let src = "oo::class create C {\n    variable counter\n    method get {} { ::return $counter }\n    method m {} { ::set unused [my get]; ::puts done }\n}\n";
    assert!(
        !opt_fires(src, D, "O126"),
        "FP-OPT-12: relative `my` dispatch must make O126 abstain; rewrites={:?}",
        opt_rewrites(src, D)
    );
}

#[test]
fn fp_opt_12_redefined_method_via_oo_define_not_pure() {
    // SF-2 soundness: method redefined by oo::define (pure -> impure) must not fire O126.
    let src = "oo::class create C {\n    method helper {} { return 42 }\n    method m {} { set unused [my helper]; puts done }\n}\noo::define C { method helper {} { puts side; return 42 } }\n";
    assert!(
        !opt_fires(src, D, "O126"),
        "FP-OPT-12: redefined method must not be treated as pure; rewrites={:?}",
        opt_rewrites(src, D)
    );
}

#[test]
fn fp_opt_12_duplicate_in_body_method_not_pure() {
    // SF-2 soundness: method listed twice in same class body is conservatively impure.
    let src = "oo::class create C {\n    method helper {} { return 42 }\n    method helper {} { puts side; return 42 }\n    method m {} { set unused [my helper]; puts done }\n}\n";
    assert!(
        !opt_fires(src, D, "O126"),
        "FP-OPT-12: duplicate method definition must not be treated as pure; rewrites={:?}",
        opt_rewrites(src, D)
    );
}

// bridge-only: nested proc in method body not lifted.
// An IR-structure assertion on `ir_module.procedures` / `m.methods` with no
// diagnostic / rewrite analogue, so it is not reproduced here.

// bridge-only: test_FP_OPT_12_methods_survive_incremental_chunk_cache
// Tests `lower_commands_to_ir` + `lower_to_ir(src, chunk_ir=cache, chunks=chunks)`
// with green_tree_scope() and segmenter — incremental-chunk-cache plumbing that
// has no counterpart in the Rust optimiser surface.

#[test]
fn fp_opt_12_runtime_selected_oo_define_method_rhs_not_deleted() {
    // An `oo::define` method has the same Tcl 9.0.4 relative-head rule: bare
    // `my` is runtime-selected and cannot justify deleting its call.
    let src = "oo::class create C {}\noo::define C {\n    method pure_helper {} { ::return 42 }\n    method m {} {\n        ::set unused [my pure_helper]\n        ::puts done\n    }\n}\n";
    assert!(
        !opt_fires(src, D, "O126"),
        "FP-OPT-12: oo::define relative `my` dispatch must make O126 abstain; rewrites={:?}",
        opt_rewrites(src, D)
    );
}
