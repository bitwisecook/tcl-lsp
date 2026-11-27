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

//! `info exists` guards and the existence facts I230 folds, over locals, parameters, array elements and instance state.

use super::*;

#[test]
fn info_exists_control_still_flags_w210() {
    // Baseline: a plain read of an unset local flags W210.
    assert!(codes_for("proc f {} { puts $u }").contains(&"W210".to_string()));
}

#[test]
fn info_exists_guard_narrows_read_in_then_arm() {
    // Reads inside `if {[info exists X]}`
    // are guarded — X provably exists there, so no W210.
    let codes = codes_for("proc f {} { if {[info exists u]} { puts $u } }");
    assert!(
        !codes.contains(&"W210".to_string()),
        "guarded read must not flag W210; got {codes:?}",
    );
}

#[test]
fn info_exists_read_outside_guard_still_flags_w210() {
    // The narrowing is scoped to the guarded arm: a read after the
    // `if` (not dominated by the guard) still flags W210.
    let codes = codes_for_in(
        "proc f {} { if {[info exists u]} { puts hi }\nputs $u }",
        "tcl8.6",
    );
    assert!(
        codes.contains(&"W210".to_string()),
        "read outside the guarded arm must still flag W210; got {codes:?}",
    );
}

#[test]
fn info_exists_negated_guard_narrows_false_arm() {
    // The false arm of `![info exists X]`
    // is guarded.
    let codes = codes_for("proc f {} { if {![info exists u]} { puts no } else { puts $u } }");
    assert!(
        !codes.contains(&"W210".to_string()),
        "false-arm read of `![info exists X]` must not flag W210; got {codes:?}",
    );
}

#[test]
fn info_exists_query_word_not_read_before_set() {
    // The existence-query word is
    // not a read-before-set — bare call and command-sub forms.
    assert!(!codes_for("proc f {} { info exists u }").contains(&"W210".to_string()));
    assert!(!codes_for("proc f {} { array exists u }").contains(&"W210".to_string()));
    let codes = codes_for("proc f {} { set y [info exists u]; puts $y }");
    assert!(
        !codes.contains(&"W210".to_string()),
        "`set y [info exists u]` must not flag W210 on u; got {codes:?}",
    );
}

#[test]
fn i230_message_quotes_bare_var_as_source_spells_it() {
    // Message fidelity: `if $n …` must render the condition as `$n`, not
    // the segmenter's re-braced `${n}` reconstruction.
    let src = "set n 1\nif $n { puts a } else { puts b }\n";
    let mut a = Analyser::new();
    a.profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    a.emit_document_cfg_ssa_diagnostics_for_test(src);
    let i230: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code.to_string() == "I230")
        .collect();

    assert!(
        !i230.is_empty(),
        "expected I230: {:?}",
        a.result.diagnostics
    );
    assert!(
        i230[0].message.contains("'$n'"),
        "message must quote the source spelling `$n`, got {:?}",
        i230[0].message
    );
    assert!(
        !i230[0].message.contains("${n}"),
        "message must not re-brace to `${{n}}`, got {:?}",
        i230[0].message
    );
}

#[test]
fn i230_message_keeps_braced_var_spelling() {
    // A source-braced `${n}` stays `${n}` — fidelity cuts both ways.
    let src = "set n 1\nif ${n} { puts a } else { puts b }\n";
    let mut a = Analyser::new();
    a.profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    a.emit_document_cfg_ssa_diagnostics_for_test(src);
    let i230: Vec<_> = a
        .result
        .diagnostics
        .iter()
        .filter(|d| d.code.to_string() == "I230")
        .collect();
    assert!(
        !i230.is_empty(),
        "expected I230: {:?}",
        a.result.diagnostics
    );
    assert!(
        i230[0].message.contains("'${n}'"),
        "message must keep the source's braced spelling, got {:?}",
        i230[0].message
    );
}

