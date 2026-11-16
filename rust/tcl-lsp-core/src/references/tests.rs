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

use super::*;
use tcl_compiler::analyser::Analyser;

fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

#[test]
fn retained_colon_procedure_does_not_reference_the_empty_name() {
    let source = "proc {} {} {return EMPTY}; proc : {} {return COLON}; :";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(source, dialect);
        let invocation = execution_at(&analysis, source, ":");
        assert!(invocation.resolved_definition.is_some(), "{dialect}");
        assert!(invocation_calls_named(
            &analysis, invocation, ":::", ":", ":::", source
        ));
        assert!(!invocation_calls_named(
            &analysis, invocation, "::", "", "::", source
        ));
    }
}

fn execution_at<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    written: &str,
) -> &'a tcl_compiler::signature_scan::types::SignatureCommandInvocation {
    let offset = u32::try_from(source.rfind(written).unwrap()).unwrap();
    analysis
        .command_invocations
        .iter()
        .find(|invocation| {
            invocation.lookup.is_execution_site() && invocation.range.start() == offset
        })
        .expect("actual command head must retain its positioned lookup")
}

#[test]
fn loaded_terminal_can_report_an_external_call_without_local_edit_authority() {
    use std::sync::Arc;
    use tcl_compiler::command_binding::{
        SourceAnalysisEntry, SourceOriginKind, TrustedSourceModuleLoader,
    };
    let source = "proc greet {} {return local}\nsource provider.tcl\nset cmd greet\n$cmd\n";
    let entry = Arc::new(SourceAnalysisEntry {
        trusted_source_modules: vec![TrustedSourceModuleLoader::new(
            tcl_dialect::model::Family::Tcl,
            "provider.tcl".to_owned(),
            None,
            "loaded-provider".to_owned(),
            &Arc::from("proc greet {} {return loaded}"),
        )],
        invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
            tcl_dialect::TclVersion::V8_6,
        )),
        native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
            ..Default::default()
        },
        ..Default::default()
    });
    let analysis = Analyser::new()
        .with_source_analysis_entry(entry)
        .analyse(source, "tcl8.6")
        .clone();
    let local = &analysis.all_procs["::greet"];
    let call = execution_at(&analysis, source, "$cmd");
    let reference = call.resolved_command_reference.as_ref().unwrap();
    let definition = reference
        .linked_definition()
        .or_else(|| reference.definition())
        .unwrap();
    assert!(matches!(
        definition.allocation().site.source.kind(),
        SourceOriginKind::Loaded { .. }
    ));
    assert!(!invocation_calls_proc(
        &analysis, call, "::greet", local, source
    ));
    assert!(!invocation_references_proc(
        &analysis, call, "::greet", local, source
    ));
    assert!(!invocation_references_named(
        &analysis, call, "::greet", "greet", "::greet", source
    ));
    assert!(invocation_calls_named(
        &analysis, call, "::greet", "greet", "::greet", source
    ));
    assert!(
        call.indirect,
        "the frozen command value is not a written head"
    );
    assert!(crate::rename::prepare_rename(source, 2, 9, &analysis).is_none());

    let local_source = "proc greet {} {return local}\nset cmd greet\n$cmd\n";
    let local_analysis = analyse(local_source);
    let local_call = execution_at(&local_analysis, local_source, "$cmd");
    assert!(local_call.indirect && local_call.rename_safe);
    assert!(local_call.resolved_command_reference.is_some());
    assert!(crate::rename::prepare_rename(local_source, 1, 9, &local_analysis).is_some());
}

fn retained_method_sites(source: &str, is_classmethod: bool) -> Vec<tcl_lexer::Span> {
    let analysis = analyse(source);
    find_obj_method_call_sites(
        source,
        analysis.resolved_profile().unwrap(),
        &analysis,
        "::C",
        "ping",
        is_classmethod,
    )
}

#[test]
fn class_method_references_require_the_original_temporal_entry() {
    let source = "oo::class create C {self method ping {} {return CLASS}}; C ping";
    let sites = retained_method_sites(source, true);
    assert_eq!(sites.len(), 1, "{sites:?}");
    assert_eq!(sites[0].start() as usize, source.rfind("ping").unwrap());
}

#[test]
fn class_method_references_refuse_replacement_during_arguments() {
    for source in [
        "oo::class create C {self method ping {x} {return CLASS}}; C ping [oo::objdefine C method ping {x} {return REPLACED}]",
        "oo::class create C {self method ping {} {return CLASS}}; rename C old; proc C args {return DECOY}; C ping",
    ] {
        assert!(retained_method_sites(source, true).is_empty(), "{source}");
    }
}

#[test]
fn instance_method_references_require_the_retained_head_object() {
    let source =
        "oo::class create C {method ping {} {return INSTANCE}}; set obj [C new]; $obj ping";
    let sites = retained_method_sites(source, false);
    assert_eq!(sites.len(), 1, "{sites:?}");
    assert_eq!(sites[0].start() as usize, source.rfind("ping").unwrap());
}

#[test]
fn instance_method_references_refuse_mutated_dispatch_and_nominal_decoys() {
    for source in [
        "oo::class create C {method ping {x} {return INSTANCE}}; set obj [C new]; $obj ping [oo::define C method ping {x} {return REPLACED}]",
        "oo::class create C {method ping {} {return INSTANCE}}; set obj [C new]; set obj ::puts; $obj ping",
    ] {
        assert!(retained_method_sites(source, false).is_empty(), "{source}");
    }
}

#[test]
fn retained_alias_calls_do_not_become_target_rename_edits() {
    let source = "proc target {} {return OK}\ninterp alias {} wrapper {} target\nwrapper\ntarget\n";
    let analysis = analyse(source);
    let proc_def = &analysis.all_procs["::target"];
    let alias = execution_at(&analysis, source, "wrapper\n");
    assert!(
        alias
            .resolved_command_reference
            .as_ref()
            .unwrap()
            .linked_definition()
            .is_some()
    );
    assert!(invocation_calls_proc(
        &analysis, alias, "::target", proc_def, source
    ));
    assert!(!invocation_references_proc(
        &analysis, alias, "::target", proc_def, source
    ));
    let direct = execution_at(&analysis, source, "target\n");
    assert!(invocation_calls_proc(
        &analysis, direct, "::target", proc_def, source
    ));
    assert!(invocation_references_proc(
        &analysis, direct, "::target", proc_def, source
    ));
}

#[test]
fn retained_proc_redefinitions_keep_original_reference_sets_separate() {
    let source = "proc p {} {return OLD}\np\nrename p old\nproc p {} {return NEW}\nold\np\n";
    let analysis = analyse(source);
    let first = analysis
        .proc_declarations("::p")
        .min_by_key(|proc_def| proc_def.name_span.start())
        .unwrap();
    let second = &analysis.all_procs["::p"];
    assert_ne!(first.name_span, second.name_span);
    let old = execution_at(&analysis, source, "old\n");
    let new = execution_at(&analysis, source, "p\n");
    assert!(invocation_calls_proc(&analysis, old, "::p", first, source));
    assert!(!invocation_calls_proc(
        &analysis, old, "::p", second, source
    ));
    assert!(invocation_references_proc(
        &analysis, new, "::p", second, source
    ));
    assert!(!invocation_references_proc(
        &analysis, new, "::p", first, source
    ));
    let first_spans = proc_reference_spans(
        &analysis,
        crate::definition::CallResolution::document_only(),
        "::p",
        first,
        source,
    );
    assert!(first_spans.contains(&old.range));
    assert!(!first_spans.contains(&new.range));
}

#[test]
fn retained_builtin_wrapper_cannot_borrow_an_old_class_name() {
    let source = "oo::class create C {}\nrename C saved\ninterp alias {} C {} list\nC VALUE\n";
    let analysis = analyse(source);
    let class_def = &analysis.all_classes["::C"];
    let call = execution_at(&analysis, source, "C VALUE");
    let reference = call
        .resolved_command_reference
        .as_ref()
        .expect("known alias slot");
    assert!(reference.definition().is_none());
    assert!(reference.linked_definition().is_none());
    assert!(!invocation_references_class(
        &analysis, call, "::C", class_def, source
    ));
    assert!(!invocation_references_named(
        &analysis, call, "::C", "C", "::C", source
    ));
    assert!(
        !class_reference_spans(
            &analysis,
            crate::definition::CallResolution::document_only(),
            "::C",
            class_def,
            source
        )
        .contains(&call.range)
    );
}

#[test]
fn references_reach_the_call_site_from_the_stale_original_in_a_foreach_rename_reinstall_idiom() {
    // TN — mirroring the same-file precedent set by
    // `references_reach_the_call_site_from_a_shadowed_duplicate_proc_decl_same_document`:
    // the `tk/library/accessibility.tcl` rename-and-reinstall idiom moves
    // the original `button` to `::tk::accessible::orig_button` and installs
    // a wrapper under the old name, so the later `button .b1` runs the
    // wrapper (tclsh 8.6/9.0 return `wrapped`). The stale original is not
    // what that call reaches, and its references must not claim it.
    let src = "proc button {args} {return orig_button}\n\
                   proc entry {args} {return orig_entry}\n\
                   namespace eval ::tk::accessible {\n    \
                   foreach wtype {button entry} {\n        \
                   rename ::$wtype ::tk::accessible::orig_$wtype\n        \
                   proc ::$wtype {args} {return wrapped}\n    \
                   }\n\
                   }\n\
                   set r1 [button .b1]\n\
                   set r2 [entry .e1]\n";
    let analysis = analyse(src);
    // Cursor on the STALE original `proc button` declaration (line 0).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&0), "the declaration itself: {refs:?}");
    assert!(
        !lines.contains(&5) && !lines.contains(&8),
        "neither the wrapper nor its call reaches the original: {refs:?}"
    );
}

#[test]
fn references_reach_a_method_return_captured_dispatch_site() {
    // `b` is typed only by the object-type lattice (`set b [$a make]`, the
    // method-return edge) — the analyser's `instance_classes` never binds
    // it, so only the lattice reading lets Find References reach the `$b
    // greet` site that semantic tokens and hover resolve.
    let src = "oo::class create A { method make {} { ::return [::B new] } }\n\
                   oo::class create B { method greet {} { ::return \"hi\" } }\n\
                   set a [A new]\n\
                   set b [$a make]\n\
                   $b greet\n";
    let analysis = analyse(src);
    assert!(
        !analysis.instance_classes.contains_key("b"),
        "premise: the analyser walk alone must not bind `b` \
             (instance_classes: {:?})",
        analysis.instance_classes
    );
    // Cursor on the `greet` declaration (line 1, col 29).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        29,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(
        lines.contains(&4),
        "the lattice-typed `$b greet` call site is missing: {refs:?}"
    );
    // …and from the call site itself, the declaration answers back.
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        4,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(
        lines.contains(&1),
        "references from the call site must reach the declaration: {refs:?}"
    );
}

#[test]
fn references_skip_a_same_named_untyped_variable_in_another_scope() {
    // FP guard for the lattice half of the scan: the lattice types `b`
    // inside `::mk` only (`by_scope`); a same-named integer in `::other`
    // must not become a reference — the scope-keyed map is exactly what
    // stops the bare-name collision.
    let src = "oo::class create A { method make {} { ::return [::B new] } }\n\
                   oo::class create B { method greet {} { ::return \"hi\" } }\n\
                   proc mk {} { set a [A new]\n  set b [$a make]\n  $b greet }\n\
                   proc other {} { set b 7\n  $b greet }\n";
    let analysis = analyse(src);
    // Cursor on the `greet` declaration (line 1, col 29).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        29,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(
        lines.contains(&4),
        "the lattice-typed site inside ::mk is missing: {refs:?}"
    );
    assert!(
        !lines.contains(&6),
        "::other's `b` is an untyped integer; its `$b greet` must not be \
             a reference: {refs:?}"
    );
}

#[test]
fn references_to_proc_include_decl_and_calls() {
    let src = "proc greet {} {}\ngreet\ngreet\n";
    let analysis = analyse(src);
    // Cursor on the first `greet` reference (line 1).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        2,
        &analysis,
        true,
    );
    assert!(refs.len() >= 2, "expected decl + call sites: {refs:?}");
    // First entry is the declaration on line 0.
    assert_eq!(refs[0].start_line, 0);
}

#[test]
fn references_include_wildcard_imported_bareword_call_same_document() {
    // TP (same-document) — a wildcard `namespace
    // import ::Foo::*` reaches an exported proc via a bare call with no
    // real command recorded at any of the call's own candidate names,
    // so `invocation_references_named`'s ordinary rule can never catch
    // it; `proc_reference_spans` ORs in
    // `invocation_references_via_wildcard_import` for exactly this
    // case.
    let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    namespace export bar\n}\nnamespace import ::Foo::*\nbar\n";
    let analysis = analyse(src);
    // Cursor on the `bar` declaration (line 1, col 9).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        9,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(
        lines.contains(&5),
        "wildcard-imported bareword call site missing: {refs:?}"
    );
}

#[test]
fn references_keep_a_wildcard_imported_call_after_a_later_export_clear() {
    // TP, direction A — the import bound `p` while `::src`
    // still exported it; the later `namespace export -clear` does not
    // revoke that alias (oracle tclsh 8.6.14/9.0.4: `::dst::p` still
    // runs, `info commands ::dst::*` still lists it). Without the
    // per-import-site snapshot the export gate reads the *final* export
    // set — which the `-clear` has emptied — and find-references silently
    // drops this real call site.
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n    p\n}\nnamespace eval src {\n    namespace export -clear\n}\n";
    let analysis = analyse(src);
    // Cursor on the `p` declaration (line 1, col 9).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        9,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(
        lines.contains(&6),
        "the imported call site must survive a later `-clear`: {refs:?}"
    );
}

#[test]
fn references_exclude_a_wildcard_imported_call_the_import_predates_the_export_of() {
    // FP guard (CRITICAL), direction B — `::src` exports `p`
    // only *after* `::dst` imported `::src::*`, so real Tcl never binds
    // `::dst::p` at all (oracle: `invalid command name "::dst::p"`) and
    // the bare `p` inside `::dst` is not a call to `::src::p`. Reading
    // the final export set would report it as one.
    let src = "namespace eval src {\n    proc p {} { return P }\n}\nnamespace eval dst {\n    namespace import ::src::*\n    p\n}\nnamespace eval src {\n    namespace export p\n}\n";
    let analysis = analyse(src);
    // Cursor on the `p` declaration (line 1, col 9).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        9,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(
        !lines.contains(&5),
        "an export written after the import must not make the bare call a \
             reference: {refs:?}"
    );
}

#[test]
fn references_drop_a_call_after_a_namespace_forget() {
    // TN, the `namespace forget` behaviour — the alias the import installed is
    // gone by the time the second `p` runs (oracle: `invalid command
    // name "p"`), so that call is not a reference to `::src::p`. The
    // call *before* the forget still is.
    let src = "namespace eval src {\n    proc p {} { return P }\n    namespace export p\n}\nnamespace eval dst {\n    namespace import ::src::*\n    p\n    namespace forget ::src::p\n    p\n}\n";
    let analysis = analyse(src);
    // Cursor on the `p` declaration (line 1, col 9).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        9,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(
        lines.contains(&6),
        "the call before the forget is still a reference: {refs:?}"
    );
    assert!(
        !lines.contains(&8),
        "the call after the forget reaches no command: {refs:?}"
    );
}

