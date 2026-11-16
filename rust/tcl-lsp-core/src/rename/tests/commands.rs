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

//! Procedure and class renames: ensemble map targets, imports, namespace prefixes, cross-document edits and collisions.

use super::*;

/// The `-map` half. Renaming the target must rewrite the
/// declaration and the `-map` *value* and leave the subcommand word
/// alone.
///
/// Oracle (tclsh 8.6.14 / 9.0.4, identical): with `-map {show
/// ::app::widget::Show}`, `::app::widget show` runs the proc and
/// `::app::widget Show` is `unknown or ambiguous subcommand "Show": must
/// be show` — the key is arbitrary, so the applied edit set must keep
/// spelling it `show`.
#[test]
fn original_primary_command_rename_uses_canonical_opaque_inputs_and_atomic_prepare() {
    // Implementation contract: naming.editor.original-command-rename-plans
    // docs/design/analysis/name-resolution-proofs/original-command-rename-plans.md
    let source = "proc p\\uD800 {argument} {return $argument}\np\\uD800 value\n";
    let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    analysis.all_procs.clear();
    analysis.global_scope.procs.clear();
    analysis.all_classes.clear();
    let prepared = prepare_rename(source, 0, 7, &analysis).unwrap();
    assert_eq!(prepared.placeholder, "p\\uD800");
    let profile = crate::profile_for_dialect("tcl8.6");
    let edits = rename_with_diagnosis(source, profile, 0, 7, "changed", &analysis, None).unwrap();
    assert_eq!(edits.len(), 2);
    let mut changed = source.to_owned();
    let index = LineIndex::new(source);
    for edit in edits.iter().rev() {
        let start = crate::definition::byte_offset_at(
            &index,
            source,
            edit.range.start_line,
            edit.range.start_character,
        );
        let end = crate::definition::byte_offset_at(
            &index,
            source,
            edit.range.end_line,
            edit.range.end_character,
        );
        changed.replace_range(start as usize..end as usize, &edit.new_text);
    }
    assert_eq!(
        changed,
        "proc changed {argument} {return $argument}\nchanged value\n"
    );
    assert!(prepare_rename("# displaced\n", 0, 5, &analysis).is_none());
    assert!(
        rename_with_diagnosis("# displaced\n", profile, 0, 5, "changed", &analysis, None).is_err()
    );
}

/// With no dispatch, nothing proves which command the `-map` value names
/// when the ensemble runs, so the rename refuses rather than leave the
/// map naming the old command.
#[test]
fn an_undispatched_map_target_refuses_rather_than_leave_the_map_stale() {
    let src = "proc ::app::Show {} { return \"showing\" }\n\
                   namespace ensemble create -command ::app::widget \
                   -map {show ::app::Show}\n";
    let analysis = analyse(src);
    let result = rename_with_diagnosis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        12,
        "Display",
        &analysis,
        None,
    );
    assert!(
        result.as_ref().is_err() || result.as_ref().is_ok_and(Vec::is_empty),
        "{result:?}"
    );
}

#[test]
fn renaming_a_map_target_does_not_rewrite_the_subcommand_word_1281() {
    let src = "namespace eval ::app::widget {}\n\
                   proc ::app::widget::Show {} { return \"showing\" }\n\
                   namespace ensemble create -command ::app::widget \
                   -map {show ::app::widget::Show}\n\
                   puts [::app::widget show]\n";
    let analysis = analyse(src);
    // Cursor on the `Show` of the declaration (line 1, col 20).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        20,
        "Display",
        &analysis,
        None,
    );
    let applied = apply_edits(src, &edits);
    assert!(
        applied.contains("proc ::app::widget::Display {}"),
        "the declaration must be renamed: {applied}"
    );
    assert!(
        applied.contains("-map {show ::app::widget::Display}"),
        "the -map value names the target and must follow it: {applied}"
    );
    assert!(
        applied.contains("puts [::app::widget show]"),
        "the -map key is arbitrary and must be left alone: {applied}"
    );
}