/// The existence branch fact is stored once, with its kind
/// (`docs/design/compiler/value-transfers.md` § *Branch facts*). The
/// solver decides `[info exists X]` inside the fixed
/// point, so the unit stores it as an `Applied` fact like any decided
/// branch — reachability follows it — and I230 reports each stored fact
/// once. Under iRules `[info exists ans_cleared]` in one event is not
/// decided when another event sets the variable, which enters `MayBound`
/// there, and a name no handler binds still is.
#[test]
fn the_existence_branch_fact_is_stored_once() {
    use crate::sccp::BranchFactKind;
    let src = "proc f {a} {\n    if {[info exists b]} { puts hi }\n    if {1} { puts one }\n}\n";
    let registry = tcl_registry::CommandRegistry::build_default();
    let unit = crate::compilation_unit::CompilationUnit::build_for(src, &registry, false);
    let function = unit.function("::f").expect("the procedure");
    let kinds: Vec<(&str, BranchFactKind)> = function
        .sccp
        .constant_branches
        .iter()
        .map(|branch| (branch.condition.as_str(), branch.kind))
        .collect();
    assert!(
        kinds.contains(&("[info exists b]", BranchFactKind::Applied))
            && kinds.contains(&("1", BranchFactKind::Applied)),
        "{kinds:?}"
    );
    let decided = function
        .sccp
        .constant_branches
        .iter()
        .find(|branch| branch.condition == "[info exists b]")
        .expect("the existence branch");
    assert!(
        function
            .cfg
            .block_id(&decided.not_taken_target)
            .is_some_and(|id| !function.sccp.executable_blocks.contains(&id)),
        "the dead arm is unreachable"
    );
    assert_eq!(
        codes_for(src).iter().filter(|code| *code == "I230").count(),
        2,
        "one I230 per stored fact"
    );

    let irule = "when HTTP_REQUEST {\n    set ans_cleared 1\n}\nwhen HTTP_RESPONSE {\n    \
                 if {[info exists ans_cleared]} { log local0. cleared }\n    \
                 if {[info exists never_set]} { log local0. never }\n}\n";
    let result = Analyser::new().analyse(irule, "f5-irules");
    let reported: Vec<&str> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::I230)
        .map(|d| d.message.as_str())
        .collect();
    assert!(
        reported.len() == 1 && reported[0].contains("never_set"),
        "{reported:?}"
    );
}

/// Execute the fixture's source analysis under an independently captured native entry.
/// Declared body metadata alone cannot establish an actual existence-fold receipt.
fn native_existence_result(
    src: &str,
    dialect: &str,
) -> crate::environment_ingress::RetainedNativeFixture<crate::analyser::AnalysisResult> {
    let profile = tcl_dialect::DialectProfile::find(dialect).expect("native fixture profile");
    let (owner, captured) = crate::environment_ingress::captured_native_entry_with_owner(profile);
    let entry = crate::command_binding::SourceAnalysisEntry {
        native_entry: Some(std::sync::Arc::new(captured)),
        invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
        native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
            ..Default::default()
        },
        ..crate::command_binding::SourceAnalysisEntry::default()
    };
    let analysis = Analyser::new()
        .with_source_analysis_entry(std::sync::Arc::new(entry))
        .analyse(src, dialect);
    crate::environment_ingress::RetainedNativeFixture::new(analysis, owner)
}

fn native_existence_codes(src: &str) -> Vec<String> {
    native_existence_result(src, "tcl8.6")
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.to_string())
        .collect()
}

#[test]
fn info_exists_folds_false_for_never_defined_local() {
    // A never-defined non-parameter never
    // exists → predicate folds false → I230.
    let codes = native_existence_codes("proc f {a} { if {[info exists b]} { puts hi } }; f VALUE");
    assert!(
        codes.contains(&"I230".to_string()),
        "`info exists` of a never-defined local should fold to I230; got {codes:?}",
    );
}

#[test]
fn info_exists_fresh_activation_absence_does_not_borrow_namespace_contents() {
    // Native proof naming.activation.fresh-local-absence:
    // docs/design/analysis/name-resolution-proofs/activation-fresh-local-absence.md
    let source = include_str!("../../../../../tests/data/native_activation_presence/source.tcl");
    assert_eq!(
        source,
        "set ::b PRESENT; proc f {} {list [info exists b] [info exists Params(key)]}; puts [f]\n"
    );
    let codes =
        native_existence_codes("set ::b PRESENT; proc f {} {if {[info exists b]} {puts hi}}; f");
    assert!(codes.contains(&"I230".to_owned()), "{codes:?}");
}