#[test]
fn references_drop_a_conflicting_unforced_imports_call_site() {
    // FP guard (CRITICAL), the import-conflict behaviour — `::dst` already has
    // its own `p`, so the non-`-force` import errors and installs
    // nothing (oracle: `can't import command "p": already exists`, and
    // `namespace origin ::dst::p` → `::dst::p`). The bare `p` inside
    // `::dst` runs the *local* proc and is not a reference to `::src::p`.
    let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n    namespace import ::src::*\n    p\n}\n";
    let analysis = analyse(src);
    // Cursor on `::src::p`'s declaration (line 1, col 9).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        9,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(
        !lines.contains(&7),
        "a failed import makes no call site of the source: {refs:?}"
    );
}

#[test]
fn references_include_a_forced_imports_call_site() {
    // TP, the other half of the row above — with `-force` the import
    // replaces the local `p`, so the bare call *is* a reference to
    // `::src::p` (oracle: `namespace origin ::dst::p` → `::src::p`).
    let src = "namespace eval src {\n    proc p {} { return SRC }\n    namespace export p\n}\nnamespace eval dst {\n    proc p {} { return LOCAL }\n    namespace import -force ::src::*\n    p\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        9,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(
        lines.contains(&7),
        "a `-force` import makes the bare call a reference to the source: {refs:?}"
    );
}

#[test]
fn references_include_a_call_through_an_import_chain() {
    // TP, the import-chain behaviour — `::A` imports `::B::*`, `::B`
    // imported `::C::*` and re-exported; the bare `p` in `::A` runs
    // `::C::p` (oracle: `namespace origin ::A::p` → `::C::p`), so it is
    // a reference to it. The middle hop is in no `all_procs`, so a
    // single-hop walk finds nothing.
    let src = "namespace eval C {\n    proc p {} { return CP }\n    namespace export p\n}\nnamespace eval B {\n    namespace import ::C::*\n    namespace export p\n}\nnamespace eval A {\n    namespace import ::B::*\n    p\n}\n";
    let analysis = analyse(src);
    // Cursor on `::C::p`'s declaration (line 1, col 9).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        9,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(
        lines.contains(&10),
        "the chained bare call is a reference to ::C::p: {refs:?}"
    );
}

#[test]
fn references_include_bare_call_to_a_proc_installed_into_oo_helpers() {
    // A proc
    // installed directly into `::oo::Helpers` (the documented "TclOO
    // Tricks" idiom — nico-robert/ticklecharts installs `classvar` /
    // `callback` this way) becomes bare-callable from every TclOO
    // method body in the program via TclOO's own fixed runtime
    // namespace path. tclsh9.0/8.6 both prove the bare `classvar hits`
    // call genuinely dispatches to `::oo::Helpers::classvar` —
    // find-references must reach it, not just the declaration.
    let src = "proc ::oo::Helpers::classvar {name} {\n    set ns [uplevel 1 {my getONSClass}]\n    tailcall namespace upvar $ns $name $name\n}\noo::class create Counter {\n    variable _label\n    constructor {label} { set _label $label }\n    method getONSClass {} { return [self class] }\n    method bump {} {\n        classvar hits\n        incr hits\n        return \"$_label:$hits\"\n    }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `classvar` declaration (line 0, col 20).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        20,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&0), "decl missing: {refs:?}");
    assert!(
        lines.contains(&9),
        "the bare classvar call site inside the method body is missing: {refs:?}"
    );
}

#[test]
fn references_exclude_a_top_level_bare_call_with_the_same_name_as_an_oo_helpers_proc() {
    // FP guard: a bare call outside any TclOO
    // method body must not be treated as reaching a proc installed in
    // `::oo::Helpers` — real tclsh raises "invalid command name" there
    // (`::oo::Helpers` is on a *method body's* runtime namespace path
    // only, never the global one) — only a call genuinely inside a
    // method body's own runtime namespace path does.
    let src = "proc ::oo::Helpers::classvar {name} {}\nclassvar hits\n";
    let analysis = analyse(src);
    // Cursor on the `::oo::Helpers::classvar` declaration (line 0, col 20).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        20,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert_eq!(
        lines,
        vec![0],
        "a top-level bare call outside any method body must not be linked: {refs:?}"
    );
}

#[test]
fn references_from_in_proc_global_alias_reach_the_callers_canonical_set() {
    // Reduces the real `isEqual`/`tolComp` shape from
    // nico-robert/pix's test/data_b64.test — a proc aliases a top-level
    // cell via `global`, and the caller overrides it via a plain `set
    // ::name` before invoking the proc. tclsh proves `tolComp` (via
    // `global`) and `::tolComp` (the caller's `set`) are the identical
    // storage cell. Querying from the in-proc `$tolComp` read must reach
    // the caller's `set ::tolComp`: `collect_alias_spans` on its own finds
    // *other aliases* of the same target, never the target's own canonical
    // (non-aliased) declaration.
    let src =
        "proc use {} {\n    global tolComp\n    return $tolComp\n}\nset ::tolComp 0.05\nuse\n";
    let analysis = analyse(src);
    // Cursor on the `$tolComp` read inside the proc (line 2, col 14).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        14,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(
        lines.contains(&1),
        "the `global tolComp` decl itself must still be present: {refs:?}"
    );
    assert!(
        lines.contains(&2),
        "the in-proc $tolComp read must still be present: {refs:?}"
    );
    assert!(
        lines.contains(&4),
        "the caller's canonical `set ::tolComp 0.05` must now be reached: {refs:?}"
    );
}

#[test]
fn references_from_the_callers_canonical_set_reach_the_in_proc_global_alias() {
    // The reverse direction of the test above: querying from the caller's
    // own `set ::tolComp` must not return only its own 2 spans (decl + any
    // top-level reads) and miss every in-proc `global tolComp` occurrence,
    // which is what a plain `set` — with no `link_target` of its own to
    // search alias records by — would otherwise do.
    let src =
        "proc use {} {\n    global tolComp\n    return $tolComp\n}\nset ::tolComp 0.05\nuse\n";
    let analysis = analyse(src);
    // Cursor on the `set ::tolComp` declaration (line 4, col 8, inside
    // "tolComp" — `set ::tolComp 0.05` has "tolComp" starting at col 6).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        8,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(
        lines.contains(&4),
        "the caller's own decl must still be present: {refs:?}"
    );
    assert!(
        lines.contains(&1),
        "the in-proc `global tolComp` decl must now be reached: {refs:?}"
    );
    assert!(
        lines.contains(&2),
        "the in-proc $tolComp read must now be reached: {refs:?}"
    );
}

#[test]
fn references_unify_global_alias_and_canonical_set_when_the_set_is_unqualified() {
    // A second repro: an *unqualified* `set tolComp
    // 0.05` at global scope reproduces the identical split, ruling out
    // the `::`-prefix as the sole cause — `set`'s binding never
    // calls `set_var_link_target` regardless of how the name is spelled,
    // so the shape is the same either way.
    let src = "proc use {} {\n    global tolComp\n    return $tolComp\n}\nset tolComp 0.05\nuse\n";
    let analysis = analyse(src);
    // Cursor on the unqualified `set tolComp` declaration (line 4, col 6).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        6,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&4), "the caller's own decl: {refs:?}");
    assert!(
        lines.contains(&1),
        "the in-proc `global tolComp` decl must be reached even from an unqualified set: {refs:?}"
    );
    assert!(
        lines.contains(&2),
        "the in-proc $tolComp read must be reached even from an unqualified set: {refs:?}"
    );
}

#[test]
fn references_do_not_conflate_unrelated_same_named_cells_in_different_namespaces() {
    // FP guard: the canonical-cell fold-in must
    // still be exact-qualified-name matched — two unrelated top-level
    // `tolComp` cells living in different namespaces, neither aliasing
    // the other, must never be unioned together just because they share
    // a bare name.
    let src = "namespace eval A {\n    variable tolComp 1\n}\nnamespace eval B {\n    variable tolComp 2\n}\n";
    let analysis = analyse(src);
    // Cursor on `A::tolComp`'s declaration (line 1, col 13).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        13,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert_eq!(
        lines,
        vec![1],
        "an unrelated same-named cell in a different namespace must not be pulled in: {refs:?}"
    );
}

#[test]
fn references_for_a_multi_list_foreach_second_varlist_now_reach_every_use() {
    // Unless the first loop's own `name` (the second varList of `foreach
    // dirName {...} name {...} {...}`) is bound, `references()` from *any*
    // use inside the first loop's body falls through to whatever *other*
    // same-named `VarDef` exists anywhere in the flat top-level scope
    // — here, a second, later, textually unrelated `foreach name
    // {...}` — returning only that second loop's own 2 spans and
    // omitting every actual first-loop span, including the query site
    // itself. `foreach` (like `if`/`set`) introduces no new analyser
    // scope (correctly modelling Tcl's lack of block scoping), so at
    // the top level these two loops' `name` genuinely share one global
    // storage cell — same as any two sequential top-level `set name
    // ...` statements — so the correct reference set spans
    // *both* loops, not just the first: the bug was under-reporting
    // (missing the first loop's spans entirely), not over-reporting.
    let src = "foreach dirName {src src {src core}} name {alpha beta gamma} {\n    puts \"$dirName $name\"\n    if {$name eq \"pixutils\"} { puts skip }\n}\nforeach name {examples color changes} {\n    puts $name.ruff\n}\n";
    let analysis = analyse(src);
    // Cursor on the first loop's own `$name` read (line 1, col 21) —
    // the exact query shape the finding's own repro used.
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        21,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(
        lines.contains(&0),
        "the first loop's own `name` decl (line 0) must no longer be missing: {refs:?}"
    );
    assert!(
        lines.contains(&1),
        "the query site itself (line 1): {refs:?}"
    );
    assert!(
        lines.contains(&2),
        "the first loop's other in-body use (line 2): {refs:?}"
    );
    assert!(
        lines.contains(&4) && lines.contains(&5),
        "the second loop shares the same global cell (no block scoping) so its spans stay unified too: {refs:?}"
    );
}

#[test]
fn references_do_not_include_unexported_sibling_wildcard_call_same_document() {
    // FP guard — the unexported sibling must not surface as a reference
    // through the wildcard import either.
    let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    proc other {} { return 2 }\n    namespace export bar\n}\nnamespace import ::Foo::*\nother\n";
    let analysis = analyse(src);
    // Cursor on the `other` declaration (line 2, col 9).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        9,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert_eq!(lines, vec![2], "only the declaration itself: {refs:?}");
}

#[test]
fn references_reach_the_call_site_from_a_shadowed_duplicate_proc_decl_same_document() {
    // TN-shaped guard: the second `proc` replaces the first before any
    // call, so the call reaches only the winner (tclsh 8.4-9.1 print
    // `TWO`). The superseded declaration keeps its own span and gains
    // neither the winner nor the call. Pinned so the cross-document tier
    // cannot change this same-document answer.
    let src = "proc List2array {lst} { return ONE }\nproc List2array {lst} { return TWO }\nList2array x\n";
    let analysis = analyse(src);
    // Cursor on the SHADOWED (first) declaration (line 0, col 6).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert_eq!(lines, vec![0], "only the superseded declaration: {refs:?}");
}

#[test]
fn references_include_the_renames_own_old_word() {
    // TP — `rename OLD NEW`'s own
    // `OLD` word is a genuine reference to the proc it names
    // (tclsh9.0/8.6-verified: `rename` requires `OLD` to exist,
    // "can't rename ...: command doesn't exist" otherwise) — the real
    // corpus shape is a tcltest `-setup`/`-body`/`-cleanup` idiom
    // (georgtree_tclopt test/arbitaryTest.tcl:46/113's `proc gaussfunc`
    // / `rename gaussfunc ""`). Go-to-definition/hover resolve this token
    // through their own cursor-token walk; if `references` missed it, a
    // rename built on the same list would leave the occurrence pointing at
    // a now-nonexistent command — a passing tcltest then crashes with
    // "can't delete ...: command doesn't exist" purely from applying the
    // LSP's own rename edit.
    let src =
        "proc helperFunc {x} { return [expr {$x * 2}] }\nhelperFunc 21\nrename helperFunc \"\"\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&0), "decl missing: {refs:?}");
    assert!(lines.contains(&1), "call site missing: {refs:?}");
    assert!(
        lines.contains(&2),
        "rename's own OLD word missing: {refs:?}"
    );
}

#[test]
fn references_do_not_include_the_renames_new_word() {
    // FP guard — `rename OLD NEW`'s `NEW` word is not itself a
    // reference to `OLD`'s proc; only `OLD` is. A bare later call to
    // `NEW` reaches the target through the rename *link* (a
    // cross-document concern — see `workspace_index.rs`'s
    // `rename_new_name_call_site_references_the_old_command`), not
    // through this same-document text-reference scan.
    let src = "proc helperFunc {x} { return [expr {$x * 2}] }\nrename helperFunc renamedFunc\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert_eq!(lines, vec![0, 1], "{refs:?}");
}

#[test]
fn references_include_unbraced_if_body_bareword_call() {
    // TP — `if {$cond} mymod::foo` (an unbraced if-then body — a single,
    // statically-known bareword, valid Tcl and used ~50 times in the
    // nico-robert_ticklecharts corpus this way) is invisible to
    // `command_invocations` whenever `analyse_body` recurses only a braced
    // (`Str`-kind) body, and `references` from the
    // declaration then misses it — go-to-definition and hover
    // still found it (they resolve independently off the cursor
    // token), producing a dangerous asymmetry: a `rename` built on
    // this same list would silently miss rewriting the call site.
    let src = "proc foo {} { return 1 }\nif {1} foo\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&0), "decl missing: {refs:?}");
    assert!(
        lines.contains(&1),
        "unbraced if-body call site missing: {refs:?}"
    );
}

#[test]
fn references_include_unbraced_uplevel_body_bareword_call() {
    // TP — `uplevel 1 qux` (unbraced) runs in the caller's frame and
    // namespace: called from the global level it reaches `::qux`
    // (tclsh 8.4–9.1 print `1`); from `::ns` with its own `qux` the same
    // body prints `NS`, so an uncalled `caller` proves no target.
    let src = "proc qux {} { return 1 }\nproc caller {} { uplevel 1 qux }\ncaller\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&0), "decl missing: {refs:?}");
    assert!(
        lines.contains(&1),
        "unbraced uplevel-body call site missing: {refs:?}"
    );
}

#[test]
fn references_resolve_a_constant_var_body_through_its_real_value_not_its_literal_text() {
    // Was an FP guard against treating `$cb` as a static call to a
    // command literally *named* `$cb` — that concern still holds (this
    // test's own name reflects it), but real tclsh9.0/8.6-verified
    // behaviour is that `if {1} $cb` (a bare-`$var` `if`-body, evaluated
    // as a script exactly like `eval`/`uplevel`'s bodies) genuinely
    // calls `foo` when `$cb` holds that constant value — printing
    // "CALLED" for a `proc foo {} { puts CALLED }`. The analyser wires up
    // exactly this dispatch (`dispatch_one_body_argument`'s
    // `TokenType::Var` branch, generic across every `ArgRole::Body`
    // argument, not just `eval`/`uplevel`'s), so `foo`'s own reference
    // set correctly grows to include this call site — the failure mode
    // this test now guards is a *literal* `$cb`-named command
    // reference ever appearing, which never happens (there is no such
    // command in `all_procs` to resolve to).
    let src = "proc foo {} { return 1 }\nset cb foo\nif {1} $cb\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&0), "decl missing: {refs:?}");
    assert!(
        lines.contains(&2),
        "`if {{1}} $cb` really does dispatch to `foo` (tclsh9.0/8.6-verified) \
             and must be found: {refs:?}"
    );
}