/// The `-subcommands` half, and the true negative for the gate above: the
/// gate must not over-correct into never rewriting an ensemble dispatch
/// word.
///
/// Oracle (tclsh 8.6.14 / 9.0.4, identical): a `-subcommands {alpha}`
/// ensemble whose proc has been renamed to `beta` answers
/// `invalid command name "alpha"` — the entry *is* the target's tail, so
/// it and the dispatch word must move with the declaration.
#[test]
fn renaming_a_subcommands_target_still_rewrites_the_subcommand_word_1281() {
    let src = "namespace eval ::app::widget {\n    \
                   proc alpha {} { return \"alpha!\" }\n    \
                   namespace ensemble create -command ::app::widget \
                   -subcommands {alpha}\n\
                   }\n\
                   puts [::app::widget alpha]\n";
    let analysis = analyse(src);
    // Cursor on the `alpha` of the declaration (line 1, col 9).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        9,
        "beta",
        &analysis,
        None,
    );
    let applied = apply_edits(src, &edits);
    assert!(
        applied.contains("proc beta {}"),
        "the declaration must be renamed: {applied}"
    );
    assert!(
        applied.contains("-subcommands {beta}"),
        "the -subcommands entry is the target's tail: {applied}"
    );
    assert!(
        applied.contains("puts [::app::widget beta]"),
        "the dispatch word must follow the target under -subcommands: {applied}"
    );
}

/// The gate is a *rename* gate, not a reference rule. Both
/// dispatch sites stay in find-references (they are genuine references
/// under either provenance); only the emitted edit sets differ. The two
/// share `invocation_references_proc`, so this is the assertion that
/// stops the hazard being "fixed" by weakening references.
#[test]
fn find_references_still_reports_both_ensemble_dispatch_sites_1281() {
    let map_src = "namespace eval ::app::widget {}\n\
                       proc ::app::widget::Show {} { return \"showing\" }\n\
                       namespace ensemble create -command ::app::widget \
                       -map {show ::app::widget::Show}\n\
                       puts [::app::widget show]\n";
    let analysis = analyse(map_src);
    let refs = crate::references::references(
        map_src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        20,
        &analysis,
        false,
    );
    assert!(
        refs.iter().any(|r| r.start_line == 3),
        "the -map dispatch site is a reference: {refs:?}"
    );

    let sub_src = "namespace eval ::app::widget {\n    \
                       proc alpha {} { return \"alpha!\" }\n    \
                       namespace ensemble create -command ::app::widget \
                       -subcommands {alpha}\n\
                       }\n\
                       puts [::app::widget alpha]\n";
    let analysis = analyse(sub_src);
    let refs = crate::references::references(
        sub_src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        9,
        &analysis,
        false,
    );
    assert!(
        refs.iter().any(|r| r.start_line == 4),
        "the -subcommands dispatch site is a reference: {refs:?}"
    );
}

/// A dynamically built `-map` records no mapping at all, so
/// the dispatch word is never attributed to the target and no rename can
/// rewrite it. The abstention is inherited from the recording site rather
/// than re-decided in rename.
#[test]
fn a_dynamic_map_leaves_the_dispatch_word_unattributed_1281() {
    let src = "namespace eval ::app::widget {}\n\
                   proc ::app::widget::Show {} { return \"showing\" }\n\
                   set m [list show ::app::widget::Show]\n\
                   namespace ensemble create -command ::app::widget -map $m\n\
                   puts [::app::widget show]\n";
    let analysis = analyse(src);
    assert!(
        analysis
            .ensemble_subcommand_targets
            .get("::app::widget")
            .is_none_or(std::collections::HashMap::is_empty),
        "a dynamic -map records no mapping: {:?}",
        analysis.ensemble_subcommand_targets,
    );
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        20,
        "Display",
        &analysis,
        None,
    );
    let applied = apply_edits(src, &edits);
    assert!(
        applied.contains("puts [::app::widget show]"),
        "an unattributed dispatch word must not be rewritten: {applied}"
    );
}

#[test]
fn rename_proc_includes_decl_and_calls() {
    let src = "proc greet {} {}\ngreet\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        "hi",
        &analysis,
        None,
    );
    assert_ne!(edits, [] as [crate::rename::TextEdit; 0]);
    assert!(edits.iter().all(|e| e.new_text == "hi"));
    // First edit is the declaration on line 0 col 5.
    assert_eq!(edits[0].range.start_line, 0);
    assert_eq!(edits[0].range.start_character, 5);
}

