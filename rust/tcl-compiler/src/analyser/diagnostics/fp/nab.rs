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

//! NAB family — not-a-bug / confirm-correct audits. Mostly TP confirmations
//! that a real hazard still fires, plus a few FP guards.
//!
//! Two NAB entries test internal APIs rather than diagnostics and are
//! covered as structure tests elsewhere (not here):
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
/// (NAB-04 accepts W110 *or* its O120 optimiser near-duplicate).
fn diags(src: &str, dialect: &str) -> Vec<(String, String)> {
    let registry = registry_for_dialect(dialect);
    let cu = CompilationUnit::build_for(src, registry, false);
    let d = (!dialect.is_empty()).then(|| tcl_dialect::DialectProfile::by_name(dialect));
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
fn fp_nab_05_dynamic_two_arg_switch_form_should_not_fire_w304() {
    // FN regression: the exemption used to require the trailing word be a
    // braced `Str` literal, missing the equally-safe dynamic 2-arg form —
    // C Tcl's own `TclNRSwitchObjCmd` never scans either trailing word as
    // an option once only `string` + pattern-list remain (`objc - 2`
    // bound), regardless of whether the pattern list is a literal or a
    // variable/command substitution.
    let src = "proc f {x} { set cases {a {puts A} b {puts B}}; switch $x $cases }";
    assert!(
        !fires(src, D, "W304"),
        "FP-NAB-05: dynamic 2-arg switch form must NOT fire W304; {:?}",
        diags(src, D)
    );
}

#[test]
fn fp_nab_05_three_trailing_args_still_fires_w304() {
    // TP control: only the *last two* trailing words are reserved: a third
    // leading dynamic word is still a genuine option-scanning candidate.
    let src = "proc f {a} { switch $a subject {puts hit} }";
    assert!(
        fires(src, D, "W304"),
        "FP-NAB-05: a 3rd trailing dynamic word must still fire W304; {:?}",
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

// FP-NAB-11 — package-gated command used without its `package require`.
// `argparse` is a modelled registry command (`package require argparse`), so
// using it without the require draws W120 ("requires `package require
// argparse`", with an add-the-require fix), not the unknown-command W123.
#[test]
fn fp_nab_11_unrequired_argparse_fires_w120() {
    assert!(
        fires("argparse {x y}", D, "W120"),
        "FP-NAB-11 TP: argparse without require must fire W120; {:?}",
        diags("argparse {x y}", D)
    );
    assert!(
        !fires("argparse {x y}", D, "W123"),
        "FP-NAB-11: a registered package command must not also draw W123; {:?}",
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

// Option-value Body role: a Tk `-command` script value is recursively analysed
// like a positional body (Phase 3), so a structure-dependent lint that requires
// parsing the inner command — W100, unbraced `expr` — fires inside it.
#[test]
fn tk_command_option_body_is_analysed() {
    let src = "button .b -command {expr $x+1}";
    assert!(
        fires(src, "tk", "W100"),
        "-command body should be analysed (W100 on unbraced expr); {:?}",
        diags(src, "tk")
    );
    // A generic-value option's value is a plain string, never a script — the
    // inner `expr` is not parsed as a command, so no W100.
    let neg = "button .b -text {expr $x+1}";
    assert!(
        !fires(neg, "tk", "W100"),
        "-text value must not be analysed as a script; {:?}",
        diags(neg, "tk")
    );
}

// FP-NAB-13 — W002 disabled-in-dialect suppression must honour the same
// same-file resolution rules as the builtin-arity suppression check
// (namespace-scoped shadowing, load-order for top-level vs proc-body calls,
// `interp alias`, static `rename`, and — for the subcommand form — a
// shadowed ensemble head), while a genuinely disabled command with no
// same-file definition anywhere must still fire.

// TN — no shadow anywhere: the plain disabled-command case must still fire.
#[test]
fn fp_nab_13_no_shadow_still_fires_w002() {
    assert!(
        fires("dict create a 1", "tcl8.4", "W002"),
        "FP-NAB-13 TN: a genuinely disabled command with no shadow must fire W002; {:?}",
        diags("dict create a 1", "tcl8.4")
    );
}

// FP — a namespace-scoped proc shadows the disabled builtin for unqualified
// calls resolved inside that namespace (current-namespace-then-global, the
// same rule the builtin-arity check already honoured).
#[test]
fn fp_nab_13_namespace_scoped_proc_shadow_silent() {
    let src = "namespace eval ::ns {\n    proc dict {args} { return $args }\n    dict foo bar\n}\n";
    assert!(
        !fires(src, "tcl8.4", "W002"),
        "FP-NAB-13: a namespace-scoped shadowing proc must suppress W002; {:?}",
        diags(src, "tcl8.4")
    );
}

// FP — a forward-declared proc (defined *after* the call, but inside a proc
// body that only runs once the whole file has loaded) shadows the disabled
// builtin. Proc-body calls are not order-gated.
#[test]
fn fp_nab_13_forward_declared_proc_body_shadow_silent() {
    let src = "proc use_dict {} {\n    dict create a 1\n}\nproc dict {args} { return $args }\n";
    assert!(
        !fires(src, "tcl8.4", "W002"),
        "FP-NAB-13: a proc-body call to a forward-declared shadowing proc must suppress W002; {:?}",
        diags(src, "tcl8.4")
    );
}

// FN guard — the same forward-declared proc, called at the *top level*
// before its own definition, must still fire: top-level commands run in
// source order during load, so the builtin is what actually runs there.
#[test]
fn fp_nab_13_top_level_call_before_shadow_still_fires() {
    let src = "dict create a 1\nproc dict {args} { return $args }\n";
    assert!(
        fires(src, "tcl8.4", "W002"),
        "FP-NAB-13 FN guard: a top-level call before its shadowing proc's \
         definition must still fire W002; {:?}",
        diags(src, "tcl8.4")
    );
}

// FP — `interp alias` establishing the disabled name shadows it exactly like
// a proc would.
#[test]
fn fp_nab_13_interp_alias_shadow_silent() {
    let src = "interp alias {} dict {} list\ndict create a 1\n";
    assert!(
        !fires(src, "tcl8.4", "W002"),
        "FP-NAB-13: an interp alias establishing the name must suppress W002; {:?}",
        diags(src, "tcl8.4")
    );
}

// FP — a static `rename` that moves an existing proc onto the disabled name
// shadows it exactly like a direct `proc` definition would.
#[test]
fn fp_nab_13_rename_shadow_silent() {
    let src = "proc myimpl {args} { return $args }\nrename myimpl dict\ndict create a 1\n";
    assert!(
        !fires(src, "tcl8.4", "W002"),
        "FP-NAB-13: a rename establishing the name must suppress W002; {:?}",
        diags(src, "tcl8.4")
    );
}

// FP — the subcommand-level W002 form (a version-gated *subcommand*, e.g.
// `package files`) is likewise suppressed when the whole ensemble command's
// own name is shadowed by a user proc — the call never reaches the registry
// ensemble at all.
#[test]
fn fp_nab_13_subcommand_form_shadowed_ensemble_head_silent() {
    let src = "proc package {args} { return $args }\npackage files mypackage\n";
    assert!(
        !fires(src, "tcl8.6", "W002"),
        "FP-NAB-13: a shadowed ensemble head must suppress the subcommand-form W002; {:?}",
        diags(src, "tcl8.6")
    );
}

// TN — the subcommand form's shadow check is specific to the *base* command
// name; an unrelated proc elsewhere must not spuriously suppress it.
#[test]
fn fp_nab_13_subcommand_form_unrelated_proc_still_fires() {
    let src = "proc unrelated {} {}\npackage files mypackage\n";
    assert!(
        fires(src, "tcl8.6", "W002"),
        "FP-NAB-13 TN: an unrelated proc must not suppress the subcommand-form W002; {:?}",
        diags(src, "tcl8.6")
    );
}

// FP — an inline `# tcl-lsp: stub` declaration establishes the disabled name
// as a document-global command, unqualified.
#[test]
fn fp_nab_13_stub_declaration_shadow_silent() {
    let src = "# tcl-lsp: stubs-begin\n\
               # tcl-lsp: stub dict {args:var} -loop\n\
               # tcl-lsp: stubs-end\n\
               dict create a 1\n";
    assert!(
        !fires(src, "tcl8.4", "W002"),
        "FP-NAB-13: an inline stub declaration must suppress W002; {:?}",
        diags(src, "tcl8.4")
    );
}

// FP — a `TclOO` class bound to the disabled name is a real command exactly
// like a proc would be (the class-create form binds the factory command
// under that name). `oo::class` itself needs Tcl 8.6+, so this uses
// `lremove` (Tcl 9.0+) under `tcl8.6` rather than `dict` under `tcl8.4`.
#[test]
fn fp_nab_13_tcloo_class_shadow_silent() {
    let src = "oo::class create lremove {\n    constructor {} {}\n}\nlremove create\n";
    assert!(
        !fires(src, "tcl8.6", "W002"),
        "FP-NAB-13: a TclOO class bound to the name must suppress W002; {:?}",
        diags(src, "tcl8.6")
    );
}

// The message enrichment: the "(available in: …)" suffix is read straight
// from the registry's own dialect gate, so it lists exactly the dialects the
// command supports — never a hardcoded per-command string.
#[test]
fn fp_nab_13_message_names_available_dialects() {
    let ds = diags("dict create a 1", "tcl8.4");
    let msg = ds
        .iter()
        .find(|(c, _)| c == "W002")
        .map(|(_, m)| m.as_str())
        .unwrap_or_default();
    assert!(
        msg.contains("available in: Tcl 8.5, Tcl 8.6, Tcl 9.0, Tcl 9.1"),
        "W002 message should name the dialects dict is available in; got {msg:?}"
    );
}

// FP-NAB-03 control — an impure proc using puts comes out pure=False. Covered
// as a Rust interproc-purity structure test elsewhere (not a diagnostic).