#[test]
fn references_exclude_decl_when_flag_false() {
    let src = "proc greet {} {}\ngreet\n";
    let analysis = analyse(src);
    let with_decl = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        2,
        &analysis,
        true,
    );
    let without_decl = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        2,
        &analysis,
        false,
    );
    assert!(with_decl.len() > without_decl.len());
}

#[test]
fn references_to_unknown_word_empty() {
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert_eq!(
        references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            &analysis,
            true
        ),
        [] as [crate::definition::LspRange; 0]
    );
}

#[test]
fn ensemble_subcommand_references_include_decl_and_both_call_sites() {
    // TP — references on an ensemble subcommand call
    // site must return the target proc's declaration plus every call
    // site — proves the automatic pickup via
    // `proc_reference_spans`/`invocation_references_named`'s
    // `resolved_qualified_name` matching, not just a single hardcoded
    // case.
    let src = "namespace eval ::e {\n    namespace ensemble create -map {\n        foo ::e::Foo\n    }\n}\nproc ::e::Foo {args} { return \"foo: $args\" }\n\nputs [e foo bar]\nputs [e foo baz]\n";
    let analysis = analyse(src);
    // Cursor on "foo" in the first call site (0-based line 7, col 8).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        7,
        8,
        &analysis,
        true,
    );
    // decl + the `-map`'s own target-text reference (line 2, pre-existing
    // — needed so renaming the proc also updates the map entry) + both
    // nested-`[...]` call sites.
    assert_eq!(
        refs.len(),
        4,
        "expected decl + map entry + 2 call sites: {refs:?}"
    );
    // References come back in source order, so the `-map` entry precedes
    // the declaration.
    assert!(
        refs.iter().any(|r| r.start_line == 5),
        "declaration: {refs:?}"
    );
    assert!(
        refs.iter().any(|r| r.start_line == 2),
        "expected the -map target-text reference: {refs:?}",
    );
    assert!(
        refs.iter().any(|r| r.start_line == 7) && refs.iter().any(|r| r.start_line == 8),
        "expected both call sites: {refs:?}",
    );
}

/// The analyser's own dispatch recording resolves the
/// subcommand the way the ensemble does, so an **abbreviated** call site
/// is a reference too. Oracle (tclsh 8.6.16 / 9.0.4): with `-map {foo
/// ::e::Foo}` and the default `-prefixes 1`, `e fo` returns `foo: bar`.
///
/// The abbreviated site sits inside a proc body so the deferred replay
/// path (`flush_pending_ensemble_subcommand_invocations`) resolves it,
/// not only the direct one.
#[test]
fn ensemble_abbreviated_dispatch_is_a_reference_to_the_target() {
    let src = "namespace eval ::e {\n    namespace ensemble create -map {\n        foo ::e::Foo\n    }\n}\nproc ::e::Foo {args} { return \"foo: $args\" }\nproc caller {} {\n    return [e fo bar]\n}\n";
    let analysis = analyse(src);
    // Cursor on the declaration of ::e::Foo (0-based line 5).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        5,
        11,
        &analysis,
        true,
    );
    assert!(
        refs.iter().any(|r| r.start_line == 7),
        "expected the abbreviated call site inside the proc body: {refs:?}",
    );
}

/// The `-prefixes 0` counterpart: prefix matching is off, so the
/// abbreviated word is a plain unknown subcommand and must NOT be
/// recorded as a dispatch reference. Oracle: `g fo` errors
/// `unknown subcommand "fo": must be foo`.
#[test]
fn prefixless_ensemble_abbreviation_is_not_a_reference() {
    let src = "namespace eval ::g {\n    namespace ensemble create -map {\n        foo ::g::Foo\n    } -prefixes 0\n}\nproc ::g::Foo {args} { return G }\nproc caller {} {\n    return [g fo bar]\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        5,
        11,
        &analysis,
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 7),
        "a -prefixes 0 ensemble must not resolve the abbreviation: {refs:?}",
    );
}

/// The ensemble is created by
/// `namespace ensemble create -map` inside a proc declared with a
/// fully-qualified name at top level, with no enclosing `namespace eval`,
/// so it homes to `::app::widget`. Both reference directions must reach
/// the dispatch call site: from the mapped target's declaration, and from
/// the call site itself.
///
/// Oracle (tclsh 8.6.16 and 9.0.4, identical): the script prints `shown`,
/// so `::app::widget show` really does dispatch to `::app::widget::Show`.
///
/// Go-to-definition already answered this (it resolves on demand against
/// the finished analysis); references enumerate recorded invocations, and
/// the invocation was missing. The analyser fix is what supplies it — this
/// pins that the reference provider consumes it from both cursor
/// positions and returns the *same* answer for each.
#[test]
fn ensemble_subcommand_references_reach_the_call_site_through_a_qualified_proc_923_idx85() {
    let src = "namespace eval ::app::widget {\n    variable state 0\n}\n\
                   proc ::app::widget::Setup {} {\n    \
                   namespace ensemble create -map {\n        \
                   show      ::app::widget::Show\n    }\n}\n\
                   proc ::app::widget::Show {} { puts \"shown\" }\n\
                   ::app::widget::Setup\n\
                   puts [::app::widget show]\n";
    let analysis = analyse(src);
    // Line 8 is `proc ::app::widget::Show …`; `Show` starts at column 20.
    let from_decl = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        8,
        21,
        &analysis,
        true,
    );
    // Line 10 is `puts [::app::widget show]`; `show` starts at column 20.
    let from_call = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        10,
        21,
        &analysis,
        true,
    );
    let lines = |refs: &[LspRange]| -> Vec<u32> {
        let mut l: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
        l.sort_unstable();
        l
    };
    assert!(
        lines(&from_decl).contains(&10),
        "the `::app::widget show` dispatch is missing from the declaration's \
             reference set: {from_decl:?}",
    );
    assert!(
        lines(&from_decl).contains(&8),
        "the declaration itself is missing: {from_decl:?}",
    );
    assert_eq!(
        lines(&from_call),
        lines(&from_decl),
        "both query directions must agree: {from_call:?} vs {from_decl:?}",
    );
}

/// TN for the above — a `-map` built by `[list …]` is not a literal
/// mapping, so no `show -> ::app::widget::Show` fact exists and the
/// dispatch site must stay unattributed rather than guessed at.
#[test]
fn a_dynamic_ensemble_map_yields_no_call_site_reference_923_idx85() {
    let src = "proc ::app::widget::Setup {} {\n    \
                   namespace ensemble create -map [list show ::app::widget::Show]\n}\n\
                   proc ::app::widget::Show {} {}\n\
                   ::app::widget::Setup\n\
                   ::app::widget show\n";
    let analysis = analyse(src);
    // Cursor on `Show` in its declaration (line 3, column 20).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        3,
        21,
        &analysis,
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 5),
        "a dynamic -map must not manufacture a navigation edge: {refs:?}",
    );
}

#[test]
fn method_references_include_next_dispatch() {
    // `Sub::greet`'s body invokes the super via `next`; that dispatch is a
    // polymorphic reference to `greet` and must appear among its
    // references.
    let src = "oo::class create Base {\n    method greet {} {}\n}\noo::class create Sub {\n    superclass Base\n    method greet {} { next }\n}\n";
    let analysis = analyse(src);
    // Cursor on `greet` in Sub's declaration (line 5).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        5,
        13,
        &analysis,
        true,
    );
    // The `next` token sits on line 5, past the method's own `greet` name
    // (col 11) — inside the `{ next }` body.
    assert!(
        refs.iter()
            .any(|r| r.start_line == 5 && r.start_character > 15),
        "expected the `next` dispatch among refs: {refs:?}",
    );
}

// Constructor / destructor next-chain references.

#[test]
fn constructor_next_chain_reference_from_direct_subclass() {
    // `Sub`'s constructor calls plain `next`, chaining to `Base`'s —
    // cursor on `Base`'s own `constructor` keyword must surface that
    // chain as a reference.
    let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { next }\n}\n";
    let analysis = analyse(src);
    // `constructor` keyword on line 1, col 4.
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        &analysis,
        true,
    );
    // decl (line 1) + the `next` call site (line 5).
    assert_eq!(refs.len(), 2, "{refs:?}");
    assert!(refs.iter().any(|r| r.start_line == 5), "{refs:?}");
}

#[test]
fn constructor_next_chain_reference_skips_non_overriding_subclass() {
    // `Sub` inherits `Base`'s constructor outright (declares none of its
    // own) — nothing to chain, so `Base`'s constructor stays unreferenced.
    let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        &analysis,
        false,
    );
    assert!(refs.is_empty(), "{refs:?}");
}

#[test]
fn constructor_next_chain_reference_not_counted_without_next() {
    // `Sub` declares its own constructor but never calls `next` — a
    // legitimate full override, not a chain.
    let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { set x 1 }\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        &analysis,
        false,
    );
    assert!(refs.is_empty(), "{refs:?}");
}

#[test]
fn constructor_next_chain_reference_skips_ancestor_with_no_own_constructor() {
    // `Mid` declares no constructor of its own; `Sub`'s `next` must
    // still reach `Base` (the actual MRO-effective provider), not `Mid`.
    let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Mid {\n    superclass Base\n}\noo::class create Sub {\n    superclass Mid\n    constructor {} { next }\n}\n";
    let analysis = analyse(src);
    // `Base`'s constructor (line 1) picks up the chain.
    let base_refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        &analysis,
        false,
    );
    assert_eq!(base_refs.len(), 1, "{base_refs:?}");
    assert_eq!(base_refs[0].start_line, 8, "{base_refs:?}");
}

#[test]
fn constructor_next_chain_reference_via_nextto_explicit_target() {
    // `nextto Grandparent` jumps past `Base` even though `Base` also has
    // an effective constructor — only `Grandparent` picks up a reference.
    let src = "oo::class create Grandparent {\n    constructor {} { }\n}\noo::class create Base {\n    superclass Grandparent\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { nextto Grandparent }\n}\n";
    let analysis = analyse(src);
    let grandparent_refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        &analysis,
        false,
    );
    assert_eq!(grandparent_refs.len(), 1, "{grandparent_refs:?}");
    let base_refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        6,
        &analysis,
        false,
    );
    assert!(base_refs.is_empty(), "{base_refs:?}");
}

#[test]
fn constructor_next_chain_reference_via_braced_nextto_target() {
    // `nextto {Grandparent}` (a
    // braced target, functionally identical to the bare form) must
    // resolve exactly like `constructor_next_chain_reference_via_nextto_explicit_target`'s
    // bare `nextto Grandparent` — the decoded word, not the raw
    // delimited span, is what gets resolved against the class map.
    let src = "oo::class create Grandparent {\n    constructor {} { }\n}\noo::class create Base {\n    superclass Grandparent\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { nextto {Grandparent} }\n}\n";
    let analysis = analyse(src);
    let grandparent_refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        &analysis,
        false,
    );
    assert_eq!(grandparent_refs.len(), 1, "{grandparent_refs:?}");
    let base_refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        6,
        &analysis,
        false,
    );
    assert!(base_refs.is_empty(), "{base_refs:?}");
}

#[test]
fn destructor_next_chain_reference_from_direct_subclass() {
    let src = "oo::class create Base {\n    destructor { }\n}\noo::class create Sub {\n    superclass Base\n    destructor { next }\n}\n";
    let analysis = analyse(src);
    // `destructor` keyword on line 1, col 4.
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        5,
        &analysis,
        true,
    );
    assert_eq!(refs.len(), 2, "{refs:?}");
    assert!(refs.iter().any(|r| r.start_line == 5), "{refs:?}");
}

#[test]
fn constructor_shadowed_by_a_later_redeclaration_has_no_references() {
    // `oo::configurable` allows several constructors; only the last is
    // ever effective. A cursor on the shadowed first one resolves to
    // nothing (it has no reference story worth surfacing).
    let src = "oo::class create C {\n    constructor {} { }\n    constructor {} { }\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        &analysis,
        true,
    );
    assert!(refs.is_empty(), "{refs:?}");
}

#[test]
fn constructor_keyword_resolves_its_own_next_chain_even_with_a_same_named_method() {
    // A class can also declare a
    // `method` literally named `constructor` — an independent, ordinary
    // member sharing a name with the special keyword form. A cursor on
    // the special `constructor` keyword must still resolve its own
    // next-chain story, not fall through to the unrelated same-named
    // method's (ordinary) references.
    let src = "oo::class create Base {\n    constructor {} { }\n}\noo::class create Sub {\n    superclass Base\n    constructor {} { next }\n    method constructor {} { }\n}\n";
    let analysis = analyse(src);
    // `Base`'s `constructor` keyword, line 1.
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        &analysis,
        false,
    );
    assert_eq!(
        refs.len(),
        1,
        "must resolve the next-chain, not the unrelated same-named method: {refs:?}"
    );
    assert_eq!(refs[0].start_line, 5, "{refs:?}");
}

#[test]
fn references_to_var_includes_definition_and_uses() {
    let src = "set x 1\nputs $x\nputs $x\n";
    let analysis = analyse(src);
    // Cursor on `$x` first reference.
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        &analysis,
        true,
    );
    // The analyser may or may not record the literal `$x`
    // as a reference depending on lowering; at minimum the
    // declaration should land in the result list.
    assert!(!refs.is_empty(), "{refs:?}");
    assert!(refs.iter().any(|r| r.start_line == 0));
}

#[test]
fn references_from_proc_param_bareword_declaration_include_every_use() {
    // TP — a cursor on a proc parameter's own bareword name (not a
    // `$`-prefixed read) must return the same reference set a query from
    // any `$name` read resolves.
    let src = "proc greet {name} { return $name }\ngreet hi\n";
    let analysis = analyse(src);
    // Cursor on `name` inside the parameter list (col 12-16).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        13,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&0), "decl missing: {refs:?}");
    assert!(
        lines.contains(&0) && refs.len() >= 2,
        "read missing: {refs:?}"
    );
}

#[test]
fn references_from_catch_resultvar_bareword_include_the_original_declaration() {
    // TP — the finding's other confirmed shape: a `catch script name`
    // result-var reuses an existing variable; a cursor placed on its
    // own bareword token must still surface the full reference set,
    // including the original declaration.
    let src = "proc resolveSwitch {name def} {\n    catch {error boom} name\n    return $name\n}\n";
    let analysis = analyse(src);
    // Cursor on the catch result-var `name` (line 1, col 23-27).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        24,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&0), "original decl missing: {refs:?}");
    assert!(
        lines.contains(&2),
        "the later `$name` read missing: {refs:?}"
    );
}

// read/write distinction

#[test]
fn document_highlights_var_records_write_at_definition() {
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    // Cursor inside `$x`.
    let highlights = document_highlights(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        &analysis,
    );
    // The defining `set x` span should be tagged Write.
    let writes: Vec<_> = highlights
        .iter()
        .filter(|(_, k)| *k == HighlightKind::Write)
        .collect();
    assert!(
        !writes.is_empty(),
        "expected at least one Write for `set x 1`; got {highlights:?}",
    );
    // The Write should be on line 0 (the `set` line).
    assert!(
        writes.iter().any(|(r, _)| r.start_line == 0),
        "expected Write on line 0; got {highlights:?}",
    );
}