#[test]
fn rename_rewrites_bare_calls_to_a_proc_installed_into_oo_helpers() {
    // Renaming `::oo::Helpers::classvar` must rewrite the bare `classvar
    // hits` call site inside the method body as well as the declaration.
    // A declaration-only edit set leaves that call pointed at a
    // nonexistent name, and the next invocation crashes with "invalid
    // command name" — while the tool reports a complete, safe rename.
    let src = "proc ::oo::Helpers::classvar {name} {\n    set ns [uplevel 1 {my getONSClass}]\n    tailcall namespace upvar $ns $name $name\n}\noo::class create Counter {\n    variable _label\n    constructor {label} { set _label $label }\n    method getONSClass {} { return [self class] }\n    method bump {} {\n        classvar hits\n        incr hits\n        return \"$_label:$hits\"\n    }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `classvar` declaration (line 0, col 20).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        20,
        "renamedClassvar",
        &analysis,
        None,
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(lines.contains(&0), "decl missing: {edits:?}");
    assert!(
        lines.contains(&9),
        "the bare classvar call site inside the method body must be rewritten too: {edits:?}"
    );
    // The qualified declaration gets the qualified replacement; the
    // bare call site gets the short replacement — same convention as
    // any other namespaced-proc rename.
    let replacements: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(
        replacements.contains(&"::oo::Helpers::renamedClassvar"),
        "expected the qualified replacement at decl; got {replacements:?}"
    );
    assert!(
        replacements.contains(&"renamedClassvar"),
        "expected the short replacement at the bare call site; got {replacements:?}"
    );
}

#[test]
fn rename_unknown_word_empty() {
    let src = "puts hello\n";
    let analysis = analyse(src);
    assert_eq!(
        rename(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            "x",
            &analysis,
            None
        ),
        [] as [crate::rename::TextEdit; 0]
    );
}

#[test]
fn rename_leaves_wildcard_imported_bareword_call_site_untouched_same_document() {
    // Same-document analogue of
    // `cross_document_symbol_edits_leaves_wildcard_imported_call_site_untouched`:
    // renaming `proc bar` (reached bare elsewhere only through a
    // wildcard `namespace import`) must rewrite the declaration but
    // leave the imported bareword call site alone — it names the
    // *local* imported command, which keeps its own spelling.
    // `rename_proc` calls `invocation_references_proc` directly (not
    // the wildcard-aware `proc_reference_spans`), so it is unaffected
    // by `references()`'s newly widened same-document reference set
    // (see `references::tests::references_include_wildcard_imported_bareword_call_same_document`).
    let src = "namespace eval Foo {\n    proc bar {} { return 1 }\n    namespace export bar\n}\nnamespace import ::Foo::*\nbar\n";
    let analysis = analyse(src);
    // Cursor on the `bar` declaration (line 1, col 9).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        9,
        "baz",
        &analysis,
        None,
    );
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(edits[0].range.start_line, 1);
    assert_eq!(edits[0].new_text, "baz");
}

#[test]
fn rename_rewrites_the_renames_own_old_word_too() {
    // TP — a rename that skips this occurrence is worse than a no-op: the
    // edit set leaves a genuine `rename OLD ""` pointing at a nonexistent
    // command, so the script fails at runtime with "can't delete ...:
    // command doesn't exist" and no diagnostic anywhere.
    let src =
        "proc helperFunc {x} { return [expr {$x * 2}] }\nhelperFunc 21\nrename helperFunc \"\"\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        "newName",
        &analysis,
        None,
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert_eq!(edits.len(), 3, "{edits:?}");
    assert!(lines.contains(&0), "decl missing: {edits:?}");
    assert!(lines.contains(&1), "call site missing: {edits:?}");
    assert!(
        lines.contains(&2),
        "the rename statement's own OLD word must be rewritten too: {edits:?}"
    );
    assert!(edits.iter().all(|e| e.new_text == "newName"));
}