/// `info exists` runs the read traces of the place it names, and one may
/// bind it: tclsh 8.5-9.1 print `hi` for the traced `f` and `no` for `g`.
#[test]
fn info_exists_does_not_fold_a_place_a_read_trace_may_bind() {
    let callback = "proc cb {n1 n2 op} { upvar 1 $n1 v; set v bound }\n";
    let traced = format!(
        "{callback}proc f {{}} {{\n trace add variable v read cb\n if {{[info exists v]}} {{ puts hi }} else {{ puts no }}\n}}\nf\n"
    );
    assert!(
        !codes_for_in(&traced, "tcl8.6").contains(&"I230".to_owned()),
        "a read trace may bind the queried place"
    );
    let plain = "proc g {} {\n if {[info exists v]} { puts hi } else { puts no }\n}\ng\n";
    assert!(
        codes_for_in(plain, "tcl8.6").contains(&"I230".to_owned()),
        "an untraced never-set local still folds"
    );
}

#[test]
fn info_exists_folds_true_for_parameter() {
    // A parameter always exists → predicate folds true → I230.
    let codes = native_existence_codes("proc f {a} { if {[info exists a]} { puts hi } }; f VALUE");
    assert!(
        codes.contains(&"I230".to_string()),
        "`info exists` of a parameter should fold to I230; got {codes:?}",
    );
}

#[test]
fn info_exists_actual_folds_do_not_borrow_declaration_or_missing_entry() {
    let declaration = "proc f {a} {if {[info exists a]} {puts hi}}";
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    // The convenience driver selects its fresh authoring contract. An unknown
    // incoming interpreter is an explicit, distinct entry in this control.
    let unknown = crate::command_binding::SourceAnalysisEntry {
        unknown_entry: true,
        invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
        ..Default::default()
    };
    let result = Analyser::new()
        .with_source_analysis_entry(std::sync::Arc::new(unknown))
        .analyse(&format!("{declaration}; f VALUE"), "tcl8.6");
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == DiagCode::I230)
    );
    for source in [
        declaration.to_owned(),
        format!("rename info {{}}; proc info args {{return UNKNOWN}}; {declaration}; f VALUE"),
        format!("{declaration}; mystery; f VALUE"),
    ] {
        assert!(
            !native_existence_codes(&source).contains(&"I230".to_owned()),
            "missing actual selected query: {source}"
        );
    }
}

#[test]
fn info_exists_does_not_fold_conditionally_set_var() {
    // A var that is set on some path is not provably set/unset —
    // no fold, no false I230.
    let codes =
        codes_for("proc f {flag} { if {$flag} { set u 1 } ; if {[info exists u]} { puts $u } }");
    assert!(
        !codes.contains(&"I230".to_string()),
        "conditionally-set var must not fold; got {codes:?}",
    );
}

#[test]
fn info_exists_does_not_fold_namespaced_or_array() {
    // Namespaced vars (and elements of namespaced arrays) may be populated
    // outside the function's view — never fold them.  A *local* array's
    // element guard folds on the base name instead (below).
    assert!(
        !codes_for("proc f {} { if {[info exists ::env(PATH)]} { puts hi } }")
            .contains(&"I230".to_string())
    );
}

// Array-element guards: an element of an array this body never
// touches is provably absent — tclsh 9.0.4 / 8.6.16:
//   proc f {} { info exists Params(key) }; f   ;# → 0
// The fold is decided on the *array* name, one-sided (false only), with the
// same scope-alias / instance-state / dynamic-write abstentions as a simple
// name.

#[test]
fn info_exists_folds_false_for_element_of_never_touched_array() {
    // TP — the SpiceGenTcl-adjacent idiom: guarding optional state through an
    // element of an array nothing in the proc ever creates.
    let codes =
        native_existence_codes("proc f {} { if {[info exists Params(key)]} { puts hi } }; f");
    assert!(
        codes.contains(&"I230".to_string()),
        "an element guard on a never-touched array should fold to I230; got {codes:?}",
    );
}

