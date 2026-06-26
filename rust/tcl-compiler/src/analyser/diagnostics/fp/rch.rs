//! RCH family — reachability (O107) + the read-before-set (W210) fallout of
//! handler bodies that used to be CFG islands.
//! Pairs to `tests/test_fp_rch.py` and the §RCH entries in `docs/design/compiler/FP.md`.

use super::{fires, D};
use crate::analyser::Analyser;
use crate::compilation_unit::CompilationUnit;
use crate::compiler_checks::run_all_checks;
use crate::optimiser::manager::optimise_with_dialect;
use tcl_core_types::DiagCode;
use tcl_registry::registry_for_dialect;

/// Full diagnostic codes for `src`, INCLUDING the optimiser's suggestions.
/// `O107` (unreachable dead code) is produced by the optimiser pass, not by the
/// analyser or `run_all_checks` — the Python `get_diagnostics` surface that the
/// RCH catalogue was authored against includes optimiser reachability
/// suggestions, so this helper unions all three sources.
fn all_codes(src: &str, dialect: &str) -> Vec<String> {
    let registry = registry_for_dialect(dialect);
    let cu = CompilationUnit::build_for(src, registry, false);
    let d = (!dialect.is_empty()).then_some(dialect);
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
    let d = (!dialect.is_empty()).then_some(dialect);
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
    let src = "proc f {} {\n    try {\n        risky\n    } on error {e} {\n        puts $e\n    }\n}";
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