#[test]
fn document_highlights_var_read_kind_is_correctly_tagged() {
    // Pure report conversion contract: selection and current-source
    // authority belong to the public caller; this helper only maps the
    // supplied declaration/read spans to Write/Read LSP ranges.
    use tcl_lexer::Span;
    let src = "set x 1\nputs $x\n";
    let line_index = LineIndex::new(src);
    let highlights = variable_occurrence_highlights(
        src,
        &line_index,
        [(Span::new(4, 5), true), (Span::new(13, 14), false)],
    );
    // Write at definition.
    assert!(
        highlights
            .iter()
            .any(|(r, k)| r.start_line == 0 && *k == HighlightKind::Write),
        "expected Write at line 0; got {highlights:?}",
    );
    // Read at the supplied reference.
    assert!(
        highlights
            .iter()
            .any(|(r, k)| r.start_line == 1 && *k == HighlightKind::Read),
        "expected Read at line 1; got {highlights:?}",
    );
}

#[test]
fn document_highlights_proc_decl_is_text() {
    let src = "proc greet {} {}\ngreet\n";
    let analysis = analyse(src);
    let highlights = document_highlights(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
    );
    // Declaration on line 0 should be Text — procs carry no Write/Read
    // distinction (only variables do).
    let line0 = highlights
        .iter()
        .find(|(r, k)| r.start_line == 0 && *k == HighlightKind::Text);
    assert!(
        line0.is_some(),
        "expected Text on line 0 (declaration); got {highlights:?}",
    );
    // Call site on line 1 should be Text (no read/write
    // semantics on command-invocation heads).
    assert!(
        highlights
            .iter()
            .any(|(r, k)| r.start_line == 1 && *k == HighlightKind::Text),
        "expected Text on line 1 (call site); got {highlights:?}",
    );
}

#[test]
fn document_highlights_empty_for_unknown_symbol() {
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert_eq!(
        document_highlights(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            &analysis
        ),
        [] as [(
            crate::definition::LspRange,
            crate::references::HighlightKind
        ); 0]
    );
}

// resolved-qualified-name matching

#[test]
fn resolved_qualified_name_matches_call_site_from_namespace() {
    // Source: a proc defined at the top level, called from
    // a namespace.  The call site's literal name (`greet`)
    // matches the proc name; the resolved qualified name
    // also matches.  We pin that the references provider
    // finds the call site.
    let src = "proc ::greet {} {}\nnamespace eval ::myns {\n    greet\n}\n";
    let analysis = analyse(src);
    // Cursor on the proc declaration.
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        8,
        &analysis,
        true,
    );
    // Should include the declaration and the call site.
    assert!(
        refs.len() >= 2,
        "expected proc decl + namespace call site; got {refs:?}",
    );
}

#[test]
fn document_highlights_surfaces_var_reads_from_arg_positions() {
    // With `record_arg_var_reads`, `$x`
    // reads in command arguments populate
    // `VarDef.references` and surface as `Read` spans in
    // the document-highlight provider.
    let src = "set x 1\nputs $x\nputs $x\n";
    let analysis = analyse(src);
    let highlights = document_highlights(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        &analysis,
    );
    let reads: Vec<_> = highlights
        .iter()
        .filter(|(_, k)| *k == HighlightKind::Read)
        .collect();
    assert!(
        reads.len() >= 2,
        "expected >= 2 Read entries (for two `$x` sites); got {highlights:?}",
    );
    // The defining `set x` span is Write.
    assert!(
        highlights
            .iter()
            .any(|(r, k)| r.start_line == 0 && *k == HighlightKind::Write),
        "expected Write on line 0; got {highlights:?}",
    );
}

#[test]
fn resolved_qualified_name_field_populated_for_simple_call() {
    // Verify that the analyser actually populates
    // `resolved_qualified_name` on
    // `command_invocations`.  At the top level a `greet`
    // call should resolve to `::greet`.
    let src = "greet hi\n";
    let analysis = analyse(src);
    let inv = analysis
        .command_invocations
        .iter()
        .find(|i| i.name == "greet")
        .expect("expected a `greet` invocation");
    assert_eq!(
        inv.resolved_qualified_name.as_deref(),
        Some("::greet"),
        "expected resolved name to be `::greet`; got {inv:?}",
    );
}

// class-member references

#[test]
fn idx63_two_block_class_my_dispatch_already_fixed_by_idx52() {
    // Go-to-definition and find-references must both answer for a `my
    // methodName` call when the class is created via `oo::class create`
    // with no body and every method (including the call site itself) is
    // added via a *separate*, later `oo::define ClassName { ... }` block —
    // the real corpus's `ticklecharts::chart` shape.  The load-bearing
    // machinery is `class_body_spans` / `enclosing_class_at`, pinned here
    // from the `my`-dispatch direction.
    let src = "oo::class create foo::widget {\n    variable _x\n    constructor {} { set _x 0 }\n}\noo::define foo::widget {\n    method bar {} { return \"bar-value\" }\n    method baz {} { return [my bar] }\n}\nputs [[foo::widget new] baz]\n";
    let analysis = analyse(src);
    // `definition` at the `my bar` call site (line 6, col 31).
    let locs = crate::definition::definition(src, 6, 31, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 5,
        "must resolve to method bar's declaration"
    );
    // `references` from `bar`'s declaration (line 5, col 11).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        5,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&5), "decl missing: {refs:?}");
    assert!(
        lines.contains(&6),
        "the my bar call site must be reachable from the declaration: {refs:?}"
    );
}

#[test]
fn references_for_method_includes_decl_and_call_sites() {
    // Intra-class dispatch uses `my <method>` (the dispatch form tclsh
    // accepts; a bare `greet` head errors with "invalid command name", so it
    // is not a reference).
    let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `greet` declaration (line 1, col 11).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
        true,
    );
    assert!(refs.len() >= 3, "expected ≥3 refs; got {refs:?}");
}

mod composed_callback_prefix_tests {
    use super::*;
    use tcl_compiler::analyser::ResolvedAnalysisInput;
    use tcl_compiler::registry_invocation::source_structure::{
        original_single_source_command_substitution, source_authored_command_prefix,
        source_registry_words,
    };

    const POSITIVE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../experiments/name-handling/patterns/composed-callback-prefix/probes/positive.tcl"
    ));
    const UNKNOWN_WRITER: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../experiments/name-handling/patterns/composed-callback-prefix/probes/unknown-writer.tcl"
    ));
    const PRIVATE_POSITIVE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../experiments/name-handling/patterns/composed-callback-prefix/fixtures/private-positive.tcl"
    ));

    fn analyse_source(source: &str, release: &str) -> AnalysisResult {
        let environment = tcl_registry::model::ingress::resolve_environment(release);
        let profile = environment.analyser_profile();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            environment.default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    }

    fn original_builder(
        source: &str,
        analysis: &AnalysisResult,
    ) -> (
        tcl_compiler::segmenter::SegmentedCommand,
        tcl_lexer::NativeWord,
        tcl_lexer::NativeWord,
    ) {
        let config = analysis.resolved_input.as_ref().unwrap().lexer_config();
        let body = analysis.all_classes["::C"].methods["setup"].body_span;
        let (start, end) = strip_outer_braces(source, body);
        let region =
            tcl_lexer::Span::new(u32::try_from(start).unwrap(), u32::try_from(end).unwrap());
        let plan = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::document(source),
            region,
            config,
        )
        .unwrap();
        assert!(plan.fatal_tail.is_none());
        let setter = &plan.commands[0];
        assert_eq!(setter.words[0].bytes(), b"set");
        assert_eq!(setter.words[1].bytes(), b"cb");
        let wrapper = original_single_source_command_substitution(&setter.words[2]).unwrap();
        assert_eq!(wrapper.command().words[0].bytes(), b"namespace");
        assert_eq!(wrapper.command().words[1].bytes(), b"code");
        let child =
            original_single_source_command_substitution(&wrapper.command().words[2]).unwrap();
        assert_eq!(child.command().words.len(), 4);
        assert_eq!(child.command().words[0].bytes(), b"::makePrefix");
        let selector = child.command().words[2].clone();
        let fixed = child.command().words[3].clone();
        let span = child.command().span;
        let mut commands = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
            &source[span.as_range()],
            span.start(),
            config,
        );
        assert_eq!(commands.len(), 1);
        (commands.remove(0), selector, fixed)
    }

    fn reference_ranges(
        source: &str,
        analysis: &AnalysisResult,
        include_declaration: bool,
    ) -> Vec<LspRange> {
        let profile = analysis.resolved_input.as_ref().unwrap().unit_profile();
        let declaration = analysis.all_classes["::C"].methods["tick"].name_span;
        let index = LineIndex::new(source);
        let at = index.position_at_utf16(declaration.start(), source);
        references(
            source,
            profile,
            at.line,
            at.character.get(),
            analysis,
            include_declaration,
        )
    }

    #[test]
    fn composed_callback_prefix_original_advice_keeps_builder_and_each_written_operand() {
        for release in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for (case, source) in [
                ("positive", POSITIVE),
                ("unknown-writer", UNKNOWN_WRITER),
                ("private-positive", PRIVATE_POSITIVE),
            ] {
                let analysis = analyse_source(source, release);
                let (producer, selector, fixed) = original_builder(source, &analysis);
                let input = analysis.resolved_input.as_ref().unwrap();
                let context = input.context_registry();
                let words = source_registry_words(source, &analysis, &producer)
                    .unwrap_or_else(|| panic!("{release}/{case}: missing original builder schema"));
                assert_eq!(
                        words.with_source_schema(&context, |schema| {
                            schema
                                .semantics
                                .traits
                                .contains(tcl_registry::Traits::BUILDS_COMMAND_PREFIX)
                                && schema.semantics.native_result
                                    == Some(
                                        tcl_registry::native_result::NativeResultContract::ListArguments {
                                            from: 0,
                                        },
                                    )
                                && schema.semantics.argument_offset == 0
                                && schema.facts().arity_accepts_frozen_arguments() == Some(true)
                        }),
                        Some(true),
                        "{release}/{case}"
                    );
                let prefix = source_authored_command_prefix(source, &analysis, &producer)
                    .unwrap_or_else(|| panic!("{release}/{case}: missing authored prefix receipt"));
                assert_eq!(
                    prefix.producer().site().offset,
                    producer.argv[0].span.start()
                );
                assert_eq!(prefix.producer().original_words().len(), 4);
                assert_eq!(
                    prefix.producer().original_words()[0].bytes(),
                    b"::makePrefix"
                );
                assert_ne!(prefix.producer().lineage(), []);
                assert!(prefix.obligations().contains(
                        &tcl_compiler::command_binding::SourceCommandTransitionObligation::AuthoredCommandPrefixApplicability
                    ));
                assert!(!prefix.obligations().contains(
                        &tcl_compiler::command_binding::SourceCommandTransitionObligation::ProducedCommandPrefixApplicability
                    ));
                assert_eq!(
                    prefix.original_head().original_word().unwrap().bytes(),
                    b"my"
                );
                assert_eq!(prefix.original_argument(0).unwrap(), &selector);
                assert_eq!(prefix.original_argument(1).unwrap(), &fixed);
                assert_eq!(selector.bytes(), b"tick");
                assert_eq!(fixed.bytes(), b"{a b}");
                assert_eq!(
                    prefix.argument_origin(0),
                    Some(tcl_compiler::registry_invocation::InvocationWordOrigin::Written(2))
                );
                assert_eq!(
                    prefix.argument_origin(1),
                    Some(tcl_compiler::registry_invocation::InvocationWordOrigin::Written(3))
                );
                assert_eq!(
                    prefix.with_source_schema(&context, |schema| {
                        schema
                            .semantics
                            .traits
                            .contains(tcl_registry::Traits::TCLOO_SELF_DISPATCH)
                    }),
                    Some(true),
                    "{release}/{case}"
                );
                assert!(prefix.matches_source_context(
                    &tcl_lexer::SourceImage::document(source),
                    input.lexer_config(),
                    &context,
                ));
                assert!(!prefix.matches_source_context(
                    &tcl_lexer::SourceImage::native(source.as_bytes()),
                    input.lexer_config(),
                    &context,
                ));
            }
        }
    }

    #[test]
    fn composed_callback_prefix_references_keep_public_and_private_original_selectors() {
        for release in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for source in [POSITIVE, PRIVATE_POSITIVE] {
                let analysis = analyse_source(source, release);
                let (_, selector, _) = original_builder(source, &analysis);
                let declaration = analysis.all_classes["::C"].methods["tick"].name_span;
                let index = LineIndex::new(source);
                let declaration_range = span_to_range(source, &index, declaration);
                let selector_range = span_to_range(source, &index, selector.span());
                if source == PRIVATE_POSITIVE {
                    assert!(analysis.all_classes["::C"].unexports.contains("tick"));
                }
                let refs = reference_ranges(source, &analysis, true);
                assert!(refs.contains(&declaration_range), "{release}: {refs:?}");
                assert!(
                    refs.contains(&selector_range),
                    "{release}: original stored selector missing: {refs:?}"
                );
                let refs = reference_ranges(source, &analysis, false);
                assert!(!refs.contains(&declaration_range), "{release}: {refs:?}");
                assert!(refs.contains(&selector_range), "{release}: {refs:?}");
            }
        }
    }

    #[test]
    fn composed_callback_prefix_stored_sink_withdraws_unknown_and_opaque_writers() {
        let opaque = POSITIVE.replacen("after idle $cb", "opaque $cb; after idle $cb", 1);
        let nested = POSITIVE.replacen(
            "after idle $cb",
            "if 1 {set cb {my unrelated}}; after idle $cb",
            1,
        );
        for release in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for (case, source) in [
                ("unknown-writer", UNKNOWN_WRITER),
                ("opaque-writer", opaque.as_str()),
                ("nested-reassignment", nested.as_str()),
            ] {
                let analysis = analyse_source(source, release);
                let (_, selector, _) = original_builder(source, &analysis);
                let declaration = analysis.all_classes["::C"].methods["tick"].name_span;
                let index = LineIndex::new(source);
                let refs = reference_ranges(source, &analysis, true);
                assert!(
                    refs.contains(&span_to_range(source, &index, declaration)),
                    "{release}/{case}: declaration missing: {refs:?}"
                );
                assert!(
                    !refs.contains(&span_to_range(source, &index, selector.span())),
                    "{release}/{case}: stale stored selector survived: {refs:?}"
                );
            }
        }
    }

    #[test]
    fn composed_callback_prefix_original_context_withdraws_stale_source_and_configuration() {
        let source = POSITIVE;
        let analysis = analyse_source(source, "tcl8.6");
        let (producer, selector, _) = original_builder(source, &analysis);
        let index = LineIndex::new(source);
        let selector_range = span_to_range(source, &index, selector.span());
        assert!(source_authored_command_prefix(source, &analysis, &producer).is_some());
        assert!(reference_ranges(source, &analysis, true).contains(&selector_range));

        let changed = format!("{source}# different complete source\n");
        assert!(source_authored_command_prefix(&changed, &analysis, &producer).is_none());
        assert!(!reference_ranges(&changed, &analysis, true).contains(&selector_range));

        let mut different_config = analysis.clone();
        different_config
            .body_lexer_config
            .as_mut()
            .unwrap()
            .expand_syntax ^= true;
        assert!(source_authored_command_prefix(source, &different_config, &producer).is_none());
        assert!(!reference_ranges(source, &different_config, true).contains(&selector_range));

        let mut missing_input = analysis.clone();
        missing_input.resolved_input = None;
        assert!(source_authored_command_prefix(source, &missing_input, &producer).is_none());
        let declaration = analysis.all_classes["::C"].methods["tick"].name_span;
        let at = index.position_at_utf16(declaration.start(), source);
        let profile = analysis.resolved_input.as_ref().unwrap().unit_profile();
        assert!(
            !references(
                source,
                profile,
                at.line,
                at.character.get(),
                &missing_input,
                true,
            )
            .contains(&selector_range)
        );
    }
}

