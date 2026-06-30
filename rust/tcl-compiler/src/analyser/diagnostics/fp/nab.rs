//! NAB family — not-a-bug / confirm-correct audits. Mostly TP confirmations
//! that a real hazard still fires, plus a few FP guards.
//! Pairs to `tests/test_fp_nab.py` and the §NAB entries in `docs/design/compiler/FP.md`.
//!
//! Two Python NAB entries test internal APIs rather than diagnostics and are
//! covered as Rust-structure tests elsewhere (not here):
//! FP-NAB-03 (interproc `pure` summary) lives in
//! `interprocedural.rs::tests::fp_nab_03_*`; FP-NAB-12 (`is_pure_var_ref`
//! value-shape parser) lives in
//! `value_shapes.rs::tests::fp_nab_12_escaped_paren_array_index_companions`.

use crate::analyser::Analyser;
use crate::compilation_unit::CompilationUnit;
use crate::compiler_checks::run_all_checks;
use crate::optimiser::manager::optimise_with_dialect;
use tcl_registry::registry_for_dialect;

const D: &str = "tcl8.6";

/// Full `(code, message)` pipeline output INCLUDING optimiser suggestions
/// (NAB-04 accepts W110 *or* its O120 optimiser near-duplicate), mirroring the
/// Python `get_diagnostics` surface.
fn diags(src: &str, dialect: &str) -> Vec<(String, String)> {
    let registry = registry_for_dialect(dialect);
    let cu = CompilationUnit::build_for(src, registry, false);
    let d = (!dialect.is_empty()).then_some(dialect);
    let mut v: Vec<(String, String)> = Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|x| (x.code.to_string(), x.message.clone()))
        .collect();
    for x in run_all_checks(&cu, registry, d) {
        v.push((x.code.to_string(), x.message.clone()));
    }
    for o in optimise_with_dialect(src, registry, d) {
        v.push((o.code.to_string(), o.message.clone()));
    }
    v
}

fn fires(src: &str, dialect: &str, code: &str) -> bool {
    diags(src, dialect).iter().any(|(c, _)| c == code)
}
fn fires_any(src: &str, dialect: &str, wanted: &[&str]) -> bool {
    diags(src, dialect)
        .iter()
        .any(|(c, _)| wanted.contains(&c.as_str()))
}
fn fires_with_msg(src: &str, dialect: &str, code: &str, needle: &str) -> bool {
    diags(src, dialect)
        .iter()
        .any(|(c, m)| c == code && m.to_lowercase().contains(needle))
}

// FP-NAB-01 — lset append-slot (index == length) is legal, NOT W231.
const FP_NAB_01_REPRO: &str = "\
# tclsh contract: lset at index == length APPENDS (legal, not an error).
set l {a b c}
lset l 3 X
puts $l
";

#[test]
fn fp_nab_01_append_slot_silent() {
    assert!(
        !fires(FP_NAB_01_REPRO, D, "W231"),
        "FP-NAB-01: append slot must NOT fire W231; {:?}",
        diags(FP_NAB_01_REPRO, D)
    );
}

#[test]
fn fp_nab_01_real_out_of_range_fires() {
    let src = "set l {a b c}\nlset l 4 X\n";
    assert!(
        fires_with_msg(src, D, "W231", "out of range"),
        "FP-NAB-01 TP: lset l 4 X must fire W231 (out of range); {:?}",
        diags(src, D)
    );
}

// FP-NAB-02 — lindex out-of-range returns "" — smell (W230), not error (W231).
const FP_NAB_02_REPRO: &str = "\
set x [lindex {a b c} 9]
return $x
";

#[test]
fn fp_nab_02_lindex_oor_smell_fires() {
    assert!(
        fires(FP_NAB_02_REPRO, D, "W230"),
        "FP-NAB-02 TP: oor lindex must fire W230; {:?}",
        diags(FP_NAB_02_REPRO, D)
    );
}

#[test]
fn fp_nab_02_lindex_oor_not_w231() {
    assert!(
        !fires(FP_NAB_02_REPRO, D, "W231"),
        "FP-NAB-02: lindex oor must NOT escalate to W231; {:?}",
        diags(FP_NAB_02_REPRO, D)
    );
}

#[test]
fn fp_nab_02_lset_same_index_does_w231() {
    let src = "set l {a b c}\nlset l 9 X\n";
    assert!(
        fires_with_msg(src, D, "W231", "out of range"),
        "FP-NAB-02 TP: lset l 9 X must fire W231 (out of range); {:?}",
        diags(src, D)
    );
}

// FP-NAB-04 — `==`/`!=` on strings → W110 / O120 (TP).
#[test]
fn fp_nab_04_string_eq_fires_w110_o120() {
    let src = "if {$x == \"hello\"} { puts y }";
    assert!(
        fires_any(src, D, &["W110", "O120"]),
        "FP-NAB-04 TP: string == must fire W110 or O120; {:?}",
        diags(src, D)
    );
}

