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

//! Procedure call sites: retained declarations, redefinitions, renames, imports, unbraced bodies and qualified names.

use super::*;

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
        invocation_dialect: Some(tcl_native_model::InvocationDialect::for_version(
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

// tcl::OptProc — the `opt` package's automatic-option-parsing proc
// definer: without its analyser hook the call site is unreachable from
// the declaration.

#[test]
fn references_from_opt_proc_declaration_reach_the_call_site() {
    // tclsh 8.4.20-9.1.0: `::tcl::OptProc` is an invalid command name until
    // `package require opt`; with it, `greet foo` returns foo.
    let src = "package require opt\n::tcl::OptProc greet {child -use -display} { return $child }\ngreet foo\n";
    let analysis = analyse(src);
    // Line 1 — cursor on "greet" right after `::tcl::OptProc` (col 15).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        15,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(lines.contains(&2), "call site missing: {refs:?}");
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

/// A command word built from braced and interpolated variable names folds
/// to the procedure it calls. tclsh 8.4.20-9.1.0: the last word runs the
/// procedure (hit1).
#[test]
fn references_reach_a_call_whose_head_folds_braced_and_interpolated_names() {
    let src = "proc WRONG:RIGHT:SCALAR {} {return hit1}\nset n WRONG; set {$n} RIGHT; set {scalar(open} SCALAR; ${n}:${$n}:${scalar(open}\n";
    let analysis = Analyser::new().analyse(src, "tcl").clone();
    let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let refs = references(src, profile, 0, 6, &analysis, true);
    let call = u32::try_from(src.rfind("${n}:").unwrap() - src.find('\n').unwrap() - 1).unwrap();
    assert!(
        refs.iter()
            .any(|r| r.start_line == 1 && r.start_character == call),
        "the folded command word is a call of the procedure: {refs:?}"
    );
    let stale = src.replace("RIGHT", "OTHER");
    assert!(
        references(&stale, profile, 0, 6, &analysis, true)
            .iter()
            .all(|r| r.start_line != 1),
        "a changed source withdraws the folded call"
    );
}

/// A nested list built from original variable components folds to the
/// procedure's name. tclsh 8.4.20-9.1.0: `$result` runs {WRONG RIGHT} (hit2).
#[test]
fn references_reach_a_call_whose_head_folds_a_nested_list() {
    let src = "proc {WRONG RIGHT} {} {return hit2}\nset n WRONG; set {$n} RIGHT; set result [list $n [list ${$n}]]; $result\n";
    let analysis = Analyser::new().analyse(src, "tcl").clone();
    let profile = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let refs = references(src, profile, 0, 6, &analysis, true);
    let call = u32::try_from(src.rfind("$result").unwrap() - src.find('\n').unwrap() - 1).unwrap();
    assert!(
        refs.iter()
            .any(|r| r.start_line == 1 && r.start_character == call),
        "the folded command word is a call of the procedure: {refs:?}"
    );
}