#[test]
fn info_exists_folds_false_for_dynamic_element_of_never_touched_array() {
    // TP — the guard is about the array, so even a runtime-selected element
    // is provably absent when the array itself was never created.
    let codes =
        native_existence_codes("proc f {k} { if {[info exists Params($k)]} { puts hi } }; f VALUE");
    assert!(
        codes.contains(&"I230".to_string()),
        "a dynamic element guard on a never-touched array should fold; got {codes:?}",
    );
}

#[test]
fn info_exists_element_fold_abstains_when_the_array_is_touched() {
    // FP guards — any touch of the base, under either spelling, abstains:
    // a sibling element write, a whole-array `array set`, an `upvar` alias,
    // a `global` / `variable` link, and a parameter base.
    for src in [
        "proc f {} { set Params(other) 1\n if {[info exists Params(key)]} { puts hi } }",
        "proc f {} { array set Params {a 1}\n if {[info exists Params(key)]} { puts hi } }",
        "proc f {} { upvar 1 src Params\n if {[info exists Params(key)]} { puts hi } }",
        "proc f {} { global Params\n if {[info exists Params(key)]} { puts hi } }",
        "proc f {} { variable Params\n if {[info exists Params(key)]} { puts hi } }",
        "proc f {Params} { if {[info exists Params(key)]} { puts hi } }",
    ] {
        let codes = codes_for(src);
        assert!(
            !codes.contains(&"I230".to_string()),
            "a touched / aliased / parameter base must abstain; got {codes:?} for {src}",
        );
    }
}

#[test]
fn info_exists_element_fold_abstains_on_dynamic_writes() {
    // FP guard — `set $n …` may create exactly `Params(key)` (or `Params`),
    // so the never-touched proof is gone.
    let codes = codes_for("proc f {n} { set $n 1\n if {[info exists Params(key)]} { puts hi } }");
    assert!(
        !codes.contains(&"I230".to_string()),
        "a dynamic write must abstain the element fold; got {codes:?}",
    );
}

#[test]
fn info_exists_element_fold_abstains_on_instance_state_arrays() {
    // FP guard — a class-level `variable Params` binds the name to per-object
    // storage: an earlier call on the same instance may have populated it,
    // the same abstention the simple-name fold takes.
    let codes = codes_for(
        "oo::class create C {\n variable Params\n method setit {} { set Params(key) 1 }\n \
         method m {} { if {[info exists Params(key)]} { puts hi } }\n}\n",
    );
    assert!(
        !codes.contains(&"I230".to_string()),
        "an instance-state array element must abstain; got {codes:?}",
    );
}

#[test]
fn info_exists_element_fold_survives_an_unrelated_array() {
    // TP control for the touched-base skip: touching a *different* array must
    // not blanket-disable the fold.
    let codes = native_existence_codes(
        "proc f {} { set Other(key) 1\n if {[info exists Params(key)]} { puts hi } }; f",
    );
    assert!(
        codes.contains(&"I230".to_string()),
        "an unrelated array's write must not disable the fold; got {codes:?}",
    );
}

#[test]
fn info_exists_does_not_fold_unset_parameter() {
    // All six native engines return false after the successful unset. An
    // incoming-formal fact must not override that later physical mutation.
    let result = native_existence_result(
        "proc f {a} { unset a; if {[info exists a]} { puts hi } }; f VALUE",
        "tcl9.0",
    );
    let branches: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == DiagCode::I230)
        .collect();
    assert_eq!(
        branches.len(),
        1,
        "the reached absent cell must retain its false existence fact"
    );
    assert!(
        branches[0].message.contains("always false"),
        "unset parameter must not fold true; got {branches:?}",
    );
}

#[test]
fn info_exists_folds_an_unset_parameter_false() {
    // A parameter that is `unset` before the check no longer exists: the
    // existence rung reads the `unset`, so the guard folds always false and
    // never the parameter's entry "always true" — tclsh 8.4.20 to 9.1b0:
    // `proc f {a} { unset a; info exists a }; f 1` → 0.
    let msgs = i230_messages("proc f {a} { unset a; if {[info exists a]} { puts hi } }");
    assert_eq!(
        msgs.len(),
        1,
        "an unset parameter's guard folds once; got {msgs:?}"
    );
    assert!(msgs[0].contains("always false"), "got {msgs:?}");
}

