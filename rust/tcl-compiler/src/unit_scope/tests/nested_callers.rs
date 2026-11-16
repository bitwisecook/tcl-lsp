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

//! Callers inside nested scripts: `namespace eval`, `catch` and `uplevel` bodies, command substitutions and script values.

use super::*;

/// A `namespace eval ::a { helper }` body calls `::a::helper`, never a
/// same-named proc in the *caller's* namespace (tclsh8.6-confirmed).
///
/// The body is scanned once, as the properly-namespaced body unit lowering
/// registers for it. Walking it a second time through the enclosing
/// statement's `ArgRole::Body` — with the caller's namespace — invents a
/// call to whatever `::helper` happens to exist. Within one file that is
/// invisible (a bare global `::helper` is rarely in a single file's own
/// `known` set); across a project it is a false edge into *another file's*
/// procedure.
#[test]
fn a_namespace_eval_body_resolves_against_its_own_namespace() {
    let reg = registry();
    let src =
        "namespace eval ::a {\n    proc helper {} { return 1 }\n    proc run {} { helper }\n}\n";
    // `::helper` is in scope project-wide, as a global proc in some other
    // file would be — the call inside `::a` must not reach it.
    let evidence = scan_source_call_sites(
        src,
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::a::helper", "::a::run", "::helper"]),
        &[],
    );
    assert_eq!(
        evidence.callees().collect::<Vec<_>>(),
        vec!["::a::helper"],
        "the call binds to ::a::helper only"
    );
    // The in-unit scan is the reference: cross-file must not see more.
    let cu = crate::compilation_unit::CompilationUnit::build_for(src, &reg, false);
    assert_eq!(
        cu.caller_scope.call_sites.callees().collect::<Vec<_>>(),
        vec!["::a::helper"],
    );
}

/// A command substitution nested in a word is a call site (#2134).
///
/// Enumerating only the statements that *are* a command let `[bump m]`
/// go unattributed, so the single visible `bump n` read as the complete
/// caller set and O100 specialised the body to `upvar 1 n v`. Measured on
/// tclsh 9.0.4, the program printed `2 11 11` and the rewritten one
/// `3 10 3`.
#[test]
fn a_nested_substitution_counts_as_a_caller() {
    let reg = registry().project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let src = "proc bump {name} {\n  upvar 1 $name v\n  incr v\n}\nset n 1\nset m 10\nbump n\nset z [bump m]\n";
    let cu = crate::compilation_unit::CompilationUnit::build_for(src, &reg, false);
    assert!(
        cu.caller_scope
            .call_sites
            .callees()
            .any(|callee| callee == "::bump"),
        "the nested `[bump m]` must be attributed to ::bump",
    );
    // The two callers disagree on the argument, so nothing may be seeded.
    assert_eq!(
        cu.caller_scope
            .call_sites
            .get("::bump")
            .and_then(|evidence| evidence.uniform_literal_at(0)),
        None,
        "disagreeing callers must not look uniform",
    );
}

/// Callers that agree still seed — the nested site is *counted*, not
/// treated as opaque.
#[test]
fn agreeing_callers_still_seed_through_a_substitution() {
    let reg = registry().project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
    let src =
        "proc bump {name} {\n  upvar 1 $name v\n  incr v\n}\nset n 1\nbump n\nset z [bump n]\n";
    let cu = crate::compilation_unit::CompilationUnit::build_for(src, &reg, false);
    assert_eq!(
        cu.caller_scope
            .call_sites
            .get("::bump")
            .and_then(|evidence| evidence.uniform_literal_at(0)),
        Some("n"),
        "callers that agree on `n` must still seed",
    );
}