#[test]
fn cross_document_symbol_edits_rewrite_import_pattern_and_rename_old_word() {
    use crate::workspace_index::WorkspaceIndex;
    // `::mymod::helper` is imported by `::app` and renamed-away in a third
    // file; renaming the source must rewrite the `namespace import` pattern
    // and the `rename` OLD word so both stay bound to the new name — but
    // never the local imported/renamed *usages*.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse("namespace eval ::app {\n    namespace import ::mymod::helper\n}\n");
    let ren = analyse("rename ::mymod::helper legacy\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///mymod.tcl", &mymod),
        ("file:///app.tcl", &app),
        ("file:///ren.tcl", &ren),
    ]);
    let edits =
        cross_document_symbol_edits("::mymod::helper", "helper2", &index, "file:///mymod.tcl");
    // The import pattern (app.tcl) and the rename OLD word (ren.tcl) both
    // become the new fully-qualified name.
    assert_eq!(edits.len(), 2, "{edits:?}");
    let app_edit = edits
        .iter()
        .find(|e| e.uri == "file:///app.tcl")
        .expect("import pattern edit");
    assert_eq!(app_edit.new_text, "::mymod::helper2");
    let ren_edit = edits
        .iter()
        .find(|e| e.uri == "file:///ren.tcl")
        .expect("rename OLD edit");
    assert_eq!(ren_edit.new_text, "::mymod::helper2");
}

#[test]
fn cross_document_symbol_edits_leaves_wildcard_imported_call_site_untouched() {
    use crate::workspace_index::WorkspaceIndex;
    // A wildcard `namespace import ::mymod::*` creates no textual link to
    // rewrite (unlike the exact-import case tested
    // above): renaming `::mymod::helper` must rewrite its own
    // declaration but leave `app.tcl`'s bareword call site untouched —
    // it names the *local* imported command, which keeps its own
    // spelling, and a glob pattern has no literal occurrence of
    // `helper` to rewrite in the first place.
    let mymod = analyse("namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n");
    let app = analyse(
        "namespace eval ::app {\n    namespace import ::mymod::*\n    proc run {} { helper }\n}\n",
    );
    let index =
        WorkspaceIndex::from_documents([("file:///mymod.tcl", &mymod), ("file:///app.tcl", &app)]);
    // `current_uri` is a third, unrelated file so `mymod.tcl`'s own
    // declaration is not excluded from this cross-document sweep.
    let edits =
        cross_document_symbol_edits("::mymod::helper", "helper2", &index, "file:///caller.tcl");
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(edits[0].uri, "file:///mymod.tcl");
}

/// The class-rename tier carries the same gate. A `-map`
/// target can name a class command just as easily as a proc
/// (`-map {make ::Widget}` dispatches `widget make` to `::Widget`), and
/// `rename_class` gathers call sites through the same invocation loop, so
/// leaving it ungated would move the corruption one tier over.
#[test]
fn renaming_a_class_used_as_a_map_target_does_not_rewrite_the_subcommand_word_1281() {
    let src = "oo::class create ::Widget { method draw {} {} }\n\
                   namespace ensemble create -command ::app -map {make ::Widget}\n\
                   ::app make new\n";
    // `::app make new` dispatches to `::Widget new` (tclsh 8.6–9.1); a
    // bare `::app make` is a `wrong # args` error, not a call of the class.
    let analysis = analyse(src);
    // Cursor on the `::Widget` of the class declaration (line 0, col 17).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        0,
        17,
        "Panel",
        &analysis,
        None,
    );
    let applied = apply_edits(src, &edits);
    // The declaration keeps its written `::` qualifier and replaces only
    // the tail; `oo::class create ::Panel` with `-map {make ::Panel}`
    // prints `drawn ::Panel` on tclsh 8.6, 9.0 and 9.1.
    assert!(
        applied.contains("oo::class create ::Panel"),
        "the class declaration must be renamed: {applied}"
    );
    assert!(
        applied.contains("-map {make ::Panel}"),
        "the -map value names the class and must follow it: {applied}"
    );
    assert!(
        applied.contains("::app make new\n"),
        "the -map key is arbitrary and must be left alone: {applied}"
    );
}