#[test]
fn info_exists_does_not_fold_scope_alias_locals() {
    // A local bound to out-of-frame storage exists iff the *linked* variable
    // does — runtime state this function cannot see, so the fold must skip
    // it for every alias kind.  tclsh 8.6: `namespace eval ns {variable s
    // ok}; proc t {} {namespace upvar ns s a; info exists a}; t` → 1, and →
    // 0 once `ns::s` is unset.  Pre-fix, `namespace upvar` (whose lowered
    // Call carries no defs, unlike `global`/`variable`/`upvar`) folded
    // "always false" — the `::safe::CheckInterp` guard shape (safe.tcl:109).
    for src in [
        // The two `namespace upvar` forms are the regression; the other
        // alias kinds pin the behaviour they already had via `Call::defs`.
        "proc f {} { namespace upvar ::ns state alias\n if {[info exists alias]} { puts hi } }",
        "proc f {} { namespace upvar ::ns arr(k) alias\n if {[info exists alias]} { puts hi } }",
        "proc f {} { upvar 1 src alias\n if {[info exists alias]} { puts hi } }",
        "proc f {} { global alias\n if {[info exists alias]} { puts hi } }",
        "proc f {} { variable alias\n if {[info exists alias]} { puts hi } }",
    ] {
        let codes = codes_for(src);
        assert!(
            !codes.contains(&"I230".to_string()),
            "existence of a scope-alias local must not fold; got {codes:?} for {src}",
        );
    }
}

#[test]
fn info_exists_fold_survives_unrelated_scope_alias() {
    // TP control for the alias skip: an alias binding for one name must not
    // blanket-disable the fold — a *different*, never-defined local still
    // folds to I230.
    let codes = native_existence_codes(
        "namespace eval ::ns {}; proc f {} { namespace upvar ::ns state alias\n if {[info exists other]} { puts hi } }; f",
    );
    assert!(
        codes.contains(&"I230".to_string()),
        "never-defined `other` must still fold beside an unrelated alias; got {codes:?}",
    );
}

// `info exists` over TclOO instance state
//
// Oracle, tclsh 9.0.4 (TclOO 1.3.1) and tclsh 8.6.14 (TclOO 1.1.0), identical
// on both:
//
// ```tcl
// oo::class create C { variable x; constructor {} { set x 1 }
//                      method m {} { puts [info exists x] } }
// [C new] m                                            ;# → 1
// oo::class create D { variable x
//                      method m {} { puts [info exists x] } }
// [D new] m                                            ;# → 0
// oo::class create F { variable x; method setit {} { set x 42 }
//                      method m {} { puts [info exists x] } }
// set f [F new]; $f m ;# → 0     $f setit; $f m        ;# → 1
// oo::class create G { variable a; constructor {} { set b 7 } }
// oo::define G { variable b; method m {} {
//     puts [list a [info exists a] b [info exists b]] } }
// [G new] m                                            ;# → a 0 b 1
// ```
//
// So a class-level `variable` declaration does *not* create the variable —
// but any earlier method call on the same instance may have, which is a
// per-instance runtime fact no per-method fold can decide.  Abstain.

#[test]
fn info_exists_does_not_fold_instance_var_assigned_in_constructor() {
    // FP guard — `[C new] m` prints 1 on both runtimes, but the fold saw no
    // assignment to `x` inside `m`'s own body and called the guard always
    // false.
    let codes = codes_for(
        "oo::class create C {\n variable x\n constructor {} { set x 1 }\n \
         method m {} { if {[info exists x]} { puts hi } }\n}\n",
    );
    assert!(
        !codes.contains(&"I230".to_string()),
        "an instance variable assigned in the constructor must not fold; got {codes:?}",
    );
}

#[test]
fn info_exists_does_not_fold_instance_var_assigned_in_sibling_method() {
    // FP guard — existence depends on call order on the instance
    // (`$f m` → 0 before `$f setit`, → 1 after), so neither direction folds.
    let codes = codes_for(
        "oo::class create F {\n variable x\n method setit {} { set x 42 }\n \
         method m {} { if {[info exists x]} { puts hi } }\n}\n",
    );
    assert!(
        !codes.contains(&"I230".to_string()),
        "an instance variable a sibling method assigns must not fold; got {codes:?}",
    );
}