/// An `uplevel` body nested inside another `ArgRole::Body` must not be
/// re-walked with the enclosing unit's namespace.
///
/// The `catch` body is walked correctly — `catch` shifts nothing — and
/// the `uplevel #0` body inside it was then walked again as `::foo`,
/// inventing a call to `::foo::helper` alongside the correct `::helper`
/// that `upframe_scan_bodies` had already recorded with the right
/// frame. Both the phantom callee and the double attribution are wrong:
/// tclsh8.6/9.0 confirm `uplevel #0 { helper b }` inside `::foo::runIt`
/// calls `::helper` and nothing else.
///
/// The `Traits::EVALUATES_IN_SHIFTED_FRAME` skip is what stops it, so
/// no command name appears in the walker.
#[test]
fn retained_upframe_namespaces_do_not_parse_the_procedure_display_label() {
    let registry = registry();
    let module = crate::lowering::lower_to_ir(
        "namespace eval ::N {}\nproc ::N::run {} {uplevel 1 {uplevel 0 {puts $x}}; uplevel 0 {puts $x}; uplevel #0 {puts $x}}",
        &registry,
    );
    let procedures = module
        .procedures
        .values()
        .filter(|procedure| procedure.qualified_name == "::N::run")
        .collect::<Vec<_>>();
    assert_eq!(procedures.len(), 1);
    let mut procedure = (*procedures[0]).clone();
    let parent = procedure
        .body
        .execution_namespace(crate::ir::ExecutionNamespace::RuntimeSelected);
    assert!(!matches!(
        parent,
        crate::ir::ExecutionNamespace::RuntimeSelected
    ));
    procedure.qualified_name = "unrelated reporting label".to_owned();
    let mut callers = Vec::new();
    collect_upframes(&procedure.body, &parent, &registry, &mut callers);
    assert_eq!(callers.len(), 4);
    assert!(matches!(
        callers[0].execution_namespace,
        crate::ir::ExecutionNamespace::RuntimeSelected
    ));
    assert!(matches!(
        callers[1].execution_namespace,
        crate::ir::ExecutionNamespace::RuntimeSelected
    ));
    assert_eq!(callers[2].execution_namespace, parent);
    assert!(
        matches!(
            callers[3].execution_namespace,
            crate::ir::ExecutionNamespace::RuntimeSelected
        ),
        "unsupplied root frame cannot borrow the source body's lexical namespace"
    );
}

#[test]
fn an_uplevel_body_inside_a_catch_body_is_not_reattributed() {
    let reg = registry();
    let src = "proc ::helper {mode} {return $mode}\nnamespace eval ::foo {\n    proc helper {mode} { return $mode }\n    proc runIt {} { catch { uplevel #0 { helper b } } }\n}\n";
    let evidence = scan_source_call_sites(
        src,
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::foo::helper", "::foo::runIt", "::helper"]),
        &[],
    );
    let mut callees: Vec<&str> = evidence.callees().collect();
    callees.sort_unstable();
    assert_eq!(
        callees,
        vec!["::helper"],
        "the global helper takes the evidence and ::foo::helper gets no phantom call",
    );
}

/// `catch { … }` does *not* shift namespace, so its body must still be
/// walked with the caller's — the guard above keys on an absolute
/// `ArgRole::Name`, which `catch` has none of.
#[test]
fn a_catch_body_is_still_walked_with_the_callers_namespace() {
    let reg = registry();
    let evidence = scan_source_call_sites(
        "proc helper {mode} { return $mode }\ncatch { helper prod }\n",
        &reg,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &known(&["::helper"]),
        &[],
    );
    assert_eq!(
        evidence
            .get("::helper")
            .and_then(|e| e.uniform_literal_at(0)),
        Some("prod"),
        "the call inside catch is still evidence"
    );
}

#[test]
fn a_retained_script_value_records_its_actual_caller() {
    let ev = evidence("proc helper {mode} { return $mode }\nset s {helper dev}\neval $s\n");
    assert_eq!(uniform(&ev, "::helper", 0).as_deref(), Some("dev"));
    assert!(!ev.get("::helper").unwrap().opaque_caller);
}

#[test]
fn a_literal_body_mentioning_a_variable_is_still_walked() {
    let ev = evidence("proc helper {mode} { return $mode }\ncatch {helper $x}\n");
    assert_eq!(slot(&ev, "::helper", 0), (Vec::new(), true));
    // The distinction that matters: the body was *walked*, so the call
    // was recorded at its real arity of one.  A withdrawal would have
    // recorded a zero-argument opaque caller instead.
    assert_eq!(
        ev.get("::helper")
            .unwrap()
            .arg_counts
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![1],
        "`catch {{helper $x}}` carries readable script text",
    );
}

#[test]
fn whole_substitution_recognises_only_a_single_reference() {
    assert!(word_is_whole_substitution(
        "$script",
        tcl_lexer::LexerConfig::default()
    ));
    assert!(word_is_whole_substitution(
        "${script}",
        tcl_lexer::LexerConfig::default()
    ));
    assert!(word_is_whole_substitution(
        "[build]",
        tcl_lexer::LexerConfig::default()
    ));
    assert!(
        !word_is_whole_substitution("puts $v", tcl_lexer::LexerConfig::default()),
        "a braced body that merely mentions a variable is literal script text",
    );
    assert!(!word_is_whole_substitution(
        "helper dev",
        tcl_lexer::LexerConfig::default()
    ));
}