/// The cross-document half: the workspace edit path
/// ([`cross_document_symbol_edits`], which `workspace_symbol_rename_edits`
/// drives over *every* document) gathers the same invocation records, so
/// it needs the same provenance gate — otherwise the corruption simply
/// moves from the in-document tier to the workspace one.
///
/// The `-subcommands` half is the control: the identical shape, sharing
/// the identical path, does rewrite its dispatch word — so the `-map`
/// verdict below is the gate at work, not a path that never reaches the
/// site.
#[test]
fn cross_document_symbol_edits_apply_the_ensemble_map_gate_1281() {
    use crate::workspace_index::WorkspaceIndex;

    let map_lib = analyse(
        "namespace eval ::app::widget {}\n\
             proc ::app::widget::Show {} { return \"showing\" }\n\
             namespace ensemble create -command ::app::widget \
             -map {show ::app::widget::Show}\n\
             puts [::app::widget show]\n",
    );
    let index = WorkspaceIndex::from_documents([("file:///lib.tcl", &map_lib)]);
    let edits = cross_document_symbol_edits(
        "::app::widget::Show",
        "Display",
        &index,
        "file:///caller.tcl",
    );
    assert!(
        edits.iter().all(|e| e.new_text != "Display"),
        "the -map key is arbitrary — no bare-tail edit may target the \
             dispatch word: {edits:?}",
    );
    assert!(
        edits.iter().any(|e| e.new_text == "::app::widget::Display"),
        "the declaration and the -map value must still be rewritten: {edits:?}",
    );

    let sub_lib = analyse(
        "namespace eval ::app::widget {\n    \
             proc alpha {} { return \"alpha!\" }\n    \
             namespace ensemble create -command ::app::widget -subcommands {alpha}\n\
             }\n\
             puts [::app::widget alpha]\n",
    );
    let index = WorkspaceIndex::from_documents([("file:///lib.tcl", &sub_lib)]);
    let edits =
        cross_document_symbol_edits("::app::widget::alpha", "beta", &index, "file:///caller.tcl");
    assert!(
        edits.iter().filter(|e| e.new_text == "beta").count() >= 2,
        "a -subcommands entry and its dispatch word must both follow the \
             target through the same path: {edits:?}",
    );
}

// namespace-aware proc renames

#[test]
fn namespace_prefix_of_qualified_names() {
    assert_eq!(namespace_prefix_of("::myns::greet"), "::myns");
    assert_eq!(namespace_prefix_of("::a::b::c"), "::a::b");
    assert_eq!(namespace_prefix_of("::greet"), "");
    assert_eq!(namespace_prefix_of("greet"), "");
}

#[test]
fn rename_namespaced_proc_keeps_namespace_prefix_at_decl() {
    // `proc ::myns::greet {} {}` renamed to `hello` should
    // rewrite the declaration's name span to
    // `::myns::hello` (keeping the namespace prefix), not
    // just `hello` (which would clobber the prefix). Without the
    // `namespace eval` tclsh 8.4-9.1 refuse the proc: `can't create
    // procedure "::myns::greet": unknown namespace`.
    let src = "namespace eval ::myns {}\nproc ::myns::greet {} {}\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        14,
        "hello",
        &analysis,
        None,
    );
    assert!(!edits.is_empty(), "{edits:?}");
    assert!(
        edits.iter().any(|e| e.new_text == "::myns::hello"),
        "expected qualified replacement at decl, got {:?}",
        edits.iter().map(|e| &e.new_text).collect::<Vec<_>>(),
    );
}

#[test]
fn rename_namespaced_proc_rewrites_call_sites_appropriately() {
    // Source has both a qualified call (`::myns::greet`)
    // and a short call (`greet` inside a `namespace eval`
    // block).  The qualified call gets the qualified
    // replacement; the short call gets the short
    // replacement. The namespace must exist before the `proc` (see
    // `rename_namespaced_proc_keeps_namespace_prefix_at_decl`).
    let src = "namespace eval ::myns {}\n\
                   proc ::myns::greet {} {}\n\
                   ::myns::greet\n\
                   namespace eval ::myns {\n\
                       greet\n\
                   }\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        14,
        "hello",
        &analysis,
        None,
    );
    let replacements: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    // Should include both `::myns::hello` (qualified) and
    // `hello` (short).
    assert!(
        replacements.contains(&"::myns::hello"),
        "expected `::myns::hello` somewhere; got {replacements:?}",
    );
    assert!(
        replacements.contains(&"hello"),
        "expected `hello` somewhere; got {replacements:?}",
    );
}