#[test]
fn info_exists_does_not_fold_instance_var_declared_in_later_define_block() {
    // FP guard, cross-definition-block shape: `variable b` is declared
    // by an `oo::define` block that lowering may walk *after* the method that
    // queries it.  `MethodDef::instance_vars` is the per-class union precisely
    // so this is order-free — the fold must abstain either way.
    let codes = codes_for(
        "oo::class create G {\n variable a\n constructor {} { set b 7 }\n}\n\
         oo::define G {\n variable b\n \
         method m {} { if {[info exists b]} { puts hi } }\n}\n",
    );
    assert!(
        !codes.contains(&"I230".to_string()),
        "an instance variable declared in a later oo::define block must not fold; got {codes:?}",
    );
}

#[test]
fn info_exists_does_not_fold_my_variable_local_in_method() {
    // FP guard — `my variable x` binds the object's variable into the method
    // frame; existence is the object's, not the frame's.  Already covered by
    // the registry-driven scope-alias skip (`my variable` resolves through the
    // spec's own `ArgRole::VarWrite` resolver), pinned here so the TclOO
    // shape cannot regress.
    let codes = codes_for(
        "oo::class create E {\n method m {} { my variable x\n \
         if {[info exists x]} { puts hi } }\n}\n",
    );
    assert!(
        !codes.contains(&"I230".to_string()),
        "a `my variable` local must not fold; got {codes:?}",
    );
}

#[test]
fn info_exists_still_folds_never_set_non_instance_local_in_method() {
    // TP guard — the abstention is name-scoped, not a blanket "no folds in
    // method bodies": `zzz` is neither instance state nor a parameter nor ever
    // assigned, so it still folds false exactly as it would inside a proc.
    let codes = native_existence_codes(
        "oo::class create C {\n variable x\n constructor {} { ::set x 1 }\n \
         method m {} { ::if {[::info exists zzz]} { ::puts hi } }\n}\n[C new] m\n",
    );
    assert!(
        codes.contains(&"I230".to_string()),
        "a never-set non-instance local in a method body must still fold; got {codes:?}",
    );
}

#[test]
fn info_exists_folds_true_not_false_for_method_parameter() {
    // A *method*'s qualified name is never in `ir_module.procedures`
    // (methods live in `ir_module.methods`), so looking a body's parameters
    // up there makes every method parameter read as a never-defined local
    // and folds the guard **always false** — `[C new] m x` runs the `then`
    // arm on both tclsh 9.0.4 and 8.6.14.  Both consumers of the fold must
    // read the same `MethodDef::params` (as `build_method_units` does), or
    // the analyser and the optimiser disagree.
    let result = native_existence_result(
        "oo::class create C {\n method m {p} { ::if {[::info exists p]} { ::puts hi } }\n}\n[C new] m VALUE\n",
        "tcl8.6",
    );
    let i230: Vec<&str> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::I230)
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(i230.len(), 1, "a method parameter must fold; got {i230:?}");
    assert!(
        i230[0].contains("always true"),
        "a method parameter exists at entry — the fold must be true, got {i230:?}",
    );
}

#[test]
fn info_exists_does_not_fold_upvar_defined_local_across_my_dispatch() {
    // FP guard — the callee reached via `my` is a method,
    // never in the upvar-procs table, so its `upvar 1 $refvar ref`
    // caller-frame definition was invisible and the guard folded always
    // false.  Oracle (tclsh 9.0.4 and 8.6.14): after `my Reference?
    // $lookup ref` returns true, `[info exists ref]` in the caller is 1;
    // on a miss, 0 — live code either way, so the fold must abstain.
    let codes = codes_for(
        "oo::class create Formatter {\n \
         method Reference? {lookup refvar} {\n upvar 1 $refvar ref\n \
         if {$lookup eq \"x\"} { set ref 1; return 1 }\n return 0\n }\n \
         method ResolvableReference? {lookup} {\n \
         my Reference? $lookup ref\n \
         if {[info exists ref]} { puts hi }\n }\n}\n",
    );
    assert!(
        !codes.contains(&"I230".to_string()),
        "an upvar-defined local across a `my` dispatch must not fold; got {codes:?}",
    );
}