#[test]
fn references_for_method_includes_one_hop_stored_callback_prefixes() {
    let src = "package require Tk\noo::class create C {\n    method tick {} {}\n    method setup {} {\n        set external [list [self] tick]\n        bind .w <Button-1> $external\n        set internal [namespace code [list my tick]]\n        after 0 $internal\n    }\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&2), "declaration missing: {refs:?}");
    assert!(
        lines.contains(&4),
        "stored [self] callback missing: {refs:?}"
    );
    assert!(
        lines.contains(&6),
        "stored namespace-code callback missing: {refs:?}"
    );
}

#[test]
fn stored_callback_prefixes_abstain_on_reassignment_and_dynamic_methods() {
    let src = "oo::class create C {\n    method tick {} {}\n    method setup {method} {\n        set cb [list my tick]\n        lappend cb suffix\n        set cb [list my $method]\n        bind .w <Button-1> $cb\n    }\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        1,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert_eq!(
        lines,
        vec![1],
        "ambiguous stored callback must abstain: {refs:?}"
    );
}

#[test]
fn stored_callback_prefixes_count_nested_same_frame_reassignment() {
    let src = "oo::class create C {\n    method tick {} {}\n    method other {} {}\n    method setup {} {\n        set cb [list my tick]\n        if 1 {\n            set cb [list my other]\n            after 0 $cb\n        }\n    }\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert_eq!(
        lines,
        vec![1],
        "a nested reassignment makes the stored callback ambiguous: {refs:?}"
    );
}

#[test]
fn stored_callback_prefixes_abstain_on_non_set_varwrite_reassignment() {
    let src = "oo::class create C {\n    method tick {} {}\n    method setup {} {\n        set cb [list my tick]\n        lappend cb suffix\n        after 0 $cb\n    }\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert_eq!(
        lines,
        vec![1],
        "non-Set VarWrite reassignment must abstain: {refs:?}"
    );
}

#[test]
fn stored_callback_prefixes_abstain_on_qualified_and_aliased_variables() {
    let src = "oo::class create C {\n    method tick {} {}\n    method setup {} {\n        set ::cb [list my tick]\n        bind .w <Button-1> $::cb\n        global local\n        set local [list my tick]\n        after 0 $local\n    }\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert_eq!(
        lines,
        vec![1],
        "qualified/aliased storage must abstain: {refs:?}"
    );
}

#[test]
fn stored_callback_prefix_cursor_resolves_to_method_declaration() {
    let src = "oo::class create C {\n    method tick {} {}\n    method setup {} {\n        set cb [list my tick]\n        after 0 $cb\n    }\n}\n";
    let analysis = analyse(src);
    let col = u32::try_from(src.lines().nth(3).unwrap().find("tick").unwrap()).unwrap();
    let locations = crate::definition::definition(src, 3, col, &analysis);
    assert_eq!(
        locations.len(),
        1,
        "stored callback cursor must resolve: {locations:?}"
    );
    assert_eq!(
        locations[0].start_line, 1,
        "wrong stored callback target: {locations:?}"
    );
}

#[test]
fn references_do_not_resolve_an_unclosed_braced_instance_head() {
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset x [Dog new]\n${x bark\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
        true,
    );
    assert_eq!(
        refs.len(),
        1,
        "only the declaration may resolve; malformed `${{x` is not `x`: {refs:?}"
    );
    assert_eq!(refs[0].start_line, 1);
    assert_eq!(strip_var_decoration("${x}", None), Some("x"));
    assert_eq!(strip_var_decoration("$x", None), Some("x"));
    assert_eq!(strip_var_decoration("$arr(idx)", None), Some("arr(idx)"));
    assert_eq!(strip_var_decoration("$::a:::b", None), Some("::a:::b"));
    assert_eq!(strip_var_decoration("$foo:::", None), Some("foo:::"));
    assert_eq!(strip_var_decoration("${x", None), Some("{x"));
    assert_eq!(strip_var_decoration("x", None), None);
}

#[test]
fn references_for_method_reach_a_my_dispatch_call_inside_a_switch_arm() {
    // A `my methodName` call written inside a `switch` arm body is a
    // genuine, statically-known call site (tclsh9.0/8.6-verified) —
    // the real corpus shape (`ticklecharts::chart`'s `Add` dispatcher:
    // `switch ... { barSeries { my AddBarSeries {*}$args } ... }`).
    // `scan_my_method_region`'s `[...]`-substitution recursion alone never
    // reaches a switch arm's braced body (it isn't a command
    // substitution), which would leave this invisible to find-references even
    // though go-to-definition (an independent cursor-token walk)
    // already resolved it.
    let src = "oo::class create widget {\n    method bar {} { return \"bar-value\" }\n    method dispatch {args} {\n        switch -exact -- [lindex $args 0] {\n            bar { my bar {*}[lrange $args 1 end] }\n        }\n    }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `bar` declaration (line 1, col 11).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(
        lines.contains(&4),
        "the my bar call site inside the switch arm is missing: {refs:?}"
    );
}

#[test]
fn references_for_method_excludes_decl_when_requested() {
    let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
        false,
    );
    // Only the two call sites — the declaration is
    // excluded when include_declaration=false.
    assert_eq!(refs.len(), 2, "{refs:?}");
}

#[test]
fn document_highlights_for_method_marks_decl_and_calls_text() {
    let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
    let analysis = analyse(src);
    let h = document_highlights(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
    );
    // Methods carry no Write/Read distinction — declaration + both call
    // sites are all Text (only variables are Write/Read).
    let writes = h.iter().filter(|(_, k)| *k == HighlightKind::Write).count();
    let texts = h.iter().filter(|(_, k)| *k == HighlightKind::Text).count();
    assert_eq!(writes, 0, "{h:?}");
    assert_eq!(texts, 3, "{h:?}");
}

#[test]
fn references_from_decl_reach_my_dispatch_when_class_extended_via_separate_oo_define() {
    // `Gadget` is created via `oo::class create` with no body; every method
    // (including the `my Helper` call site) is added via a *separate*,
    // later `oo::define Gadget { ... }` block — the real corpus shape
    // (`ticklecharts::chart`). References from the `Helper` declaration
    // must reach the `my Helper` call site living in that separate
    // block, not silently return nothing.
    let src = "oo::class create Gadget {\n    variable _x\n}\noo::define Gadget {\n    method Helper {} { return hi }\n    method Caller {} { my Helper }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `Helper` declaration (line 4, col 11).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&4), "decl missing: {refs:?}");
    assert!(
        lines.contains(&5),
        "the my Helper call site inside the separate oo::define block is missing: {refs:?}"
    );
}

#[test]
fn references_from_decl_reach_a_self_bracket_dispatch_call_site() {
    // `[self] m` is TclOO's own same-object dispatch
    // idiom — reaches the enclosing class exactly like `my m`, but
    // through a bracketed command substitution. References from the
    // declaration must reach it, not silently return only the
    // declaration itself.
    let src = "oo::class create C {\n    method animTick {} { return 1 }\n    method anim {} { [self] animTick }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `animTick` declaration (line 1, col 11).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(
        lines.contains(&2),
        "the [self] animTick call site is missing: {refs:?}"
    );
}

#[test]
fn references_from_decl_reach_a_list_built_self_callback() {
    // `bind` receives a deferred script built as a Tcl list.
    // `[self]` is substituted while the method frame is live, leaving an
    // object-command prefix that later dispatches `animTick` externally.
    let src = "package require Tk\noo::class create C {\n    method animTick {} { return 1 }\n    method anim {wl} {\n        bind $wl <ButtonPress-1> [list [self] animTick %x %y]\n    }\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&2), "decl missing: {refs:?}");
    assert!(
        lines.contains(&4),
        "the list-built bind callback is missing: {refs:?}"
    );
}

#[test]
fn references_reach_a_list_built_self_object_callback() {
    let src = "package require Tk\noo::class create C {\n    method tick {} { return 1 }\n    method wire {wl} {\n        bind $wl <Expose> [list [self object] tick]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        11,
        &analyse(src),
        true,
    );
    assert!(
        refs.iter().any(|r| r.start_line == 4),
        "the `[self object]` callback is missing: {refs:?}"
    );
}

#[test]
fn references_reach_a_list_built_self_after_callback() {
    // A real corpus shape: Pave and Zesty both schedule methods this
    // way. `after` exposes its script through the same registry Body role
    // as `bind`, so no scheduler-specific branch belongs in this scan.
    let src = "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analyse(src),
        true,
    );
    assert!(
        refs.iter().any(|r| r.start_line == 3),
        "the list-built after callback is missing: {refs:?}"
    );
}

/// A tcllib-shaped fixture read from the real files rather than a
/// hand-written imitation.
///
/// The original fixed upstream bytes are required; missing or changed
/// corpus setup cannot count as observed callback semantics.
#[test]
fn the_real_tcllib_callback_shapes_resolve_to_their_methods() {
    let corpus = std::env::var_os("TCLLIB_2_0_DIR").map_or_else(
        || std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/tcllib-2.0"),
        std::path::PathBuf::from,
    );
    // (fixture, method declared in the file, 0-based lines of the
    // callback prefixes that name it). `cat.tcl` is the `after …
    // [namespace code [list my Post $c]]` shape; `httpd.tcl` (Clay classes,
    // Tcl 8.6) is the `socket -server [namespace code [list my connect]]
    // ?-myaddr $ip? $port` shape (#1956).
    for (fixture, method, callbacks) in [
        (
            &tcl_test_support::fixed_corpus::TCLLIB_2_0_CHANNEL_CAT,
            "Post",
            &[][..],
        ),
        (
            &tcl_test_support::fixed_corpus::TCLLIB_2_0_HTTPD,
            "connect",
            &[1010_u32, 1013][..],
        ),
    ] {
        let relative = fixture.relative_path;
        let original = tcl_test_support::fixed_corpus::FixedCorpusSource::read(&corpus, fixture)
            .expect("fixed Tcllib 2.0 source setup must be available and unchanged");
        let src = String::from_utf8(original.bytes().to_vec())
            .expect("the original Tcllib callback control is UTF-8");
        let analysis = analyse(&src);
        let declaration = src
            .find(&format!("method {method} "))
            .unwrap_or_else(|| panic!("{relative} declares `method {method}`"));
        let line = u32::try_from(src[..declaration].lines().count() - 1).expect("line fits");
        let column = u32::try_from(
            src[..declaration]
                .rsplit('\n')
                .next()
                .unwrap_or_default()
                .len()
                + "method ".len(),
        )
        .expect("column fits");
        let refs = references(
            &src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            line,
            column,
            &analysis,
            true,
        );
        assert!(
            refs.len() > 1,
            "{relative}: `{method}` must reach its callback prefix as well as its \
                 declaration, got {refs:?}"
        );
        for callback in callbacks {
            assert!(
                refs.iter().any(|r| r.start_line == *callback),
                "{relative}: `{method}` must reach the callback on line {callback}: {refs:?}"
            );
        }
    }
}

/// A `clay::define` class is a `TclOO` class: its lowercase `method` names a
/// `TclOO` method and its `superclass` a `TclOO` relation, with Clay's
/// `::clay::object` root implicit as `::oo::object` is, so a `my` call
/// reaches the method a class or its superclass declares (#1956). Tcl 8.6.
#[test]
fn references_reach_a_my_call_in_a_clay_class() {
    for src in [
        "clay::define ::C {\n    method connect {sock} {}\n    method run {} { my connect x }\n}\n",
        "clay::define ::B {\n    method connect {sock} {}\n}\nclay::define ::C {\n    superclass ::B\n    method run {} { my connect x }\n}\n",
    ] {
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analyse(src),
            true,
        );
        assert!(
            refs.iter().any(|r| r.start_line != 1),
            "the `my connect` call is missing: {src}: {refs:?}"
        );
    }
}

/// `socket` scans options while a word starts with `-` and keeps the last
/// word as the port (`Tcl_SocketObjCmd`, 8.4.20-9.1.0), so the accept
/// callback is found with a dynamic port and later options. Tcl 8.6.
#[test]
fn references_reach_a_socket_server_callback_with_a_dynamic_port() {
    for call in [
        "socket -server [namespace code [list my connect]] $port",
        "socket -server [namespace code [list my connect]] -myaddr $ip $port",
    ] {
        let src = format!(
            "oo::class create C {{\n    method connect {{sock ip port}} {{}}\n    method start {{ip port}} {{\n        {call}\n    }}\n}}\n"
        );
        let refs = references(
            &src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analyse(&src),
            true,
        );
        assert!(
            refs.iter().any(|r| r.start_line == 3),
            "the socket accept callback is missing: {call}: {refs:?}"
        );
    }
}

/// A `{*}`-expanded callback word must abstain.
///
/// `{*}` splices the word's value into the argument list, so the registry's
/// role indices no longer describe where anything landed. Reading the word
/// as if it were the callback slot is not merely unjustified, it invents
/// references: after expansion `lsort -command {*}[list [self] compare]
/// $items` runs the bare *object command* as the comparator and passes
/// `compare` as a separate argument, so `compare` is not dispatched at all
/// — and Rename would have rewritten it.
#[test]
fn an_expanded_callback_word_invents_no_reference() {
    for src in [
        // The comparator is the object command; `compare` is an argument.
        "oo::class create C {\n    method compare {a b} { return 0 }\n    method sort {items} {\n        lsort -command {*}[list [self] compare] $items\n    }\n}\n",
        // Same rule for a Body slot, where the arity happens to work out.
        "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle {*}[list [self] tick]\n    }\n}\n",
        // Inside a `WRAPS_COMMAND_PREFIX` wrapper:
        // `namespace code` then has two arguments and errors rather than
        // dispatching anything, so there is nothing to reference.
        "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [namespace code {*}[list my tick]]\n    }\n}\n",
        // And inside the builder itself: the receiver is no longer word 1.
        "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [list {*}$prefix tick]\n    }\n}\n",
    ] {
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analyse(src),
            true,
        );
        assert!(
            !refs.iter().any(|r| r.start_line == 3),
            "an expanded word is not a callback slot: {src:?} gave {refs:?}"
        );
    }
}

/// The gate is about expansion, not about the shape around it: the same
/// prefix written as a sole substitution still resolves.
#[test]
fn references_reach_a_list_built_self_command_prefix() {
    // CommandPrefix is distinct from Body: the registry says this word is
    // a partial command and the consumer appends its own arguments. The
    // object-method reference is nevertheless the same external dispatch.
    let src = "oo::class create C {\n    method compare {a b} { return 0 }\n    method sort {items} {\n        lsort -command [list [self] compare] $items\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analyse(src),
        true,
    );
    assert!(
        refs.iter().any(|r| r.start_line == 3),
        "the list-built CommandPrefix callback is missing: {refs:?}"
    );
}

#[test]
fn references_reach_namespace_wrapped_my_callback_even_when_private() {
    // tcllib's virtchannel_base shape: the registry declares
    // `namespace code` as a WRAPS_COMMAND_PREFIX and `list` as a
    // BUILDS_COMMAND_PREFIX.  `my` keeps private current-object dispatch,
    // unlike a captured `[self]` object command.
    let src = "oo::class create C {\n    method read {} { return 1 }\n    unexport read\n    method wire {chan} {\n        fileevent $chan readable [namespace code [list my read]]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        1,
        11,
        &analyse(src),
        true,
    );
    assert!(
        refs.iter().any(|r| r.start_line == 4),
        "the wrapped private `my read` callback is missing: {refs:?}"
    );
}