#[test]
fn rename_top_level_proc_uses_short_form_at_decl() {
    // For a top-level proc (`proc greet {} {}` →
    // `proc.qualified_name == "::greet"`, no enclosing
    // namespace prefix), the declaration rewrite stays
    // unqualified (`hello`, not `::hello`).
    let src = "proc greet {} {}\ngreet\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        "hello",
        &analysis,
        None,
    );
    assert!(
        edits.iter().any(|e| e.new_text == "hello"),
        "expected short `hello` at decl; got {:?}",
        edits.iter().map(|e| &e.new_text).collect::<Vec<_>>(),
    );
    // No qualified `::hello` rewrites either — the source
    // never uses a qualified call.
    assert!(
        edits.iter().all(|e| e.new_text == "hello"),
        "expected every edit to be `hello`; got {:?}",
        edits.iter().map(|e| &e.new_text).collect::<Vec<_>>(),
    );
}

// class rename

/// Oracle (tclsh 8.6, 9.0 and 9.1): renaming only the declaration and
/// the static call heads breaks each of these programs. A name reached
/// dynamically (`$c new`, `oo::define $c`) fails with `invalid command
/// name "MyClass"` or `MyClass does not refer to an object`; a
/// `superclass`, `mixin` or `forward` operand left unwritten fails the
/// same way; an `[info object class]` comparison prints `different`
/// instead of `same`; and a taken name fails with `can't create object
/// "Renamed": command already exists with that name`. A superclass the
/// document does not declare runs a constructor nobody has read. A
/// method comparing `[self class]` or `[info object class [self]]` with
/// `::MyClass` prints `0` instead of `1`. Each rename must refuse.
#[test]
fn rename_class_refuses_dynamic_relation_reflected_and_colliding_uses() {
    for source in [
        "oo::class create MyClass {}\nset c MyClass\n$c new\n",
        "oo::class create MyClass {}\nset c MyClass\noo::define $c {method m {} {}}\n",
        "oo::class create MyClass {}\nMyClass create obj\n\
             if {[info object class obj] eq \"::MyClass\"} {puts same}\n",
        "oo::class create MyClass {}\noo::class create Sub {superclass MyClass}\n",
        "oo::class create MyClass {}\noo::class create Other {mixin MyClass}\n",
        "oo::class create MyClass {}\nset b MyClass\noo::class create Sub {superclass $b}\n",
        "oo::class create MyClass {\n    forward mk MyClass new\n}\n",
        "oo::class create MyClass {}\nproc Renamed {} {}\nMyClass new\n",
        "oo::class create MyClass {}\noo::class create Renamed {}\n",
        "oo::class create MyClass {superclass ::pkg::Base}\nMyClass new\n",
        "oo::class create MyClass {\n    method isme {} {string equal [self class] ::MyClass}\n}\n",
        "oo::class create MyClass {\n    \
             method isme {} {string equal [info object class [self]] ::MyClass}\n}\n",
    ] {
        let analysis = analyse(source);
        let result = rename_with_diagnosis(
            source,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            0,
            17,
            "Renamed",
            &analysis,
            None,
        );
        assert!(
            !matches!(&result, Ok(edits) if !edits.is_empty()),
            "{source}: {result:?}"
        );
    }
}