#[test]
fn info_exists_does_not_fold_upvar_defined_local_across_next_dispatch() {
    // Abstention guard, `next`-chain shape.  `next` names no
    // target at all, so which implementation runs — and which frame its
    // `upvar 1` lands in — needs MRO modelling this analysis does not do.
    // Oracle (tclsh 9.0.4): the chained implementation's
    // `upvar 1` actually SKIPS the calling implementation's frame and
    // lands in the frame of whoever invoked the whole method
    // (`in-sub:0`, `global:1` for a top-level `[Sub new] probe`) — so a
    // per-chain answer is provable only with the chain in hand.  Until
    // then the fold must abstain rather than claim either constant.
    let codes = codes_for(
        "oo::class create Base {\n \
         method probe {} { upvar 1 status status\n set status ok\n return 1 }\n}\n\
         oo::class create Sub {\n superclass Base\n \
         method probe {} {\n next\n \
         if {[info exists status]} { puts hi }\n }\n}\n",
    );
    assert!(
        !codes.contains(&"I230".to_string()),
        "an upvar-defined local across a `next` dispatch must not fold; got {codes:?}",
    );
}

#[test]
fn info_exists_still_folds_across_my_when_no_method_reaches_the_caller_frame() {
    // Tcl 9.0.4 resolves the relative `my` command head at runtime and an
    // object's namespace may shadow it, so the fold must abstain.
    let codes = codes_for(
        "oo::class create C {\n \
         method Helper {} { ::return 1 }\n \
         method m {} {\n my Helper\n \
         ::if {[::info exists zzz]} { ::puts hi }\n }\n}\n",
    );
    assert!(
        !codes.contains(&"I230".to_string()),
        "relative `my` dispatch must make the fold abstain; got {codes:?}",
    );
}

#[test]
fn read_after_my_dispatch_to_an_upvar_sibling_draws_no_read_before_set() {
    // The W210 face of the same evidence rule: `$ref` after
    // the dispatch is genuinely defined on the hit path — tclsh 9.0.4 /
    // 8.6.14 run it to completion.
    let codes = codes_for(
        "oo::class create Formatter {\n \
         method Reference? {lookup refvar} {\n upvar 1 $refvar ref\n \
         set ref 1\n return 1\n }\n \
         method ResolvableReference? {lookup} {\n \
         my Reference? $lookup ref\n \
         return $ref\n }\n}\n",
    );
    assert!(
        !codes.contains(&"W210".to_string()),
        "a `my` dispatch to an upvar sibling defines the local; got {codes:?}",
    );
}

/// Every I230 message emitted for `src`, in emission order.
fn i230_messages(src: &str) -> Vec<String> {
    native_existence_result(src, "tcl8.6")
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::I230)
        .map(|d| d.message.clone())
        .collect()
}

#[test]
fn info_exists_folds_true_for_method_parameter_shadowing_an_instance_var() {
    // Confirmed against the oracle: a formal parameter whose name collides with a class-level `variable`
    // declaration **shadows** it outright: the name is an ordinary local that
    // always exists, and writes through it never reach object state.  So the
    // instance-variable abstention must not swallow it.
    //
    // tclsh 9.0.4 (TclOO 1.3.1) and 8.6.14 (TclOO 1.1.0), identical:
    //
    //   oo::class create A1 { variable x; constructor {} { set x 42 }
    //                         method m {x} { set r [list [info exists x] $x]
    //                                        set x 9; return $r }
    //                         method peek {} { list [info exists x] $x } }
    //   set a [A1 new]; $a m hello  ;# → exists 1 value hello
    //   $a peek                     ;# → exists 1 value 42   (`set x 9` did
    //                                                         not reach it)
    //
    // and with the instance variable never assigned, the parameter still
    // binds (`A2 in-method: exists 1 value hello`, `A2 after m: 0`).
    let msgs = i230_messages(
        "oo::class create A {\n variable x\n constructor {} { ::set x 42 }\n \
         method m {x} { ::if {[::info exists x]} { ::puts hi } }\n}\n[A new] m VALUE\n",
    );
    assert_eq!(
        msgs.len(),
        1,
        "the shadowing parameter must still fold; got {msgs:?}",
    );
    assert!(
        msgs[0].contains("always true"),
        "a parameter shadowing an instance variable always exists — the fold \
         must be true, got {msgs:?}",
    );
}