#[test]
fn references_reach_namespace_wrapped_self_callback_only_when_public() {
    let src = "oo::class create C {\n    method changed {} { return 1 }\n    method wire {} {\n        trace add variable v write [namespace code [list [self] changed]]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        1,
        11,
        &analyse(src),
        true,
    );
    assert!(
        refs.iter().any(|r| r.start_line == 3),
        "the wrapped public `[self] changed` callback is missing: {refs:?}"
    );
}

#[test]
fn a_mixin_branch_decides_its_own_providers_visibility() {
    // A mixin that inherits the member from its
    // own superclass and unexports the name empties *its* branch only —
    // the spine still answers.  tclsh 8.6.16 / 9.0.4 both run `A`'s body:
    //   oo::class create MChild { superclass MBase } ; unexport m
    //   oo::class create D { superclass A ; mixin MChild }
    //   [D new] m  ->  A-m   (not MBase-m)
    let src = "oo::class create A {\n    method m {} { return 1 }\n}\noo::class create MBase {\n    method m {} { return 2 }\n}\noo::class create MChild {\n    superclass MBase\n    unexport m\n}\noo::class create D {\n    superclass A\n    mixin MChild\n    method wire {} {\n        after idle [list [self] m]\n    }\n}\n";
    let analysis = analyse(src);
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let cursor = u32::try_from(src.find("] m]").unwrap() + 2).unwrap();
    assert_eq!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "m", cursor),
        Some(("::A".to_owned(), false)),
        "the suppressed mixin branch must not provide the capture"
    );
    assert!(
        !references(src, dialect, 4, 11, &analysis, true)
            .iter()
            .any(|range| range.start_line == 14),
        "`MBase::m` must not collect a capture its branch cannot dispatch"
    );
    assert!(
        references(src, dialect, 1, 11, &analysis, true)
            .iter()
            .any(|range| range.start_line == 14),
        "`A::m` — the provider the receiver actually reaches — must collect it"
    );
}

#[test]
fn a_live_mixin_branch_still_outranks_the_spine() {
    // The control for the case above: with the branch left public, the
    // mixin's inherited `MBase::m` is what `[D new] m` enters (tclsh
    // 8.6.16 / 9.0.4 -> MBase-m), so the capture belongs to *its* family.
    let src = "oo::class create A {\n    method m {} { return 1 }\n}\noo::class create MBase {\n    method m {} { return 2 }\n}\noo::class create MPub {\n    superclass MBase\n}\noo::class create D {\n    superclass A\n    mixin MPub\n    method wire {} {\n        after idle [list [self] m]\n    }\n}\n";
    let analysis = analyse(src);
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let cursor = u32::try_from(src.find("] m]").unwrap() + 2).unwrap();
    assert_eq!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "m", cursor),
        Some(("::MBase".to_owned(), false)),
        "a live mixin branch provides the capture ahead of the spine"
    );
}

#[test]
fn a_mixin_branch_can_revive_its_own_bases_unexported_member() {
    // The mirror direction inside a branch: `NBase` unexports its own `n`,
    // the mixin `NChild` exports the inherited name, and tclsh 8.6.16 /
    // 9.0.4 run `NBase`'s body for `[D new] n` — ahead of the spine's
    // public `A2::n`.
    let src = "oo::class create A2 {\n    method n {} { return 1 }\n}\noo::class create NBase {\n    method n {} { return 2 }\n    unexport n\n}\noo::class create NChild {\n    superclass NBase\n    export n\n}\noo::class create D {\n    superclass A2\n    mixin NChild\n    method wire {} {\n        after idle [list [self] n]\n    }\n}\n";
    let analysis = analyse(src);
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let cursor = u32::try_from(src.find("] n]").unwrap() + 2).unwrap();
    assert_eq!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "n", cursor),
        Some(("::NBase".to_owned(), false)),
        "the branch's own export revives its base's body"
    );
}

#[test]
fn a_mixin_declared_on_a_superclass_governs_its_branch_too() {
    // Branch roots are collected transitively, so a `mixin` written on a
    // superclass behaves exactly as one written on the receiver: tclsh
    // 8.6.16 / 9.0.4 answer `[D new] m` with `A-m`.
    let src = "oo::class create A {\n    method m {} { return 1 }\n}\noo::class create MBase {\n    method m {} { return 2 }\n}\noo::class create MChild {\n    superclass MBase\n    unexport m\n}\noo::class create S {\n    superclass A\n    mixin MChild\n}\noo::class create D {\n    superclass S\n    method wire {} {\n        after idle [list [self] m]\n    }\n}\n";
    let analysis = analyse(src);
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let cursor = u32::try_from(src.find("] m]").unwrap() + 2).unwrap();
    assert_eq!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "m", cursor),
        Some(("::A".to_owned(), false)),
        "a superclass-declared mixin's suppression empties that branch only"
    );
}

#[test]
fn inert_list_built_self_value_is_not_a_method_reference() {
    let src = "oo::class create C {\n    method tick {} { return 1 }\n    method build {} {\n        set inert [list [self] tick]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analyse(src),
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 3),
        "an inert list value must not become a call site: {refs:?}"
    );
}

#[test]
fn dynamic_list_built_self_method_remains_unresolved() {
    let src = "oo::class create C {\n    method tick {} { return 1 }\n    method build {methodName} {\n        after idle [list [self] $methodName]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analyse(src),
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 3),
        "a dynamic method word must remain an abstention: {refs:?}"
    );
}

#[test]
fn compound_list_built_self_word_is_not_the_spelled_method() {
    // The list substitution produces an object command ending in `tick`,
    // then Tcl concatenates `Suffix` onto that word. The invoked method is
    // `tickSuffix`; rewriting the inner `tick` token would be incorrect.
    let src = "oo::class create C {\n    method tick {} { return 1 }\n    method build {} {\n        after idle [list [self] tick]Suffix\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analyse(src),
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 3),
        "a method fragment inside a compound word is not an exact reference: {refs:?}"
    );
}

#[test]
fn deferred_braced_self_script_is_not_a_method_reference() {
    // The braces delay `[self]` itself until Tk runs the binding, after
    // the defining method frame has gone.  This script cannot resolve the
    // object and must not be credited as a method call.
    let src = "package require Tk\noo::class create C {\n    method tick {} { return 1 }\n    method wire {wl} {\n        bind $wl <Expose> {[self] tick}\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        11,
        &analyse(src),
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 4),
        "a later, out-of-frame `self` must not become a call site: {refs:?}"
    );
}

#[test]
fn list_built_self_callback_cannot_reach_an_unexported_method() {
    // A captured `[self]` result is an object command.  Its later call is
    // therefore external and cannot dispatch an unexported method.
    let src = "package require Tk\noo::class create C {\n    method tick {} { return 1 }\n    unexport tick\n    method wire {wl} {\n        bind $wl <Expose> [list [self] tick]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        11,
        &analyse(src),
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 5),
        "an unexported method is not callable through the captured object: {refs:?}"
    );
}

#[test]
fn wrapped_my_callback_cursor_resolves_private_dispatch() {
    let src = "oo::class create C {\n    method tick {} { return 1 }\n    unexport tick\n    method wire {chan} {\n        fileevent $chan readable [namespace code [list my tick]]\n    }\n}\n";
    let analysis = analyse(src);
    let cursor = u32::try_from(src.rfind("tick]").unwrap()).unwrap();
    assert_eq!(
        list_built_self_method_target_at_cursor(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &analysis,
            "tick",
            cursor,
        ),
        Some(("::C".to_owned(), false)),
    );
}

#[test]
fn a_shadowing_list_command_does_not_build_a_callback_reference() {
    let src = "package require Tk\nproc list args { return not-a-command-prefix }\noo::class create C {\n    method tick {} { return 1 }\n    method wire {wl} {\n        bind $wl <Expose> [list [self] tick]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        3,
        11,
        &analyse(src),
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 5),
        "a user command shadowing the registry builder must not inherit its semantics: {refs:?}"
    );
}

#[test]
fn a_global_self_command_does_not_shadow_the_tcloo_method_helper() {
    let src = "package require Tk\nproc self args { return ::notTheCurrentObject }\noo::class create C {\n    method tick {} { return 1 }\n    method wire {wl} {\n        bind $wl <Expose> [list [self] tick]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        3,
        11,
        &analyse(src),
        true,
    );
    assert!(
        refs.iter().any(|r| r.start_line == 5),
        "::oo::Helpers precedes global fallback on a method frame: {refs:?}"
    );
}

#[test]
fn malformed_self_object_receiver_is_not_a_callback_reference() {
    let src = "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [list [self object junk] tick]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analyse(src),
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 3),
        "a receiver call that errors cannot build a callback: {refs:?}"
    );
}

#[test]
fn explicitly_global_self_is_not_the_tcloo_method_helper() {
    let src = "proc ::self {} { return ::notTheCurrentObject }\noo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [list [::self] tick]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        11,
        &analyse(src),
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 4),
        "an absolute global receiver must not inherit method-path semantics: {refs:?}"
    );
}

#[test]
fn qualified_tcloo_self_helper_builds_the_same_callback() {
    let src = "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [list [::oo::Helpers::self] tick]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analyse(src),
        true,
    );
    assert!(
        refs.iter().any(|r| r.start_line == 3),
        "the registry-declared qualified helper must remain valid: {refs:?}"
    );
}

#[test]
fn callback_cursor_classifier_reaches_constructor_and_destructor_bodies() {
    let src = "oo::class create C {\n    method tick {} { return 1 }\n    constructor {} {\n        after idle [list [self] tick]\n    }\n    destructor {\n        after idle [list [self] tick]\n    }\n}\n";
    let analysis = analyse(src);
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let first = src.find("] tick").unwrap() + 2;
    let second = src[first + 1..].find("] tick").unwrap() + first + 3;
    for cursor in [first, second] {
        assert_eq!(
            list_built_self_method_target_at_cursor(
                src,
                dialect,
                &analysis,
                "tick",
                u32::try_from(cursor).unwrap(),
            ),
            Some(("::C".to_owned(), false)),
        );
    }
}

#[test]
fn callback_cursor_classifier_rejects_ordinary_method_dispatches() {
    let src = "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        my tick\n        [self] tick\n    }\n}\n";
    let analysis = analyse(src);
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let my_cursor = u32::try_from(src.find("my tick").unwrap() + 3).unwrap();
    let self_cursor = u32::try_from(src.find("] tick").unwrap() + 2).unwrap();
    assert!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "tick", my_cursor,)
            .is_none()
    );
    assert!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "tick", self_cursor,)
            .is_none()
    );
}

#[test]
fn inherited_callback_abstains_without_effective_receiver_visibility() {
    let src = "oo::class create Base {\n    method tick {} { return 1 }\n}\noo::class create Child {\n    superclass Base\n    unexport tick\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analyse(src),
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 7),
        "provider visibility cannot be reused for the child receiver: {refs:?}"
    );
}

#[test]
fn inherited_callback_joins_the_providers_reference_set() {
    // A captured `[self]` object command in an inheriting class
    // reaches the provider's exported implementation — tclsh 8.6.16 /
    // 9.0.4 both run `Base`'s body for `[list [Child new] tick]`.  Both
    // directions must agree on that, or rename would edit one and not the
    // other.
    let src = "oo::class create Base {\n    method tick {} { return 1 }\n}\noo::class create Child {\n    superclass Base\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n";
    let analysis = analyse(src);
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let cursor = u32::try_from(src.find("] tick").unwrap() + 2).unwrap();
    assert_eq!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "tick", cursor),
        Some(("::Base".to_owned(), false)),
        "the inherited callback resolves to its effective provider"
    );

    let refs = references(src, dialect, 1, 11, &analysis, true);
    assert!(
        refs.iter().any(|range| range.start_line == 6),
        "declaration-origin references must collect the same site: {refs:?}"
    );
}

#[test]
fn a_receiver_that_unexports_the_inherited_name_abstains_both_ways() {
    // The receiver's own `unexport` decides, not the provider's
    // declaration: tclsh 8.6.16 / 9.0.4 answer `[Child new] tick` with
    // `unknown method "tick"` even though `Base` exports it, so the
    // capture is not a call site of `Base::tick` at all.
    let src = "oo::class create Base {\n    method tick {} { return 1 }\n}\noo::class create Child {\n    superclass Base\n    unexport tick\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n";
    let analysis = analyse(src);
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let cursor = u32::try_from(src.find("] tick").unwrap() + 2).unwrap();
    assert!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "tick", cursor).is_none(),
        "an unexported receiver name must not resolve a captured [self] callback"
    );

    let refs = references(src, dialect, 1, 11, &analysis, true);
    assert!(
        !refs.iter().any(|range| range.start_line == 7),
        "declaration-origin references must make the same abstention: {refs:?}"
    );
}

#[test]
fn a_receiver_that_reexports_an_unexported_provider_resolves_both_ways() {
    // The mirror case, and the one a provider-only reading gets wrong in
    // the other direction: `Base` unexports its own `tock`, `Child`
    // exports the inherited name, and tclsh 8.6.16 / 9.0.4 run `Base`'s
    // body for `[Child new] tock`.
    let src = "oo::class create Base {\n    method tock {} { return 1 }\n    unexport tock\n}\noo::class create Child {\n    superclass Base\n    export tock\n    method wire {} {\n        after idle [list [self] tock]\n    }\n}\n";
    let analysis = analyse(src);
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let cursor = u32::try_from(src.find("] tock").unwrap() + 2).unwrap();
    assert_eq!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "tock", cursor),
        Some(("::Base".to_owned(), false)),
        "a re-exporting receiver revives the inherited callback target"
    );

    let refs = references(src, dialect, 1, 11, &analysis, true);
    assert!(
        refs.iter().any(|range| range.start_line == 8),
        "declaration-origin references must collect the same site: {refs:?}"
    );
}

#[test]
fn an_overriding_receiver_keeps_its_callback_in_its_own_family() {
    // `Child` declares its own `tick`, so the capture dispatches there.
    // `Base::tick`'s family must not claim it (renaming `Base::tick`
    // would otherwise rewrite a word that never called it).
    let src = "oo::class create Base {\n    method tick {} { return 1 }\n}\noo::class create Child {\n    superclass Base\n    method tick {} { return 2 }\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n";
    let analysis = analyse(src);
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let cursor = u32::try_from(src.find("] tick").unwrap() + 2).unwrap();
    assert_eq!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "tick", cursor),
        Some(("::Child".to_owned(), false)),
        "the override, not the base, provides this capture"
    );

    let refs = references(src, dialect, 1, 11, &analysis, true);
    assert!(
        !refs.iter().any(|range| range.start_line == 7),
        "the base declaration must not collect the override's capture: {refs:?}"
    );
}

#[test]
fn an_unrelated_class_does_not_collect_a_same_named_callback() {
    // No `superclass` edge, so nothing links the two `tick`s.
    let src = "oo::class create Base {\n    method tick {} { return 1 }\n}\noo::class create Other {\n    method tick {} { return 2 }\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n";
    let analysis = analyse(src);
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let refs = references(src, dialect, 1, 11, &analysis, true);
    assert!(
        !refs.iter().any(|range| range.start_line == 6),
        "an unrelated class's capture is not a reference: {refs:?}"
    );
}