// FP-NAB-05 — missing `--` terminator (W304).
#[test]
fn fp_nab_05_file_delete_missing_dash_dash_fires_w304() {
    let src = "proc f {f} { file delete $f }";
    assert!(
        fires(src, D, "W304"),
        "FP-NAB-05 TP: file delete $f must fire W304; {:?}",
        diags(src, D)
    );
}

#[test]
fn fp_nab_05_switch_split_form_missing_dash_dash_fires_w304() {
    let src = "switch $x -nocase {puts hit1} default {puts hit2}";
    assert!(
        fires(src, D, "W304"),
        "FP-NAB-05 TP: split switch form must fire W304; {:?}",
        diags(src, D)
    );
}

#[test]
fn fp_nab_05_braced_switch_form_should_not_fire_w304() {
    // FP: braced pattern-list switch form is unambiguous — W304 must NOT fire.
    let src = "proc f {x} { switch $x { -nocase {puts a} default {puts b} } }";
    assert!(
        !fires(src, D, "W304"),
        "FP-NAB-05: braced switch must NOT fire W304; {:?}",
        diags(src, D)
    );
}

#[test]
fn fp_nab_05_w304_lexical_does_not_cross_proc_boundary() {
    // FP: W304 'currently resolves to ...' must not attribute an outer set to a
    // shadowing proc parameter.
    let src = "set path -force\nproc useit {path} { file delete $path }\n";
    let misattributed = diags(src, D)
        .into_iter()
        .any(|(c, m)| c == "W304" && m.contains("-force"));
    assert!(
        !misattributed,
        "FP-NAB-05: W304 must not attribute outer 'path=-force' to inner param; {:?}",
        diags(src, D)
    );
}

// FP-NAB-06 — `open "|$cmd"` pipe (W103, TP).
#[test]
fn fp_nab_06_open_variable_pipe_fires_w103() {
    let src = "proc f {cmd} { set fh [open \"|$cmd\" r] }";
    assert!(
        fires(src, D, "W103"),
        "FP-NAB-06 TP: open |$cmd must fire W103; {:?}",
        diags(src, D)
    );
}

// FP-NAB-07 — destructive op with variable path (W313, TP).
#[test]
fn fp_nab_07_destructive_variable_path_fires_w313() {
    let src = "proc f {p} { file delete $p }";
    assert!(
        fires(src, D, "W313"),
        "FP-NAB-07 TP: file delete $p must fire W313; {:?}",
        diags(src, D)
    );
}

// FP-NAB-08 — substitution where var-name expected (W212, TP).
#[test]
fn fp_nab_08_set_substituted_name_fires_w212() {
    let src = "proc f {name v} { set $name $v }";
    assert!(
        fires(src, D, "W212"),
        "FP-NAB-08 TP: set $name must fire W212; {:?}",
        diags(src, D)
    );
}

#[test]
fn fp_nab_08_incr_substituted_name_fires_w212() {
    let src = "proc f {x} { incr $x }";
    assert!(
        fires(src, D, "W212"),
        "FP-NAB-08 TP: incr $x must fire W212; {:?}",
        diags(src, D)
    );
}

// FP-NAB-09 — uplevel multi-arg concatenation (W301, TP).
#[test]
fn fp_nab_09_uplevel_multiarg_fires_w301() {
    let src = "proc f {a b} { uplevel 1 puts $a $b }";
    assert!(
        fires(src, D, "W301"),
        "FP-NAB-09 TP: multi-arg uplevel must fire W301; {:?}",
        diags(src, D)
    );
}

// FP-NAB-10 — dialect-aware W002 (dict disabled in tcl8.4, enabled in 9.0).
#[test]
fn fp_nab_10_dict_disabled_in_tcl_8_4_fires_w002() {
    assert!(
        fires("dict create a 1", "tcl8.4", "W002"),
        "FP-NAB-10 TP: dict in tcl8.4 must fire W002; {:?}",
        diags("dict create a 1", "tcl8.4")
    );
}

#[test]
fn fp_nab_10_dict_enabled_in_tcl_9_0_silent() {
    assert!(
        !fires("dict create a 1", "tcl9.0", "W002"),
        "FP-NAB-10: dict in tcl9.0 must NOT fire W002; {:?}",
        diags("dict create a 1", "tcl9.0")
    );
}

// FP-NAB-11 — unresolved command (W123, TP).
#[test]
fn fp_nab_11_unresolved_argparse_fires_w123() {
    assert!(
        fires("argparse {x y}", D, "W123"),
        "FP-NAB-11 TP: argparse must fire W123; {:?}",
        diags("argparse {x y}", D)
    );
}

#[test]
fn fp_nab_11_stub_registered_command_silent() {
    assert!(
        !fires("puts hi", D, "W123"),
        "FP-NAB-11: puts must NOT fire W123; {:?}",
        diags("puts hi", D)
    );
}

// FP-NAB-03 control — an impure proc using puts comes out pure=False. Covered
// as a Rust interproc-purity structure test elsewhere (not a diagnostic).