#[test]
fn info_exists_still_abstains_on_the_non_shadowed_instance_vars() {
    // Companion control for the shadowing case: shadowing is per-name.  `x`
    // is shadowed by the parameter and folds; the sibling instance variable
    // `y`, which nothing in this method binds, must keep abstaining.
    let msgs = i230_messages(
        "oo::class create A {\n variable x y\n constructor {} { ::set x 42 }\n \
         method m {x} {\n ::if {[::info exists x]} { ::puts hi }\n \
         ::if {[::info exists y]} { ::puts ho }\n }\n}\n[A new] m VALUE\n",
    );
    assert_eq!(
        msgs.len(),
        1,
        "only the shadowed name may fold; got {msgs:?}",
    );
    assert!(msgs[0].contains("always true"), "got {msgs:?}");
}

#[test]
fn info_exists_frame_facts_survive_a_proc_method_qname_collision() {
    // Confirmed against the oracle: a `TclOO` method and a namespace
    // procedure can carry the same qualified
    // name, so the per-function dispatcher must take its frame identity from
    // the caller (which knows the map it is walking) rather than probing
    // `ir_module.procedures` / `.methods` by name.
    //
    // tclsh 9.0.4 and 8.6.14, identical — note `oo::class create C` does not
    // create namespace `::C`, so the procedure needs `namespace eval` first:
    //
    //   oo::class create C { variable x; constructor {} { set x 42 }
    //                        method m {p} { list method [info exists p] $p $x } }
    //   namespace eval ::C {}
    //   proc ::C::m {q} { list proc [info exists q] $q [info exists x] }
    //   [C new] m hello   ;# → method 1 hello 42
    //   ::C::m world      ;# → proc 1 world 0
    //
    // Both frames are wholly separate: the procedure sees none of the class's
    // instance state. Pre-fix the name probe gave the *method* the
    // procedure's parameter list (so its own `[info exists p]` folded "always
    // false") and gave the *procedure* the class's instance variables (so it
    // abstained on `x`, a name it fully owns as a never-set local).
    let msgs = i230_messages(
        "oo::class create C {\n variable x\n constructor {} { ::set x 42 }\n \
         method m {p} { ::if {[::info exists p]} { ::puts inmethod } }\n}\n\
         namespace eval ::C {}\n\
         proc ::C::m {q} {\n if {[info exists q]} { puts inproc }\n \
         if {[info exists x]} { puts nope }\n}\n[C new] m VALUE; ::C::m VALUE\n",
    );
    // Method: `p` is its own parameter → always true.
    // Proc: `q` is its own parameter → always true; `x` is a never-set local
    // of the *procedure* (the class's instance state is not in scope there)
    // → always false.
    assert_eq!(
        msgs.len(),
        3,
        "each frame must fold its own three guards; got {msgs:?}",
    );
    assert_eq!(
        msgs.iter().filter(|m| m.contains("always true")).count(),
        2,
        "both parameter guards must fold true; got {msgs:?}",
    );
    assert_eq!(
        msgs.iter().filter(|m| m.contains("always false")).count(),
        1,
        "the procedure's never-set `x` must fold false — it is not object \
         state in a procedure frame; got {msgs:?}",
    );
}

#[test]
fn info_exists_folds_true_for_an_apply_lambda_parameter() {
    // The lambda loop hands the dispatcher the body unit's own `Procedure`
    // rather than letting it probe
    // `ir_module.procedures` for a key that map never holds, so a lambda's
    // parameters reach the fold like a proc's.  A lambda is an anonymous
    // procedure with a fresh frame whose bound names are exactly its
    // parameter list — tclsh 9.0.4 / 8.6.14: `apply {{a} {info exists a}} 1`
    // → 1.
    let msgs = i230_messages("apply {{a} { if {[info exists a]} { puts hi } }} 1\n");
    assert_eq!(msgs.len(), 1, "a lambda parameter must fold; got {msgs:?}");
    assert!(
        msgs[0].contains("always true"),
        "a lambda parameter exists at entry, got {msgs:?}",
    );
}