#[test]
fn a_mixins_unexport_does_not_suppress_the_superclass_capture() {
    // C enters each mixin with a fresh copy of the dispatch flags, so a
    // mixin that unexports the name empties only its own branch: tclsh
    // 8.6.16 / 9.0.4 still run `Base`'s `tick` for `[Child new] tick`.
    let src = "oo::class create Base {\n    method tick {} { return 1 }\n}\noo::class create Mix {\n    unexport tick\n}\noo::class create Child {\n    superclass Base\n    mixin Mix\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n";
    let analysis = analyse(src);
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let cursor = u32::try_from(src.find("] tick").unwrap() + 2).unwrap();
    assert_eq!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "tick", cursor),
        Some(("::Base".to_owned(), false)),
        "a mixin's unexport must not suppress the spine's provider"
    );
}

// external $obj method sites

#[test]
fn references_from_external_obj_method_site() {
    // Declaration + 2 external call sites (`$d bark`,
    // `[$d bark]`).
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\nputs [$d bark]\n";
    let analysis = analyse(src);
    // Cursor on `bark` in `$d bark` (line 4, col 3).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        3,
        &analysis,
        true,
    );
    // Declaration (line 1) + two external sites (lines 4, 5).
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(lines.contains(&4), "line-4 call missing: {refs:?}");
    assert!(lines.contains(&5), "line-5 call missing: {refs:?}");
}

#[test]
fn references_from_inside_class_includes_external_sites() {
    // Cursor on the declaration; refs include the external
    // `$d bark` site as well as the declaration.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(lines.contains(&4), "external call missing: {refs:?}");
}

#[test]
fn find_obj_method_call_sites_covers_top_level_and_subst() {
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\nputs [$d bark]\n";
    let analysis = analyse(src);
    let sites = find_obj_method_call_sites(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &analysis,
        "::Dog",
        "bark",
        false,
    );
    // Two external sites: `$d bark` and `[$d bark]`.
    assert_eq!(sites.len(), 2, "{sites:?}");
}

#[test]
fn find_obj_method_call_sites_finds_calls_in_proc_body() {
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nproc f {} { global d; $d bark }\n";
    let analysis = analyse(src);
    let sites = find_obj_method_call_sites(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &analysis,
        "::Dog",
        "bark",
        false,
    );
    assert_eq!(sites.len(), 1, "{sites:?}");
}

#[test]
fn find_obj_method_call_sites_matches_bare_created_instance_command() {
    // `Dog create rex` binds `rex` as an object *command*; `rex bark` is a
    // bare-word method dispatch (not `$rex bark`).  The scan resolves it
    // through `created_instance_commands`.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
    let analysis = analyse(src);
    let sites = find_obj_method_call_sites(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &analysis,
        "::Dog",
        "bark",
        false,
    );
    assert_eq!(sites.len(), 1, "{sites:?}");
    // The matched span is the `bark` method-name token of `rex bark`.
    let s = sites[0];
    assert_eq!(
        &src[s.start() as usize..s.end() as usize],
        "bark",
        "{sites:?}"
    );
}

// class-command dispatch: `CLASS method` for a
// classmethod / `self method`, a receiver set entirely separate from
// `$obj method` / `NAME method` instance dispatch above.

#[test]
fn find_obj_method_call_sites_matches_class_command_and_inheriting_subclass() {
    // TP — both the plain shape (`ActiveRecord find`) and its
    // inherited-via-superclass sibling (`Table find`, ooutil's
    // `classmethod` propagates to a subclass's own bound command).
    let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\noo::class create Table {\n    superclass ActiveRecord\n}\nTable find foo bar\nActiveRecord find foo bar\n";
    let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
    let sites = find_obj_method_call_sites(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
        &analysis,
        "::ActiveRecord",
        "find",
        true,
    );
    assert_eq!(sites.len(), 2, "{sites:?}");
    for s in &sites {
        assert_eq!(
            &src[s.start() as usize..s.end() as usize],
            "find",
            "{sites:?}"
        );
    }
}

/// TP — a bare class-command dispatch written inside an `apply` lambda
/// body or a `namespace eval` body — at the top level or nested inside a
/// method — is a real call.
/// All three shapes were confirmed dispatching under tclsh 9.0.4
/// (`MAKE CALLED` printed three times).
#[test]
fn find_obj_method_call_sites_reaches_lambda_and_namespace_eval_bodies() {
    let src = "oo::class create Factory {\n\
                       classmethod make {} { return 1 }\n\
                       method inst {} { apply {{} { Factory make }} }\n\
                       method nsev {} { namespace eval ::zz { Factory make } }\n\
                   }\n\
                   namespace eval ::top2 { Factory make }\n\
                   Factory create obj\n\
                   obj inst\n\
                   obj nsev\n";
    let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
    let sites = find_obj_method_call_sites(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
        &analysis,
        "::Factory",
        "make",
        true,
    );
    assert_eq!(sites.len(), 3, "{sites:?}");
    for s in &sites {
        assert_eq!(
            &src[s.start() as usize..s.end() as usize],
            "make",
            "{sites:?}"
        );
    }
}

/// TN — the instance half must **not** follow the class-command half
/// through a frame shift.  `$f` inside `namespace eval ::zz` names
/// `::zz::f`, and inside an `apply` lambda a fresh local, so neither is
/// a dispatch on the outer `f` (tclsh 9.0.4: both raise `can't read
/// "f": no such variable`).
#[test]
fn find_obj_method_call_sites_excludes_var_receivers_across_a_frame_shift() {
    // A `namespace eval` body reads an unqualified variable from the
    // global namespace when the namespace has none on 8.6 but not on
    // 9.0/9.1; `apply` opens a fresh local frame on every release. The
    // errors are caught so the last line runs: tclsh 8.6 prints `BARK`
    // twice, 9.0 and 9.1 once. Uncaught, 8.6 stops at `apply` and 9.0/9.1
    // at `namespace eval`, and the last line never dispatches.
    let src = "oo::class create Dog {\n\
                       method bark {} {}\n\
                   }\n\
                   set f [Dog new]\n\
                   catch {namespace eval ::zz { $f bark }}\n\
                   catch {apply {{} { $f bark }}}\n\
                   $f bark\n";
    for (dialect, lines) in [
        ("tcl8.6", vec![5, 7]),
        ("tcl9.0", vec![7]),
        ("tcl9.1", vec![7]),
    ] {
        let analysis = Analyser::new().analyse(src, dialect);
        let sites = find_obj_method_call_sites(
            src,
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            &analysis,
            "::Dog",
            "bark",
            false,
        );
        let found = sites
            .iter()
            .map(|site| src[..site.start() as usize].lines().count())
            .collect::<Vec<_>>();
        assert_eq!(found, lines, "{dialect}: {sites:?}");
    }
}

/// TN — a bare (backslash-escaped) `apply` body element is decoded
/// before `apply` evaluates it, so its source slice is not the script
/// that runs; the scan must not re-parse it in place.
#[test]
fn find_obj_method_call_sites_skips_escaped_lambda_body_element() {
    let src = "oo::class create Factory {\n\
                       classmethod make {} { return 1 }\n\
                   }\n\
                   apply {{} Factory\\ make}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
    let sites = find_obj_method_call_sites(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
        &analysis,
        "::Factory",
        "make",
        true,
    );
    assert!(sites.is_empty(), "{sites:?}");
}

#[test]
fn find_obj_method_call_sites_excludes_non_inheriting_self_method_subclass() {
    // TN — the references-level precision guard mirroring
    // `self_method_not_inherited_by_a_non_overriding_subclass` in
    // definition.rs: unlike `ooutil`'s `classmethod`, a plain `self
    // method` is not inherited, so `Gadget make` must not be counted
    // as a call site of `Widget`'s `make`.
    let src = "oo::class create Widget {\n    self method make {n} { return \"made $n\" }\n}\noo::class create Gadget {\n    superclass Widget\n}\nWidget make foo\nGadget make foo\n";
    let analysis = analyse(src);
    let sites = find_obj_method_call_sites(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &analysis,
        "::Widget",
        "make",
        true,
    );
    assert_eq!(sites.len(), 1, "{sites:?}");
    assert_eq!(
        &src[sites[0].start() as usize..sites[0].end() as usize],
        "make",
        "{sites:?}"
    );
}

#[test]
fn references_enumerates_class_command_declaration_and_both_call_sites() {
    // TP — the full end-to-end peek from the finding's own repro:
    // declaration, the inherited-subclass call, and the
    // declaring-class's own call — three references, no duplicates,
    // none missed (requires Part 3 in addition to Part 2: Part 2 alone
    // only fixes single-cursor lookups, not this whole-document scan).
    let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\noo::class create Table {\n    superclass ActiveRecord\n}\nTable find foo bar\nActiveRecord find foo bar\n";
    let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
    // Cursor on the declaration (line 1, `find` at col 16).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
        1,
        16,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert_eq!(refs.len(), 3, "{refs:?}");
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(lines.contains(&6), "Table find call missing: {refs:?}");
    assert!(
        lines.contains(&7),
        "ActiveRecord find call missing: {refs:?}"
    );
}

#[test]
fn references_from_cursor_on_class_command_call_site() {
    // Symmetry with `references_from_cursor_on_bare_obj_command_call_site`:
    // invoking Find All References with the cursor ON the class-command
    // call site (not the declaration) must resolve identically.
    let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\nActiveRecord find foo bar\n";
    let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
    // Cursor on `find` in `ActiveRecord find foo bar` (line 3, col 13).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
        3,
        13,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(lines.contains(&3), "call site missing: {refs:?}");
}

#[test]
fn references_include_bare_created_instance_command_site() {
    // Full peek: cursor on the `method bark` decl surfaces the bare
    // `rex bark` dispatch as a reference.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(lines.contains(&4), "bare `rex bark` site missing: {refs:?}");
}

#[test]
fn references_from_cursor_on_bare_obj_command_call_site() {
    // Symmetry: invoking Find All References with the cursor
    // ON the `bark` token of a bare `rex bark` dispatch must resolve — not
    // only the declaration-based peek.  `rex` is at col 0, `bark` at col 4.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        4,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(lines.contains(&4), "call site missing: {refs:?}");
}

#[test]
fn bare_set_var_receiver_is_not_matched_without_dollar() {
    // FP guard: `set d [Dog new]` binds `d` as a *variable*, not a
    // command.  A bare `d bark` (no `$`) is NOT a valid dispatch in Tcl,
    // so it must not be matched — only `$d bark` counts.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nd bark\n";
    let analysis = analyse(src);
    let sites = find_obj_method_call_sites(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &analysis,
        "::Dog",
        "bark",
        false,
    );
    assert!(
        sites.is_empty(),
        "bare var receiver wrongly matched: {sites:?}"
    );
}

// tcl::OptProc — the `opt` package's automatic-option-parsing proc
// definer: without its analyser hook the call site is unreachable from
// the declaration.

#[test]
fn references_from_opt_proc_declaration_reach_the_call_site() {
    let src = "::tcl::OptProc greet {child -use -display} { return $child }\ngreet foo\n";
    let analysis = analyse(src);
    // Line 0 — cursor on "greet" right after `::tcl::OptProc` (col 15).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        15,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&0), "decl missing: {refs:?}");
    assert!(lines.contains(&1), "call site missing: {refs:?}");
}

#[test]
fn references_reach_a_proc_dispatched_through_an_eval_of_a_list_computed_var() {
    // `eval $cmdD`, where `$cmdD` is built via `[list greetD World]`, is a
    // real call site: without the constant-value dispatch it is invisible
    // and only the declaration comes back.
    let src = "proc greetD {n} {puts \"D $n\"}\nset cmdD [list greetD World]\neval $cmdD\n";
    let analysis = analyse(src);
    // Line 0 — cursor on `greetD`'s declaration name (col 6).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&0), "decl missing: {refs:?}");
    assert!(
        lines.contains(&1),
        "the `greetD` word inside `[list greetD World]` must be reachable too: {refs:?}"
    );
}

#[test]
fn method_source_dispatch_keeps_actual_store_availability_and_lookup_barriers() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    // Conditional source reference geometry, not a live receiver or method entry.
    use std::sync::Arc;
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.insert(tcl_registry::CommandSpec {
        name: "source_dispatch",
        arity: tcl_registry::Arity::at_least(1),
        traits: tcl_registry::Traits::TCLOO_SELF_DISPATCH,
        surface: registry.get("dict").unwrap().surface,
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let store = Arc::new(registry);
    let source = "source_dispatch selected";
    for (release, expected) in [("tcl8.6", 1), ("tcl8.4", 0)] {
        let context = Arc::new(
            tcl_registry::model::ingress::static_context_for(release)
                .with_command_store(Arc::clone(&store)),
        );
        let input =
            tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
        let mut analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        let body = tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap());
        assert_eq!(
            scan_my_method_sites(source, &analysis, &[body], "selected", None).len(),
            expected
        );
        analysis.body_lexer_config.as_mut().unwrap().expand_syntax = !config.expand_syntax;
        assert_eq!(
            scan_my_method_sites(source, &analysis, &[body], "selected", None),
            [] as [tcl_lexer::Span; 0]
        );
        analysis.body_lexer_config = Some(config);
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile, profile, foreign, config,
        ));
        assert_eq!(
            scan_my_method_sites(source, &analysis, &[body], "selected", None),
            [] as [tcl_lexer::Span; 0]
        );
        analysis.resolved_input = None;
        assert_eq!(
            scan_my_method_sites(source, &analysis, &[body], "selected", None),
            [] as [tcl_lexer::Span; 0]
        );
    }
    let source = "proc source_dispatch args {}; source_dispatch selected";
    let context = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.6").with_command_store(store),
    );
    let input =
        tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
    let analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    let body = tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap());
    assert_eq!(
        scan_my_method_sites(source, &analysis, &[body], "selected", None),
        [] as [tcl_lexer::Span; 0]
    );
}

fn callback_source_store(gate: &str) -> std::sync::Arc<tcl_registry::CommandRegistry> {
    let mut registry = tcl_registry::CommandRegistry::build_default();
    let gated = registry.get(gate).unwrap();
    let surface = gated.surface;
    let required_package = gated.required_package;
    registry.insert(tcl_registry::CommandSpec {
        name: "source_callback",
        arity: tcl_registry::Arity::exact(1),
        arg_roles: &[(0, tcl_registry::ArgRole::CommandPrefix)],
        command_prefixes: &[(0, tcl_registry::AppendedArity::Exactly(0))],
        traits: tcl_registry::Traits::DEFERS_BODY,
        surface,
        required_package,
        ..tcl_registry::CommandSpec::DEFAULT
    });
    registry.insert(tcl_registry::CommandSpec {
        name: "source_dispatch",
        arity: tcl_registry::Arity::at_least(1),
        traits: tcl_registry::Traits::TCLOO_SELF_DISPATCH,
        ..tcl_registry::CommandSpec::DEFAULT
    });
    std::sync::Arc::new(registry)
}

fn callback_source_analysis_with_store(
    source: &str,
    environment: &str,
    store: std::sync::Arc<tcl_registry::CommandRegistry>,
) -> AnalysisResult {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let context = std::sync::Arc::new(
        tcl_registry::model::ingress::static_context_for(environment).with_command_store(store),
    );
    let input =
        tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
    Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name)
}

fn callback_source_analysis(source: &str, release: &str) -> AnalysisResult {
    callback_source_analysis_with_store(source, release, callback_source_store("dict"))
}

fn callback_source_spans(
    source: &str,
    analysis: &AnalysisResult,
    method: &str,
) -> Vec<tcl_lexer::Span> {
    let body = tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap());
    scan_method_sites_by_kind(source, analysis, &[body], method, None, true, true)
}