/// Oracle (tclsh 8.6, 9.0 and 9.1): renaming a subclass with its static
/// call heads, including one reached from `namespace eval`, keeps
/// `base ::Base` and the instance class follows the new name.
#[test]
fn rename_class_with_a_superclass_rewrites_every_static_call_head() {
    let source = "oo::class create Base {}\n\
                      oo::class create MyClass {\n    superclass Base\n    method m {} {}\n}\n\
                      MyClass new\n\
                      namespace eval ns { MyClass new }\n";
    let analysis = analyse(source);
    let edits = rename_with_diagnosis(
        source,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        1,
        17,
        "Renamed",
        &analysis,
        None,
    )
    .unwrap();
    assert_eq!(
        apply_edits(source, &edits),
        "oo::class create Base {}\n\
             oo::class create Renamed {\n    superclass Base\n    method m {} {}\n}\n\
             Renamed new\n\
             namespace eval ns { Renamed new }\n"
    );
}

fn rename_at_8_6(
    source: &str,
    line: u32,
    character: u32,
    new_name: &str,
) -> Result<Vec<TextEdit>, crate::rename_safety::RenameRefusal> {
    rename_with_diagnosis(
        source,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        line,
        character,
        new_name,
        &analyse(source),
        None,
    )
}

/// Oracle (tclsh 8.6, 9.0, 9.1): a method or constructor body looks a
/// command up in the object's namespace, then `::oo::Helpers`, then
/// global. Renaming the global procedure and both member-body calls
/// keeps the program printing `h`.
#[test]
fn rename_proc_rewrites_calls_in_method_and_constructor_bodies() {
    let source = "proc helper {} {return h}\n\
                      oo::class create C {\n    constructor {} {helper}\n    \
                      method m {} {set r [helper]}\n}\n\
                      C new\n";
    let edits = rename_at_8_6(source, 0, 5, "renamed").unwrap();
    assert_eq!(
        apply_edits(source, &edits),
        "proc renamed {} {return h}\n\
             oo::class create C {\n    constructor {} {renamed}\n    \
             method m {} {set r [renamed]}\n}\n\
             C new\n"
    );
}

/// Oracle (tclsh 8.6, 9.0, 9.1): a conditional body inside a method runs
/// in the method's frame and namespace, so its call follows the same
/// lookup (`if_*`: `h` before and after).
#[test]
fn rename_proc_rewrites_a_call_in_a_method_conditional_body() {
    let source = "proc helper {} {return h}\n\
                      oo::class create C {\n    method m {} {if {1} {helper}}\n}\n";
    let edits = rename_at_8_6(source, 0, 5, "renamed").unwrap();
    assert_eq!(
        apply_edits(source, &edits),
        "proc renamed {} {return h}\n\
             oo::class create C {\n    method m {} {if {1} {renamed}}\n}\n"
    );
}

/// Oracle (tclsh 8.6, 9.0, 9.1): `::oo::Helpers::helper` shadows the
/// global `helper` inside a method (`helpers` before and after), so the
/// method's call is not a reference to the renamed global procedure.
#[test]
fn rename_proc_leaves_a_method_call_that_oo_helpers_shadows() {
    let source = "proc ::oo::Helpers::helper {} {return helpers}\n\
                      proc helper {} {return global}\n\
                      oo::class create C {\n    method m {} {set r [helper]}\n}\n";
    if let Ok(edits) = rename_at_8_6(source, 1, 5, "renamed") {
        assert_eq!(
            apply_edits(source, &edits),
            "proc ::oo::Helpers::helper {} {return helpers}\n\
                 proc renamed {} {return global}\n\
                 oo::class create C {\n    method m {} {set r [helper]}\n}\n"
        );
    }
}

/// Oracle (tclsh 8.6, 9.0, 9.1): the object namespace's `my` and
/// `::oo::Helpers` both precede global lookup in a method, so a member
/// call renamed to `my` or to a Helpers command would reach it
/// (`c_collide`: `helpers`, not `global`); a relative `proc` in a method
/// body publishes into the object namespace (`e_pub`: `local`).
#[test]
fn rename_proc_refuses_names_the_method_lookup_reaches_first() {
    for (source, new_name) in [
        (
            "proc ::oo::Helpers::renamed {} {return helpers}\n\
                 proc helper {} {return global}\n\
                 oo::class create C {\n    method m {} {set r [helper]}\n}\n",
            "renamed",
        ),
        (
            "proc helper {} {return global}\n\
                 oo::class create C {\n    method m {} {set r [helper]}\n}\n",
            "my",
        ),
        (
            "proc helper {} {return global}\n\
                 oo::class create C {\n    \
                 method m {} {proc renamed {} {return local}; return [helper]}\n}\n",
            "renamed",
        ),
    ] {
        let line = u32::from(source.starts_with("proc ::oo"));
        let result = rename_at_8_6(source, line, 5, new_name);
        assert!(
            !matches!(&result, Ok(edits) if !edits.is_empty()),
            "{source}: {result:?}"
        );
    }
}