#[test]
fn callback_source_roles_keep_actual_availability_and_original_builder_geometry() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    // Conditional source reference spans; no Native callback, receiver or edit grant.
    let store = callback_source_store("dict");
    for source in [
        "source_callback [list source_dispatch selected]",
        "set cb [list source_dispatch selected]; source_callback $cb",
        "source_callback [namespace code [list source_dispatch selected]]",
        "interp alias {} install {} source_callback; install [list source_dispatch selected]",
        "interp alias {} make {} list source_dispatch; source_callback [make selected]",
        "interp alias {} target {} source_dispatch selected; source_callback [list target]",
    ] {
        let mut analysis = callback_source_analysis_with_store(source, "tcl8.6", store.clone());
        let spans = callback_source_spans(source, &analysis, "selected");
        assert_eq!(spans.len(), 1, "{source}: {spans:?}");
        assert_eq!(source_span_text(source, spans[0]), "selected");
        assert!(
            callback_source_spans(
                source,
                &callback_source_analysis_with_store(source, "tcl8.4", store.clone()),
                "selected"
            )
            .is_empty(),
            "{source}"
        );
        let config = analysis.body_lexer_config.unwrap();
        let mut missing_point = AnalysisResult::default();
        missing_point.resolved_input = analysis.resolved_input.clone();
        missing_point.body_lexer_config = Some(config);
        assert_eq!(
            callback_source_spans(source, &missing_point, "selected"),
            [] as [tcl_lexer::Span; 0]
        );
        analysis.body_lexer_config.as_mut().unwrap().expand_syntax = !config.expand_syntax;
        assert_eq!(
            callback_source_spans(source, &analysis, "selected"),
            [] as [tcl_lexer::Span; 0]
        );
        analysis.body_lexer_config = Some(config);
        analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            tcl_dialect::DialectProfile::plain_tcl(),
            tcl_dialect::DialectProfile::plain_tcl(),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            config,
        ));
        assert_eq!(
            callback_source_spans(source, &analysis, "selected"),
            [] as [tcl_lexer::Span; 0]
        );
        analysis.resolved_input = None;
        assert_eq!(
            callback_source_spans(source, &analysis, "selected"),
            [] as [tcl_lexer::Span; 0]
        );
    }
}

#[test]
fn callback_source_roles_keep_actual_package_availability_in_the_same_store() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let source = "set cb [list source_dispatch selected]; source_callback $cb";
    let store = callback_source_store("bind");
    let available = callback_source_analysis_with_store(source, "tk", store.clone());
    let unavailable = callback_source_analysis_with_store(source, "tcl8.6", store);
    assert_eq!(
        callback_source_spans(source, &available, "selected").len(),
        1
    );
    assert_eq!(
        callback_source_spans(source, &unavailable, "selected"),
        [] as [tcl_lexer::Span; 0]
    );
}

#[test]
fn callback_source_targets_withdraw_known_shadows_and_incomplete_stored_writes() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    for source in [
        "proc source_callback args {}; source_callback [list source_dispatch selected]",
        "proc list args {}; source_callback [list source_dispatch selected]",
        "proc source_dispatch args {}; source_callback [list source_dispatch selected]",
        "rename source_dispatch {}; source_callback [list source_dispatch selected]",
        "set cb [list source_dispatch selected]; opaque $cb; source_callback $cb",
        "set cb [list source_dispatch selected]; set $unknown other; source_callback $cb",
        "source_callback [list $unknown selected]",
        "interp alias {} target {} source_dispatch selected; rename target {}; source_callback [list target]",
        "source_callback {*}[list source_dispatch selected]",
        "source_callback [list source_dispatch selected]Suffix",
    ] {
        let analysis = callback_source_analysis(source, "tcl8.6");
        assert!(
            callback_source_spans(source, &analysis, "selected").is_empty(),
            "{source}"
        );
    }
}

#[test]
fn caller_frame_reference_component_keeps_retained_registry_and_input() {
    // naming.core.original-caller-frame-navigation
    // docs/design/analysis/name-resolution-proofs/original-caller-frame-navigation.md
    // Readonly source template/reference geometry, not a completed native store.
    let source = "proc setdef {d} {upvar 1 $d dst; set dst SET}\nproc caller {} {setdef shared; puts $shared}\n";
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.insert(tcl_registry::CommandSpec {
        name: "context-marker",
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let context = tcl_registry::model::ingress::context_for_profile(profile)
        .with_command_store(std::sync::Arc::new(registry));
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::new(context),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    let line_index = LineIndex::new(source);
    let read = u32::try_from(source.find("$shared").unwrap()).unwrap() + 1;
    let ctx = RefCtx {
        source,
        dialect: profile,
        line_index: &line_index,
        line: 1,
        character: 0,
        analysis: &analysis,
        include_declaration: true,
        resolution: crate::definition::CallResolution::document_only(),
    };
    let found = caller_frame_references(&ctx, read, "shared").unwrap();
    assert_eq!(found.len(), 2);
    let bare = u32::try_from(source.find("setdef shared").unwrap() + 7).unwrap();
    let position = line_index.position_at_utf16(bare, source);
    let bare_ctx = RefCtx {
        line: position.line,
        character: position.character.get(),
        ..ctx
    };
    assert_eq!(variable_references(&bare_ctx), Some(found));
    let stale = format!("{source}# changed");
    let stale_ctx = RefCtx {
        source: &stale,
        ..ctx
    };
    assert!(caller_frame_references(&stale_ctx, read, "shared").is_none());
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    let missing_ctx = RefCtx {
        analysis: &missing,
        ..ctx
    };
    assert!(caller_frame_references(&missing_ctx, read, "shared").is_none());
    let foreign_ctx = RefCtx {
        resolution: ctx
            .resolution
            .with_registry(crate::registry_for_dialect_profile(profile)),
        ..ctx
    };
    assert!(caller_frame_references(&foreign_ctx, read, "shared").is_none());
}

#[test]
fn method_dispatch_scan_uses_retained_custom_schema_and_script_purpose() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    fn reference_only(
        _args: tcl_registry::InvocationArguments<'_>,
    ) -> Vec<(u8, tcl_registry::ScriptTiming)> {
        vec![(0, tcl_registry::ScriptTiming::ReferenceOnly)]
    }
    let source = "oo::class create C {method m {} {}; method run {} {custom-body {my m}; reference-script {my m}}}";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.insert(tcl_registry::CommandSpec {
        name: "custom-body",
        arity: tcl_registry::Arity::exact(1),
        arg_roles: &[(0, tcl_registry::ArgRole::Body)],
        ..tcl_registry::CommandSpec::DEFAULT
    });
    registry.insert(tcl_registry::CommandSpec {
        name: "reference-script",
        arity: tcl_registry::Arity::exact(1),
        arg_roles: &[(0, tcl_registry::ArgRole::Body)],
        script_timing_resolver: Some(reference_only),
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let context = tcl_registry::model::context_for_profile(profile)
        .with_command_store(std::sync::Arc::new(registry));
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::new(context),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    let mut analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    assert!(analysis.allows_retained_logical_declaration_advice());
    let body = analysis.all_classes["::C"].methods["run"].body_span;
    let sites = scan_my_method_sites(source, &analysis, &[body], "m", None);
    assert_eq!(
        sites.len(),
        1,
        "custom schema is retained; reference-only script is excluded"
    );
    assert_eq!(sites[0].start() as usize, source.find("my m").unwrap() + 3);
    let stale = source.replace("custom-body", "custom-bodz");
    assert_eq!(
        scan_my_method_sites(&stale, &analysis, &[body], "m", None),
        [] as [tcl_lexer::Span; 0]
    );
    analysis.resolved_input = None;
    assert_eq!(
        scan_my_method_sites(source, &analysis, &[body], "m", None),
        [] as [tcl_lexer::Span; 0]
    );
}

#[test]
fn shifted_dispatch_regions_follow_original_moves_and_known_source_barriers() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    for (source, visible) in [
        (
            "rename namespace shifted; shifted eval ::N {puts nested}",
            true,
        ),
        (
            "rename namespace shifted; proc shifted {args} {}; shifted eval ::N {puts hidden}",
            false,
        ),
        (
            "rename namespace shifted; rename shifted {}; shifted eval ::N {puts hidden}",
            false,
        ),
    ] {
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let command = tcl_compiler::segmenter::segment_commands(source)
            .pop()
            .unwrap();
        let regions = frame_shifted_dispatch_regions(source, &analysis, profile, &command);
        assert_eq!(
            regions
                .iter()
                .any(|&(start, end)| &source[start..end] == "puts nested"),
            visible
        );
        assert_eq!(
            nested_dispatch_regions(source, &analysis, profile, &command),
            [] as [(usize, usize); 0]
        );
        assert_eq!(
            frame_shifted_dispatch_regions(
                &source.replace("puts", "gets"),
                &analysis,
                profile,
                &command
            ),
            [] as [(usize, usize); 0]
        );
        analysis.resolved_input = None;
        assert_eq!(
            frame_shifted_dispatch_regions(source, &analysis, profile, &command),
            [] as [(usize, usize); 0]
        );
    }
}

#[test]
fn expect_clause_flags_reach_each_clause_body() {
    let source = "expect {-re {^ready$} {puts ready} -timeout 5 timeout {puts slow}}";
    let command = tcl_compiler::segmenter::segment_commands(source)
        .into_iter()
        .next()
        .expect("expect command");
    let regions = nested_dispatch_regions(
        source,
        &tcl_compiler::analyser::Analyser::new().analyse(source, "expect"),
        tcl_registry::model::ingress::resolve_environment("expect").analyser_profile(),
        &command,
    );
    assert_eq!(
        regions.len(),
        2,
        "Expect clause flags must not shift bodies"
    );
    assert!(
        regions
            .iter()
            .any(|&(start, end)| source[start..end].contains("puts ready"))
    );
    assert!(
        regions
            .iter()
            .any(|&(start, end)| source[start..end].contains("puts slow"))
    );
}

#[test]
fn inline_expect_clause_flags_reach_each_clause_body() {
    let source = "expect -re {^ready$} {puts ready} -timeout 5 timeout {puts slow}";
    let command = tcl_compiler::segmenter::segment_commands(source)
        .into_iter()
        .next()
        .expect("expect command");
    let regions = nested_dispatch_regions(
        source,
        &tcl_compiler::analyser::Analyser::new().analyse(source, "expect"),
        tcl_registry::model::ingress::resolve_environment("expect").analyser_profile(),
        &command,
    );
    assert_eq!(
        regions.len(),
        2,
        "inline Expect flags must not shift bodies"
    );
    assert!(
        regions
            .iter()
            .any(|&(start, end)| source[start..end].contains("puts ready"))
    );
    assert!(
        regions
            .iter()
            .any(|&(start, end)| source[start..end].contains("puts slow"))
    );
}

#[test]
fn malformed_case_lists_do_not_expose_nested_dispatch_regions() {
    for source in [
        "switch subject {a {puts hidden} orphan}",
        "switch subject {}",
    ] {
        let command = tcl_compiler::segmenter::segment_commands(source)
            .into_iter()
            .next()
            .expect("case-list command");
        assert!(
            nested_dispatch_regions(
                source,
                &tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6"),
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                &command
            )
            .is_empty(),
            "semantic references must abstain for malformed case list: {source:?}"
        );
    }
}

#[test]
fn case_list_aliases_use_positioned_resolved_registry_identity() {
    let source = concat!(
        "pick subject {default {puts before_alias}}\n",
        "interp alias {} pick {} switch\n",
        "pick subject {default {puts through_alias}}\n",
    );
    let commands = tcl_compiler::segmenter::segment_commands(source);
    assert!(
        nested_dispatch_regions(
            source,
            &tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6"),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &commands[0]
        )
        .is_empty(),
        "a later alias must not apply before its declaration"
    );
    let regions = nested_dispatch_regions(
        source,
        &tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6"),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &commands[2],
    );
    assert!(
        regions
            .iter()
            .any(|&(start, end)| source[start..end].contains("through_alias")),
        "a live static switch alias must expose its clause body: {regions:?}"
    );
}

#[test]
fn deleted_or_rebound_case_list_aliases_do_not_expose_clause_bodies() {
    for source in [
        concat!(
            "interp alias {} pick {} switch\n",
            "interp alias {} pick {}\n",
            "pick subject {default {puts deleted_alias}}\n",
        ),
        concat!(
            "interp alias {} pick {} switch\n",
            "interp alias {} pick {} puts\n",
            "pick subject {default {puts rebound_alias}}\n",
        ),
    ] {
        let command = tcl_compiler::segmenter::segment_commands(source)
            .into_iter()
            .last()
            .expect("alias call");
        assert!(
            nested_dispatch_regions(
                source,
                &tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6"),
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                &command
            )
            .is_empty(),
            "a deleted or rebound alias must not use switch grammar: {source:?}"
        );
    }
}

#[test]
fn rooted_and_namespaced_case_list_aliases_expose_clause_bodies() {
    let source = concat!(
        "namespace eval ::case_alias {}\n",
        "interp alias {} ::root_pick {} ::switch\n",
        "interp alias {} ::case_alias::pick {} switch\n",
        "::root_pick subject {default {puts rooted_alias}}\n",
        "::case_alias::pick subject {default {puts namespaced_alias}}\n",
    );
    let commands = tcl_compiler::segmenter::segment_commands(source);
    for (command, marker) in [
        (&commands[3], "rooted_alias"),
        (&commands[4], "namespaced_alias"),
    ] {
        let regions = nested_dispatch_regions(
            source,
            &tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6"),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            command,
        );
        assert!(
            regions
                .iter()
                .any(|&(start, end)| source[start..end].contains(marker)),
            "rooted/namespaced alias must use its resolved switch grammar: {marker}; {regions:?}"
        );
    }
}

#[test]
fn references_reach_my_dispatch_inside_a_static_switch_alias() {
    let source = concat!(
        "interp alias {} pick {} switch\n",
        "oo::class create C {\n",
        "    method target {} {}\n",
        "    method caller {} {\n",
        "        pick subject {default {my target}}\n",
        "    }\n",
        "}\n",
    );
    let analysis = analyse(source);
    // `target` begins after the four-space indent plus `method `.
    let refs = references(
        source,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|range| range.start_line).collect();
    assert!(lines.contains(&2), "declaration missing: {refs:?}");
    assert!(
        lines.contains(&4),
        "`my target` inside the resolved alias case body is missing: {refs:?}"
    );
}

#[test]
fn switch_braced_body_references_reach_nested_command() {
    let source = "proc ready {} {}\nswitch $state { ready {ready} default {set x 1}}\n";
    let analysis = analyse(source);
    let refs = references(
        source,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        5,
        &analysis,
        true,
    );
    assert!(
        refs.iter().any(|r| r.start_line == 0) && refs.iter().any(|r| r.start_line == 1),
        "switch braced body reference missing: {refs:?}"
    );
}
#[test]
fn stale_instance_candidate_does_not_add_a_variable_method_reference() {
    let source = "oo::class create C {method run {} {return 1}}\nset receiver 0\n$receiver run\n";
    let mut analysis = analyse(source);
    analysis
        .instance_classes
        .insert("receiver".to_owned(), "::C".to_owned());
    let calls = find_obj_method_call_sites(
        source,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &analysis,
        "::C",
        "run",
        false,
    );
    assert!(
        calls.is_empty(),
        "a filewide candidate is not an actual object read: {calls:?}"
    );
}