/// Oracle (tclsh 8.6, 9.0, 9.1): renaming a class whose method calls a
/// global procedure keeps `hi h`.
#[test]
fn rename_class_whose_method_body_calls_a_procedure() {
    let source = "proc helper {} {return h}\n\
                      oo::class create MyClass {\n    method greet {} {set greeting \"hi [helper]\"}\n}\n\
                      MyClass new\n";
    let edits = rename_at_8_6(source, 1, 17, "Renamed").unwrap();
    assert_eq!(
        apply_edits(source, &edits),
        "proc helper {} {return h}\n\
             oo::class create Renamed {\n    method greet {} {set greeting \"hi [helper]\"}\n}\n\
             Renamed new\n"
    );
}

#[test]
fn rename_class_at_decl_rewrites_decl_and_calls() {
    // `oo::class create MyClass { ... }` plus a `MyClass new`
    // invocation — both should be rewritten.
    let src = "oo::class create MyClass {\n\
                       method greet {} {}\n\
                   }\n\
                   MyClass new\n";
    let analysis = analyse(src);
    // Cursor on the `MyClass` declaration name (column 17).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        0,
        17,
        "Renamed",
        &analysis,
        None,
    );
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(!edits.is_empty(), "expected non-empty edits");
    assert!(
        texts.iter().all(|t| *t == "Renamed"),
        "every replacement should be `Renamed`; got {texts:?}",
    );
    // The decl site + the `MyClass new` call site = 2 edits
    // (more if the analyser surfaces inheritance refs).
    assert!(edits.len() >= 2, "{edits:?}");
}

#[test]
fn rename_class_at_call_site_also_works() {
    // Cursor on the `MyClass` head of an invocation — same
    // outcome as renaming from the declaration.
    let src = "oo::class create MyClass {\n}\nMyClass new\n";
    let analysis = analyse(src);
    // Cursor on the `MyClass` in `MyClass new` (line 2, col 3).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        2,
        3,
        "Renamed",
        &analysis,
        None,
    );
    assert!(!edits.is_empty(), "{edits:?}");
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(
        texts.iter().all(|t| *t == "Renamed"),
        "expected every edit to be `Renamed`; got {texts:?}",
    );
}

#[test]
fn rename_class_rejects_unsafe_new_name() {
    let src = "oo::class create MyClass {}\nMyClass new\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        17,
        "1bad",
        &analysis,
        None,
    );
    assert!(edits.is_empty(), "{edits:?}");
}

#[test]
fn rename_class_blocked_when_shadowing_builtin() {
    let src = "oo::class create MyClass {}\nMyClass new\n";
    let analysis = analyse(src);
    let mut r = tcl_registry::CommandRegistry::build_default();
    r.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        17,
        "if",
        &analysis,
        Some(&r),
    );
    assert!(
        edits.is_empty(),
        "renaming to a built-in should be blocked; got {edits:?}",
    );
}

#[test]
fn prepare_rename_returns_range_for_class() {
    let src = "oo::class create MyClass {}\n";
    let analysis = analyse(src);
    let p = prepare_rename(src, 0, 17, &analysis).expect("expected prepare_rename on class");
    assert_eq!(p.placeholder, "MyClass");
    // Anchored at the class name span.
    assert_eq!(p.range.start_line, 0);
    // The span sits at column 17.
    assert_eq!(p.range.start_character, 17);
}

#[test]
fn rename_proc_rejected_on_existing_proc_collision() {
    let src = "proc greet {} {}\nproc hello {} {}\ngreet\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        "hello",
        &analysis,
        None,
    );
    assert!(edits.is_empty(), "proc collision not rejected: {edits:?}");
}
