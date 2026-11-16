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
use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};
use tcl_dialect::model::{Family, SurfaceLayer};

#[test]
fn rename_edit_plan_refuses_conflicts_before_utf16_publication() {
    use tcl_lexer::Span;
    let source = "αbc def";
    let index = LineIndex::new(source);
    let edit = |start, end, text: &str| PendingRenameEdit {
        span: Span::new(start, end),
        new_text: text.to_owned(),
    };
    let same = edit(0, 3, "x");
    let accepted = finish_rename_edits(
        source,
        &index,
        vec![same.clone(), edit(5, 8, "y"), same.clone()],
    )
    .unwrap();
    assert_eq!(accepted.len(), 2);
    assert_eq!(accepted[0].range.end_character, 2);
    assert_eq!(accepted[0].new_text, "x");
    assert!(finish_rename_edits(source, &index, vec![same.clone(), edit(0, 3, "z")]).is_err());
    assert!(finish_rename_edits(source, &index, vec![same.clone(), edit(2, 4, "x")]).is_err());
    assert!(finish_rename_edits(source, &index, vec![edit(0, 8, "x"), same]).is_err());
    assert!(finish_rename_edits(source, &index, vec![edit(1, 3, "x")]).is_err());
    assert!(finish_rename_edits(source, &index, vec![edit(5, 9, "x")]).is_err());
    assert!(finish_rename_edits(source, &index, vec![edit(2, 2, "x")]).is_err());
    assert_eq!(
        finish_rename_edits(source, &index, vec![edit(2, 3, "x"), edit(3, 4, "y")])
            .unwrap()
            .len(),
        2
    );
}

fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

/// Apply every edit's `(range, new_text)` back onto `source` — the
/// same thing an editor does — sorted back-to-front so earlier byte
/// offsets stay valid as later edits are spliced in. Some tests below
/// use this to check the *result*, not just that `new_text` looks
/// right in isolation: a rename edit range short by one byte is
/// invisible to a test that only asserts `new_text` and never applies
/// it.
fn apply_edits(source: &str, edits: &[TextEdit]) -> String {
    let line_index = LineIndex::new(source);
    let mut spans: Vec<(usize, usize, &str)> = edits
        .iter()
        .map(|e| {
            let start = crate::definition::byte_offset_at(
                &line_index,
                source,
                e.range.start_line,
                e.range.start_character,
            ) as usize;
            let end = crate::definition::byte_offset_at(
                &line_index,
                source,
                e.range.end_line,
                e.range.end_character,
            ) as usize;
            (start, end, e.new_text.as_str())
        })
        .collect();
    spans.sort_by_key(|s| std::cmp::Reverse(s.0));
    let mut result = source.to_string();
    for (start, end, new_text) in spans {
        result.replace_range(start..end, new_text);
    }
    result
}

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

#[test]
fn original_primary_member_rename_does_not_enter_string_override_families() {
    // Implementation contract: naming.editor.original-command-rename-plans
    // docs/design/analysis/name-resolution-proofs/original-command-rename-plans.md
    // The family is the resolved relation component, never equal names:
    // `D` inherits `ping` without declaring it and unrelated `E` declares
    // its own. tclsh 8.6/9.0/9.1 print `destroy ping/e` before and
    // `changed destroy/e` after renaming only `C`'s declaration.
    let source = "oo::class create C {method ping {} {return}}\noo::class create D {superclass C}\noo::class create E {method ping {} {return e}}\n";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    let row = analysis.original_class_declarations().next().unwrap();
    assert_eq!(
        row.metadata()
            .original_members
            .methods(tcl_compiler::analyser::types::MemberSide::Instance)
            .unwrap()
            .len(),
        1
    );
    let offset = u32::try_from(source.find("ping").unwrap()).unwrap();
    let position = LineIndex::new(source).position_at_utf16(offset, source);
    assert!(prepare_rename(source, position.line, position.character.get(), &analysis).is_some());
    let edits = rename_with_diagnosis(
        source,
        crate::profile_for_dialect("tcl8.6"),
        position.line,
        position.character.get(),
        "changed",
        &analysis,
        None,
    )
    .unwrap();
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(
        (edits[0].range.start_line, edits[0].range.start_character),
        (position.line, position.character.get())
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
fn rename_rewrites_a_method_return_captured_dispatch_site() {
    // `b` is typed only by the object-type lattice's method-return edge,
    // which makes the site a proven `::B` dispatch rather than an
    // untracked receiver: the rename proceeds and rewrites it.
    let src = "oo::class create A { method make {} { ::return [::B new] } }\n\
                   oo::class create B { method greet {} { ::return \"hi\" } }\n\
                   set a [A new]\n\
                   set b [$a make]\n\
                   $b greet\n";
    let analysis = analyse(src);
    // Cursor on the `greet` declaration (line 1, col 29).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        29,
        "salute",
        &analysis,
        None,
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(lines.contains(&1), "declaration edit missing: {edits:?}");
    assert!(
        lines.contains(&4),
        "the lattice-typed `$b greet` call site must be rewritten: {edits:?}"
    );
    let result = apply_edits(src, &edits);
    assert!(
        result.contains("$b salute") && result.contains("method salute"),
        "applied rename must leave a consistent document:\n{result}"
    );
}

#[test]
fn rename_still_refuses_on_a_genuinely_untracked_receiver() {
    // The refusal gate stays honest: a `$x greet` whose receiver has no
    // binding in either map is still the untracked-receiver
    // hazard, so the rename must refuse (empty edit set) rather than
    // leave the site pointing at a dead name.
    let src = "oo::class create B { method greet {} { return \"hi\" } }\n\
                   proc use {x} { $x greet }\n\
                   set b [B new]\n\
                   $b greet\n";
    let analysis = analyse(src);
    // `x` is a proc parameter — the lattice has no caller to type it
    // from, so it stays untracked.
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        28,
        "salute",
        &analysis,
        None,
    );
    assert!(
        edits.is_empty(),
        "an untracked `$x greet` dispatch must keep refusing the rename: {edits:?}"
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
fn rename_from_the_in_proc_global_alias_rewrites_the_callers_canonical_set_too() {
    // Triggering Rename from the in-proc `global tolComp` alias must
    // rewrite the caller's `set ::tolComp 0.05` too, not just the proc's
    // own 4 spans.  Leaving the caller behind decouples the two, so the
    // program silently falls back to `isEqual`'s hardcoded 0.01 default
    // instead of the caller-intended tolerance — a runtime break
    // introduced by a "successful" rename.
    let src =
        "proc use {} {\n    global tolComp\n    return $tolComp\n}\nset ::tolComp 0.05\nuse\n";
    let analysis = analyse(src);
    // Cursor on the `$tolComp` read inside the proc (line 2, col 14).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        14,
        "tolerance",
        &analysis,
        None,
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(lines.contains(&1), "the `global tolComp` decl: {edits:?}");
    assert!(lines.contains(&2), "the in-proc $tolComp read: {edits:?}");
    assert!(
        lines.contains(&4),
        "the caller's canonical `set ::tolComp 0.05` must be rewritten too: {edits:?}"
    );
    let replacements: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(
        replacements.contains(&"::tolerance"),
        "the canonical cell's own `::` qualifier must be preserved; got {replacements:?}"
    );
}

#[test]
fn rename_from_the_callers_canonical_set_rewrites_every_in_proc_global_alias_too() {
    // The reverse direction of the test above:
    // triggering Rename from the caller's own `set ::tolComp` must
    // rewrite every in-proc `global tolComp` occurrence too, not just
    // the caller's own 2 spans.
    let src =
        "proc use {} {\n    global tolComp\n    return $tolComp\n}\nset ::tolComp 0.05\nuse\n";
    let analysis = analyse(src);
    // Cursor on the `set ::tolComp` declaration (line 4, col 8).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        8,
        "tolerance",
        &analysis,
        None,
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(
        lines.contains(&4),
        "the caller's own decl must be rewritten: {edits:?}"
    );
    assert!(
        lines.contains(&1),
        "the in-proc `global tolComp` decl must be rewritten too: {edits:?}"
    );
    assert!(
        lines.contains(&2),
        "the in-proc $tolComp read must be rewritten too: {edits:?}"
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
fn workspace_resolves_inherited_namespace_wrapped_callback_with_internal_access() {
    use crate::workspace_index::{MethodAccess, WorkspaceIndex};

    let base_src = "oo::class create Base {\n    method read {} {}\n    unexport read\n}\n";
    let child_src = "oo::class create Child {\n    superclass Base\n    method wire {chan} {\n        fileevent $chan readable [namespace code [list my read]]\n    }\n}\n";
    let base = analyse(base_src);
    let child = analyse(child_src);
    let index = WorkspaceIndex::from_documents([
        ("file:///base.tcl", &base),
        ("file:///child.tcl", &child),
    ]);
    // The resolved target is the **receiver**, exactly as a bare `$obj
    // method` position resolves to one: the provider is whatever the
    // workspace chain rooted there reaches, and re-rooting the chain at
    // the provider instead would drop the receiver's own `export` /
    // `unexport` stub on the way.
    assert_eq!(
        method_target_with_access_in_workspace(
            child_src,
            3,
            60,
            &child,
            Some((&index, "file:///child.tcl")),
        ),
        Some((
            "::Child".to_owned(),
            "read".to_owned(),
            false,
            MethodAccess::Internal,
        )),
    );
    // …and that chain still names `Base` as the provider, so the
    // definition / lens / rename tiers reach the inherited declaration.
    assert_eq!(
        index
            .method_dispatch_chain("::Child", "read", MethodAccess::Internal)
            .first()
            .map(|c| c.uri.as_str()),
        Some("file:///base.tcl"),
    );
}

#[test]
fn workspace_rejects_inherited_namespace_wrapped_external_private_callback() {
    use crate::workspace_index::WorkspaceIndex;

    let base_src = "oo::class create Base {\n    method read {} {}\n    unexport read\n}\n";
    let child_src = "oo::class create Child {\n    superclass Base\n    method wire {chan} {\n        fileevent $chan readable [namespace code [list [self] read]]\n    }\n}\n";
    let base = analyse(base_src);
    let child = analyse(child_src);
    let index = WorkspaceIndex::from_documents([
        ("file:///base.tcl", &base),
        ("file:///child.tcl", &child),
    ]);
    assert!(
        method_target_with_access_in_workspace(
            child_src,
            3,
            64,
            &child,
            Some((&index, "file:///child.tcl")),
        )
        .is_none()
    );
}

#[test]
fn workspace_callback_dispatch_obeys_mixin_mro_over_child_override() {
    use crate::workspace_index::{MethodAccess, WorkspaceIndex};

    let base = analyse("oo::class create Base { method read {} {} }\n");
    let mixin = analyse("oo::class create Mixin { method read {} {} }\n");
    let child_src = "oo::class create Child {\n    superclass Base\n    mixin Mixin\n    method read {} {}\n    method wire {chan} { fileevent $chan readable [namespace code [list my read]] }\n}\n";
    let child = analyse(child_src);
    let index = WorkspaceIndex::from_documents([
        ("file:///base.tcl", &base),
        ("file:///mixin.tcl", &mixin),
        ("file:///child.tcl", &child),
    ]);
    assert_eq!(
        method_target_with_access_in_workspace(
            child_src,
            4,
            75,
            &child,
            Some((&index, "file:///child.tcl")),
        ),
        Some((
            "::Child".to_owned(),
            "read".to_owned(),
            false,
            MethodAccess::Internal,
        )),
    );
    // The MRO fact this test exists for, pinned where it is decided: a
    // `mixin` is searched ahead of the class's own table, so the chain
    // rooted at the receiver enters `Mixin::read`, not `Child`'s override.
    assert_eq!(
        index
            .method_dispatch_chain("::Child", "read", MethodAccess::Internal)
            .first()
            .map(|c| c.uri.as_str()),
        Some("file:///mixin.tcl"),
    );
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

#[test]
fn rename_var_includes_decl_span() {
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    // Cursor inside `$x`.
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        "y",
        &analysis,
        None,
    );
    assert_ne!(edits, [] as [crate::rename::TextEdit; 0]);
    // Declaration replaces just `x` → `y`; reference
    // replaces `$x` → `$y` so the `$` prefix is preserved.
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(texts.contains(&"y"), "{texts:?}");
    assert!(texts.contains(&"$y"), "{texts:?}");
}

#[test]
fn rename_from_proc_param_bareword_declaration_rewrites_every_use() {
    // TP — renaming from a cursor placed directly on a proc parameter's
    // own bareword name (not a `$`-prefixed read) must rewrite both the
    // parameter's declaration and its `$name` read.  Answering *zero*
    // edits is worse than failing loudly: the user has no signal that
    // anything went wrong.
    let src = "proc greet {name} { return $name }\n";
    let analysis = analyse(src);
    // Cursor on `name` inside the parameter list (col 12-16).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        13,
        "label",
        &analysis,
        None,
    );
    assert_eq!(edits.len(), 2, "{edits:?}");
    // The declaration rewrites bare `name` -> `label`; the `$name` read
    // preserves its `$` prefix -> `$label` (mirrors
    // `rename_var_includes_decl_span`'s established convention).
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(texts.contains(&"label"), "{texts:?}");
    assert!(texts.contains(&"$label"), "{texts:?}");
    assert!(edits.iter().any(|e| e.range.start_line == 0), "{edits:?}");
}

#[test]
fn rename_from_catch_resultvar_bareword_rewrites_the_original_declaration_too() {
    // TP — the finding's other confirmed shape: a `catch script name`
    // result-var reuses an existing variable; renaming from a cursor
    // on its own bareword token must rewrite every occurrence,
    // including the original declaration elsewhere in the proc.
    let src = "proc resolveSwitch {name def} {\n    catch {error boom} name\n    return $name\n}\n";
    let analysis = analyse(src);
    // Cursor on the catch result-var `name` (line 1, col 23-27).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        24,
        "resolved",
        &analysis,
        None,
    );
    assert!(
        edits.iter().any(|e| e.range.start_line == 0),
        "original param declaration must be rewritten too: {edits:?}"
    );
    assert!(
        edits.iter().any(|e| e.range.start_line == 1),
        "the catch result-var site itself must be rewritten: {edits:?}"
    );
    assert!(
        edits.iter().any(|e| e.range.start_line == 2),
        "the later $name read must be rewritten: {edits:?}"
    );
    // The formal list and lexical root have independent edit extents;
    // applying the complete edit set preserves the read's original `$`.
    assert_eq!(
        apply_edits(src, &edits),
        "proc resolveSwitch {resolved def} {\n    catch {error boom} resolved\n    return $resolved\n}\n",
    );
}

// brace-ref escaping

#[test]
fn rename_var_preserves_braced_reference_form() {
    let src = "set x 1\nputs ${x}\n";
    let analysis = analyse(src);
    // Cursor inside `${x}` on the `x`.
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        "y",
        &analysis,
        None,
    );
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(texts.contains(&"y"), "{texts:?}");
    assert!(texts.contains(&"${y}"), "{texts:?}");
}

#[test]
fn var_ref_edit_span_extends_the_ordinary_braced_form() {
    let src = "${x}";
    // The lexer's own span for a non-degenerate name stops one byte
    // short of the closing `}` (see `var_ref_edit_span`'s doc).
    assert_eq!(
        var_ref_edit_span(src, tcl_lexer::Span::new(0, 3)),
        tcl_lexer::Span::new(0, 4)
    );
}

#[test]
fn var_ref_edit_span_leaves_the_degenerate_empty_name_span_untouched() {
    // `${}` immediately followed by a literal (unrelated) `}` — the
    // pathological case that would fool a naive "does the next byte
    // look like `}`" check. The lexer already extends `${}`'s own
    // span to include its closing brace, so `var_ref_edit_span` must
    // recognise the degenerate case and stop, not also swallow the
    // following literal `}` into the edit range.
    let src = "${}}";
    assert_eq!(
        var_ref_edit_span(src, tcl_lexer::Span::new(0, 3)),
        tcl_lexer::Span::new(0, 3)
    );
}

#[test]
fn var_ref_edit_span_extends_a_tcl9_nested_brace_name() {
    // `${a{b}}` names `a{b}` (Tcl 9's brace-nesting rule) — the span
    // stops before the *outer* `}`, one byte short, same as the
    // ordinary case.
    let src = "${a{b}}";
    assert_eq!(
        var_ref_edit_span(src, tcl_lexer::Span::new(0, 6)),
        tcl_lexer::Span::new(0, 7)
    );
}

#[test]
fn var_ref_edit_span_covers_a_braced_name_word() {
    // A braced *name* word (`set {a b} 1`) follows the same inner-end
    // convention as `${…}`, so an edit range that stops short leaves the
    // `}` orphaned after the replacement.  `word_span_at` covers every
    // delimited shape, so both are widened the same way.
    let src = "set {a b} 1";
    assert_eq!(&src[4..8], "{a b");
    assert_eq!(
        var_ref_edit_span(src, tcl_lexer::Span::new(4, 8)),
        tcl_lexer::Span::new(4, 9)
    );
}

#[test]
fn var_ref_edit_span_leaves_an_empty_braced_word_untouched() {
    // The universal discriminator: an empty `{}` whose span already
    // covers its own closer, with the enclosing body's `}` right after.
    let src = "proc p {} {}";
    assert_eq!(
        var_ref_edit_span(src, tcl_lexer::Span::new(7, 9)),
        tcl_lexer::Span::new(7, 9)
    );
}

#[test]
fn var_ref_edit_span_leaves_non_braced_forms_untouched() {
    let src = "$x";
    let span = tcl_lexer::Span::new(0, 2);
    assert_eq!(var_ref_edit_span(src, span), span);
}

#[test]
fn var_ref_edit_span_leaves_an_unterminated_reference_untouched() {
    // No closing brace anywhere in the source to extend to.
    let src = "${x";
    let span = tcl_lexer::Span::new(0, 3);
    assert_eq!(var_ref_edit_span(src, span), span);
}

#[test]
fn rename_array_variable_applying_the_braced_index_edit_does_not_duplicate_the_closing_brace() {
    let src = "set arr(0) 1\nputs ${arr(0)}\n";
    let analysis = analyse(src);
    // Cursor on `arr` inside `${arr(0)}` (line 1, col 7).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        "data",
        &analysis,
        None,
    );
    assert_eq!(apply_edits(src, &edits), "set data(0) 1\nputs ${data(0)}\n");
}

#[test]
fn rename_var_applying_the_braced_reference_edit_does_not_duplicate_the_closing_brace() {
    // The `Var` token's own lexer span for a
    // non-degenerate `${name}` form stops one byte short of the
    // closing `}` (`${a{b}}` names `a{b}`, whose content can itself
    // legitimately end in `}` — see `var_ref_edit_span`'s doc), so
    // using that raw span as the *edit range* (rather than just for
    // matching/reference bookkeeping) left the source's own original
    // `}` sitting right after the replacement once applied, silently
    // corrupting `${x}` into `${y}}`. `rename_var_preserves_braced_reference_form`
    // above already pins `new_text`; this applies the edit for real.
    let src = "set x 1\nputs ${x}\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        "y",
        &analysis,
        None,
    );
    assert_eq!(apply_edits(src, &edits), "set y 1\nputs ${y}\n");
}

#[test]
fn rename_var_applying_the_dir_view_idiom_edit_produces_valid_tcl() {
    // The real `tk/library/tk.tcl:594-596` idiom this finding traces
    // through (`$w ${dir}view scroll ...`, subcommand synthesised by
    // concatenating `$dir` with literal `view`): a one-byte-short edit
    // range applies as `$w ${direction}}view ...`, and tclsh8.6/9.0 both
    // fail to even parse the enclosing proc ("extra characters after
    // close-brace"), since the stray extra `}` shifts Tcl's own
    // brace-counting scan for where the proc body ends.
    let src = "proc ::tk::MouseWheel {w dir amount {factor -120.0} {units units}} {\n    $w ${dir}view scroll [expr {$amount/$factor}] $units\n}\n";
    let analysis = analyse(src);
    // Cursor on the `d` of `dir` inside `${dir}view` (line 1, col 9).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        9,
        "direction",
        &analysis,
        None,
    );
    let applied = apply_edits(src, &edits);
    assert!(
        applied.contains("${direction}view"),
        "expected a single, correctly-closed brace: {applied}"
    );
    assert!(
        !applied.contains("${direction}}view"),
        "must not duplicate the closing brace: {applied}"
    );
}

#[test]
fn rename_array_variable_preserves_element_index() {
    // Renaming the base array variable `arr` must keep each
    // element index intact: `$arr(0)` → `$data(0)`, not `$data`.
    let src = "set arr(0) 1\nputs $arr(0)\n";
    let analysis = analyse(src);
    // Cursor on `arr` in the `set` target (line 0, col 4).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        6,
        "data",
        &analysis,
        None,
    );
    assert!(!edits.is_empty(), "expected array rename edits");
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(
        texts.contains(&"$data(0)"),
        "reference edit must preserve the array index, got {texts:?}",
    );
}

#[test]
fn rename_array_variable_preserves_index_in_braced_reference() {
    // The `${arr(0)}` braced form is recorded against base `arr`,
    // so renaming must rewrite it to `${data(0)}` (index kept).
    let src = "set arr(0) 1\nputs ${arr(0)}\n";
    let analysis = analyse(src);
    // Cursor on `arr` inside `${arr(0)}` (line 1, col 7).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        "data",
        &analysis,
        None,
    );
    assert!(!edits.is_empty(), "expected braced array rename edits");
    let texts: Vec<&str> = edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(
        texts.contains(&"${data(0)}"),
        "braced reference edit must preserve the array index, got {texts:?}",
    );
}

#[test]
fn prepare_rename_returns_range_for_proc() {
    let src = "proc greet {} {}\ngreet\n";
    let analysis = analyse(src);
    let p = prepare_rename(src, 0, 6, &analysis).expect("expected prepare_rename on proc name");
    assert_eq!(p.placeholder, "greet");
    // Anchored at the proc name span.
    assert_eq!(p.range.start_line, 0);
    assert_eq!(p.range.start_character, 5);
}

#[test]
fn prepare_rename_returns_range_for_var() {
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    let p = prepare_rename(src, 1, 7, &analysis).expect("expected prepare_rename on var");
    assert_eq!(p.placeholder, "x");
}

#[test]
fn original_prepare_variable_rename_selects_the_tail_and_preserves_index_geometry() {
    let source = "namespace eval N {}\nset ::N::a(k) 1\nlist $::N::a(k) ${::N::a(k)}\n";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut analysis = Analyser::new().analyse(source, dialect);
        analysis.global_scope.variables.clear();
        for reference in &mut analysis.qualified_var_refs {
            reference.qualified_name.clear();
        }
        for prefix in ["set ::N::", "list $::N::", "${::N::"] {
            let offset = source.find(prefix).unwrap() + prefix.len();
            let position =
                LineIndex::new(source).position_at_utf16(u32::try_from(offset).unwrap(), source);
            let prepared =
                prepare_rename(source, position.line, position.character.get(), &analysis)
                    .unwrap_or_else(|| panic!("{dialect}: {prefix}"));
            assert_eq!(prepared.placeholder, "a");
            assert_eq!(prepared.range.start_line, prepared.range.end_line);
            assert_eq!(
                prepared.range.end_character - prepared.range.start_character,
                1
            );
        }
        assert!(prepare_rename("# changed\n", 0, 2, &analysis).is_none());
    }
}

#[test]
fn prepare_rename_returns_none_for_unknown_word() {
    let src = "puts hello\n";
    let analysis = analyse(src);
    // `hello` isn't a proc or var declaration.
    assert!(prepare_rename(src, 0, 6, &analysis).is_none());
}

#[test]
fn build_var_ref_replacement_handles_all_forms() {
    // Pure helper exercise — bare `$x`, braced `${x}`, and
    // qualified forms all produce the right replacement text.
    // Namespace-qualified refs preserve the prefix so e.g.
    // `$ns::z` → `$ns::c` even if the analyser doesn't yet
    // surface namespace-scoped variable lookups (the helper
    // is the building block; the lookup side isn't wired
    // yet).
    let src = "  $x  ${y}  $ns::z  ${ns::w}  ";
    let span_x = tcl_lexer::Span::new(2, 4);
    let span_braced_y = tcl_lexer::Span::new(6, 10);
    let span_qualified_z = tcl_lexer::Span::new(12, 18);
    let span_braced_w = tcl_lexer::Span::new(20, 28);
    assert_eq!(
        build_var_ref_replacement(src, span_x, "a", tcl_lexer::LexerConfig::default()).unwrap(),
        "$a"
    );
    assert_eq!(
        build_var_ref_replacement(src, span_braced_y, "b", tcl_lexer::LexerConfig::default())
            .unwrap(),
        "${b}"
    );
    assert_eq!(
        build_var_ref_replacement(
            src,
            span_qualified_z,
            "c",
            tcl_lexer::LexerConfig::default()
        )
        .unwrap(),
        "$ns::c"
    );
    assert_eq!(
        build_var_ref_replacement(src, span_braced_w, "d", tcl_lexer::LexerConfig::default())
            .unwrap(),
        "${ns::d}"
    );
}

#[test]
fn build_var_ref_replacement_preserves_array_index() {
    // Renaming the base array variable must keep the element
    // index: `$arr(idx)` → `$data(idx)`, not `$data`.
    let src = "$arr(idx)  $arr($i)  $ns::a(k)";
    assert_eq!(
        build_var_ref_replacement(
            src,
            tcl_lexer::Span::new(0, 9),
            "data",
            tcl_lexer::LexerConfig::default()
        )
        .unwrap(),
        "$data(idx)"
    );
    // A substituted index is copied verbatim (the inner `$i`
    // reference is renamed on its own if `i` is renamed).
    assert_eq!(
        build_var_ref_replacement(
            src,
            tcl_lexer::Span::new(11, 19),
            "data",
            tcl_lexer::LexerConfig::default()
        )
        .unwrap(),
        "$data($i)"
    );
    // Namespace prefix + array index together.
    assert_eq!(
        build_var_ref_replacement(
            src,
            tcl_lexer::Span::new(21, 30),
            "b",
            tcl_lexer::LexerConfig::default()
        )
        .unwrap(),
        "$ns::b(k)"
    );
}

#[test]
fn build_var_ref_replacement_preserves_array_index_in_braced_form() {
    // `${arr(idx)}` is recorded against base `arr`, so renaming it
    // must keep the index inside the braces: `${data(idx)}`.
    let src = "${arr(idx)}  ${ns::a(k)}";
    assert_eq!(
        build_var_ref_replacement(
            src,
            tcl_lexer::Span::new(0, 11),
            "data",
            tcl_lexer::LexerConfig::default()
        )
        .unwrap(),
        "${data(idx)}"
    );
    assert_eq!(
        build_var_ref_replacement(
            src,
            tcl_lexer::Span::new(13, 23),
            "b",
            tcl_lexer::LexerConfig::default()
        )
        .unwrap(),
        "${ns::b(k)}"
    );
}

#[test]
fn original_reference_replacement_preserves_closed_elements_and_scalar_parentheses() {
    // Implementation contract: naming.core.selected-variable-cursor-syntax
    // docs/design/analysis/name-resolution-proofs/selected-variable-cursor-syntax.md
    for style in [
        tcl_dialect::BracedVarStyle::FirstClose,
        tcl_dialect::BracedVarStyle::Tcl9Nesting,
    ] {
        let config = tcl_lexer::LexerConfig {
            braced_var: style,
            ..Default::default()
        };
        for (source, expected) in [
            ("${scalar(open}", "${new}"),
            ("${scalar(open)tail}", "${new}"),
            ("${arr(first(second)}", "${new(first(second)}"),
            ("${cash$name}", "${new}"),
            ("${café🙂}", "${new}"),
            ("$arr($idx)", "$new($idx)"),
            ("${ns::arr(k)}", "${ns::new(k)}"),
        ] {
            let span = tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap());
            assert_eq!(
                build_var_ref_replacement(source, span, "new", config).as_deref(),
                Some(expected)
            );
        }
        for source in ["${missing", "$a tail", "${a}(tail)", "not_a_reference"] {
            let span = tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap());
            assert!(build_var_ref_replacement(source, span, "new", config).is_none());
        }
        assert!(
            build_var_ref_replacement("$a", tcl_lexer::Span::new(0, 9), "new", config).is_none()
        );
    }
    assert_eq!(split_array_suffix("scalar(open"), ("scalar(open", ""));
    assert_eq!(
        split_array_suffix("scalar(open)tail"),
        ("scalar(open)tail", "")
    );
}

#[test]
fn original_logical_rename_keeps_scalar_parentheses_and_literal_dollar_names() {
    // Implementation contract: naming.core.selected-variable-cursor-syntax
    // docs/design/analysis/name-resolution-proofs/selected-variable-cursor-syntax.md
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    for name in [
        "scalar(open",
        "scalar(open)tail",
        "cash$name",
        "$n",
        "café🙂",
    ] {
        let source = format!("set {{{name}}} 1\nputs ${{{name}}}\n");
        let input = ResolvedAnalysisInput::new(profile, profile, context.clone(), config);
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(&source, profile.name)
            .clone();
        assert_eq!(analysis.resolved_input.as_ref(), Some(&input));
        assert!(analysis.allows_lexical_declaration_advice());
        assert!(
            rename_with_diagnosis(&source, profile, 1, 7, "new", &analysis, None).is_err(),
            "{name}"
        );
        let proposal = conditional_variable_rename_with_diagnosis(&source, 1, 7, "new", &analysis)
            .unwrap_or_else(|error| panic!("{name}: {error:?}"));
        let width = u32::try_from(name.encode_utf16().count()).unwrap();
        assert_eq!(
            proposal.edits,
            vec![
                TextEdit {
                    range: LspRange {
                        start_line: 0,
                        start_character: 4,
                        end_line: 0,
                        end_character: 6 + width,
                    },
                    new_text: "{new}".to_owned(),
                },
                TextEdit {
                    range: LspRange {
                        start_line: 1,
                        start_character: 7,
                        end_line: 1,
                        end_character: 7 + width,
                    },
                    new_text: "new".to_owned(),
                },
            ],
            "{name}"
        );
        assert_eq!(
            proposal.generated_source, "set {new} 1\nputs ${new}\n",
            "{name}"
        );
        assert_eq!(
            apply_edits(&source, &proposal.edits),
            proposal.generated_source
        );
        assert_eq!(
            rename(
                &(source.clone() + " "),
                profile,
                1,
                7,
                "new",
                &analysis,
                None
            ),
            [] as [crate::rename::TextEdit; 0]
        );
        assert!(
            conditional_variable_rename_with_diagnosis(
                &(source.clone() + " "),
                1,
                7,
                "new",
                &analysis,
            )
            .is_err(),
            "{name}"
        );
    }
}

// safety gating

#[test]
fn is_safe_symbol_name_accepts_canonical_identifiers() {
    assert!(is_safe_symbol_name("foo"));
    assert!(is_safe_symbol_name("Foo"));
    assert!(is_safe_symbol_name("_underscore"));
    assert!(is_safe_symbol_name("a1"));
    assert!(is_safe_symbol_name("snake_case_42"));
}

#[test]
fn is_safe_symbol_name_rejects_invalid_shapes() {
    assert!(!is_safe_symbol_name(""), "empty name should be invalid");
    assert!(!is_safe_symbol_name("1leading_digit"));
    assert!(!is_safe_symbol_name("has space"));
    assert!(!is_safe_symbol_name("has-dash"));
    assert!(!is_safe_symbol_name("has::colon"));
    assert!(!is_safe_symbol_name("with$dollar"));
    // Tcl-valid but rename-rejects (forces editor to refuse
    // partially-edited symbols).
    assert!(!is_safe_symbol_name("dotted.name"));
}

#[test]
fn rename_returns_empty_for_unsafe_new_name() {
    let src = "proc greet {} {}\ngreet\n";
    let analysis = analyse(src);
    // Whitespace in the new name fails the shape gate.
    assert_eq!(
        rename(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            "bad name",
            &analysis,
            None
        ),
        [] as [crate::rename::TextEdit; 0]
    );
    // Leading digit fails.
    assert_eq!(
        rename(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            "1lead",
            &analysis,
            None
        ),
        [] as [crate::rename::TextEdit; 0]
    );
    // Dash fails.
    assert_eq!(
        rename(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            0,
            6,
            "with-dash",
            &analysis,
            None
        ),
        [] as [crate::rename::TextEdit; 0]
    );
}

#[test]
fn rename_var_also_rejects_unsafe_new_name() {
    // The shape gate applies to variable renames too.
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    assert_eq!(
        rename(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            7,
            "bad name",
            &analysis,
            None
        ),
        [] as [crate::rename::TextEdit; 0]
    );
}

#[test]
fn rename_proc_to_builtin_command_name_blocked() {
    // Renaming `greet` to `puts` would shadow the built-in
    // `puts` command — the safety gate must reject the
    // rename when a registry is provided.
    let src = "proc greet {} {}\ngreet\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        "puts",
        &analysis,
        Some(&registry),
    );
    assert!(
        edits.is_empty(),
        "rename to built-in `puts` must produce no edits, got {edits:?}",
    );
}

#[test]
fn rename_proc_to_non_builtin_succeeds_with_registry() {
    // Renaming `greet` to a non-built-in name should still
    // succeed when the registry is provided.
    let src = "proc greet {} {}\ngreet\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        6,
        "salut",
        &analysis,
        Some(&registry),
    );
    assert!(
        !edits.is_empty(),
        "rename to non-built-in `salut` should produce edits",
    );
    assert!(edits.iter().all(|e| e.new_text == "salut"));
}

#[test]
fn rename_var_to_builtin_name_allowed() {
    // The built-in-shadow gate is proc-only.  Variable
    // names live in a separate namespace and can use any
    // shape-valid identifier — including `puts`.
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        "puts",
        &analysis,
        Some(&registry),
    );
    assert!(
        !edits.is_empty(),
        "variable rename to `puts` should succeed (different namespace)",
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

// method rename

#[test]
fn rename_method_at_decl_rewrites_decl_and_calls() {
    // Method declared on line 1, dispatched via `my greet` from line 2's
    // body twice — should rewrite the declaration plus both call sites.
    // A bare `greet` call is never valid `TclOO` dispatch (only `my
    // greet` reaches the method), so the fixture must use `my`.
    let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `greet` declaration (line 1 col 11).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        "salute",
        &analysis,
        None,
    );
    assert!(!edits.is_empty(), "{edits:?}");
    // All three sites should be present.
    assert!(edits.len() >= 3, "{edits:?}");
    for e in &edits {
        assert_eq!(e.new_text, "salute");
    }
}

#[test]
fn rename_method_rewrites_a_my_dispatch_call_inside_a_switch_arm() {
    // The `ticklecharts::chart::Add` dispatcher shape — `switch
    // ... { barSeries { my AddBarSeries {*}$args } ... }`. `rename`
    // reaches `my`-dispatch call sites via `references::
    // method_references_for_class` (`scan_my_method_region`), so this
    // exercises the same resolver as
    // `references_for_method_reach_a_my_dispatch_call_inside_a_switch_arm`
    // end-to-end through rename.
    let src = "oo::class create widget {\n    method bar {} { return \"bar-value\" }\n    method dispatch {args} {\n        switch -exact -- [lindex $args 0] {\n            bar { my bar {*}[lrange $args 1 end] }\n        }\n    }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `bar` declaration (line 1, col 11).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        "baz",
        &analysis,
        None,
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {edits:?}");
    assert!(
        lines.contains(&4),
        "the my bar call site inside the switch arm must be rewritten too: {edits:?}"
    );
    assert!(edits.iter().all(|e| e.new_text == "baz"));
}

#[test]
fn rename_method_rewrites_my_dispatch_when_class_extended_via_separate_oo_define() {
    // `Gadget` is created via `oo::class create` with no body; every
    // method (including the `my Helper` call site) is added via a
    // *separate*, later `oo::define Gadget { ... }` block — the
    // `ticklecharts::chart` shape. Renaming from the `Helper` declaration
    // must rewrite the `my Helper` call site living in that separate
    // block too, not silently skip it (which would leave the program
    // calling a now-nonexistent method after the rename).
    let src = "oo::class create Gadget {\n    variable _x\n}\noo::define Gadget {\n    method Helper {} { return hi }\n    method Caller {} { my Helper }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `Helper` declaration (line 4, col 11).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        11,
        "Assist",
        &analysis,
        None,
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(lines.contains(&4), "decl missing: {edits:?}");
    assert!(
        lines.contains(&5),
        "the my Helper call site inside the separate oo::define block must be rewritten too: {edits:?}"
    );
    assert!(edits.iter().all(|e| e.new_text == "Assist"));
}

#[test]
fn rename_method_does_not_rewrite_next_dispatch() {
    // `next` is a keyword, not the method name.  Find-references counts the
    // super-dispatch as a reference, but a rename must never rewrite it —
    // the reference paths add `next` sites, the rename path must not.
    let src = "oo::class create Base {\n    method greet {} {}\n}\noo::class create Sub {\n    superclass Base\n    method greet {} { next }\n}\n";
    let analysis = analyse(src);
    // Cursor on `Sub::greet`'s declaration (line 5, col 11).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        5,
        11,
        "salute",
        &analysis,
        None,
    );
    assert!(!edits.is_empty(), "{edits:?}");
    // The declaration (line 5, col 11) is rewritten; nothing in the `{ next
    // }` body region (col >= 20 on line 5) is.
    assert!(
        edits
            .iter()
            .all(|e| !(e.range.start_line == 5 && e.range.start_character >= 20)),
        "a rename edit landed on the `next` keyword: {edits:?}",
    );
}

#[test]
fn rename_method_at_call_site_also_works() {
    let src = "oo::class create C {\n    method greet {} {}\n    method twice {} { my greet ; my greet }\n}\n";
    let analysis = analyse(src);
    // Cursor on the first `greet` call site (line 2 col 25, after `my `).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        25,
        "salute",
        &analysis,
        None,
    );
    assert!(edits.len() >= 3, "{edits:?}");
    for e in &edits {
        assert_eq!(e.new_text, "salute");
    }
}

/// A method rename must not run a bare-head scan over every command
/// invocation whose head text matches the renamed method's name —
/// including an unrelated builtin call that merely happens to share it.
/// A `TclOO` method is never a bare-callable command (only `my <method>`
/// dispatches it), so a bare call is always a call to something else.
#[test]
fn rename_method_does_not_rewrite_unrelated_bare_command_with_same_name() {
    let src =
        "oo::class create C {\n    method format {} {}\n    method show {} { format %d 1 }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `format` method's declaration (line 1, col 11).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        "render",
        &analysis,
        None,
    );
    // Only the declaration is renamed — the bare `format %d 1` call
    // inside `show` invokes the builtin, not this method.
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(edits[0].range.start_line, 1);
}

/// Regression: methods, classmethods, and properties are independent
/// tables, so a name shared by more than one (rare, but `TclOO` never
/// merges them) must resolve to whichever declaration the cursor
/// actually sits on — not always the same one by blind priority, which
/// would silently rename the wrong declaration.
#[test]
fn rename_disambiguates_property_and_method_sharing_a_name_by_cursor() {
    let src = "oo::class create C {\n    property color\n    method color {} { return c }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
    // Cursor on the *property* declaration (line 1, col 13).
    let property_edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        1,
        13,
        "shade",
        &analysis,
        None,
    );
    assert_eq!(
        property_edits.len(),
        1,
        "expected only the property decl: {property_edits:?}"
    );
    assert_eq!(property_edits[0].range.start_line, 1, "{property_edits:?}");
    // Cursor on the *method* declaration (line 2) picks the method
    // instead, leaving the same-named property untouched.
    let method_edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        2,
        11,
        "shade",
        &analysis,
        None,
    );
    assert!(
        method_edits.iter().any(|e| e.range.start_line == 2),
        "{method_edits:?}"
    );
    assert!(
        method_edits.iter().all(|e| e.range.start_line != 1),
        "must not touch the same-named property: {method_edits:?}"
    );
}

/// Regression: an instance `method` and a `classmethod` sharing a name
/// (rare, but `TclOO` keeps them in independent tables, so it's legal)
/// must never cross-link during rename — `my <word>` dispatch scope
/// depends on which table the *caller's own body* belongs to (`self` is
/// the class object inside a `classmethod`, the instance everywhere
/// else), so renaming the classmethod must not touch a `my greet` site
/// that actually dispatches to the unrelated instance method — doing so
/// would corrupt otherwise-working code (the call would then reference
/// a name that no longer exists on the instance).
#[test]
fn rename_classmethod_does_not_corrupt_unrelated_instance_method_call_site() {
    let src = "oo::class create C {\n    method greet {} {}\n    classmethod greet {} {}\n    method twice {} { my greet }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
    // Cursor on the *classmethod* declaration (line 2, col 16).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
        2,
        16,
        "hail",
        &analysis,
        None,
    );
    assert_eq!(
        edits.len(),
        1,
        "only the classmethod's own decl — `twice`'s `my greet` targets \
             the unrelated instance method and must be untouched: {edits:?}"
    );
    assert_eq!(edits[0].range.start_line, 2, "{edits:?}");
}

// `method_target_with_access`'s receiver-side split.

/// TP: a bare `ClassName member` naming a **class-side** member resolves as
/// a classmethod, so downstream cross-file resolution reads the
/// class-object side's tables.
///
/// `receiver_instance_class`'s last resort is "this bare word names a
/// class", so without the class-side gate ahead of it every bare
/// `ClassName member` classifies instance-side and no class-side member
/// can reach the cross-file class-command dispatch, whatever the
/// workspace index holds.
#[test]
fn method_target_classifies_a_bare_class_receiver_as_a_classmethod() {
    let src =
        "oo::class create Widget {\n    self { method make {} { return 1 } }\n}\nWidget make\n";
    let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
    assert_eq!(
        method_target_with_access(src, 3, 8, &analysis),
        Some((
            "::Widget".to_owned(),
            "make".to_owned(),
            true,
            crate::workspace_index::MethodAccess::External
        )),
    );
}

/// TN: an `$obj member` receiver is an object handle, never a class, so it
/// stays instance-side — the gate that keeps the reordering above confined
/// to the shape it is about.
#[test]
fn method_target_keeps_an_object_handle_receiver_instance_side() {
    let src = "oo::class create Widget {\n    method make {} { return 1 }\n}\nset w [Widget new]\n$w make\n";
    let analysis = analyse(src);
    assert_eq!(
        method_target_with_access(src, 4, 4, &analysis),
        Some((
            "::Widget".to_owned(),
            "make".to_owned(),
            false,
            crate::workspace_index::MethodAccess::External
        )),
    );
}

/// TN: a bare class receiver whose class-object side has **no** such member
/// falls through to the instance branch unchanged.
#[test]
fn method_target_falls_through_when_the_class_side_lacks_the_member() {
    let src = "oo::class create Widget {\n    method make {} { return 1 }\n}\nWidget make\n";
    let analysis = analyse(src);
    assert_eq!(
        method_target_with_access(src, 3, 8, &analysis),
        Some((
            "::Widget".to_owned(),
            "make".to_owned(),
            false,
            crate::workspace_index::MethodAccess::External
        )),
    );
}

/// TN (CRITICAL): a stock `self method` is **not inherited** — `Gadget
/// make` against a parent's `self method make` errors `unknown method
/// "make": must be create, destroy or new` on tclsh 8.6 and 9.0.4 alike —
/// so the subclass receiver must not resolve to the parent's class-side
/// member. `classmethod_dispatch_class` applies the same
/// `is_self_method` rule the in-document provider does.
#[test]
fn method_target_does_not_inherit_a_parents_self_method() {
    let src = concat!(
        "oo::class create Widget {\n",
        "    self { method make {} { return 1 } }\n",
        "}\n",
        "oo::class create Gadget {\n",
        "    superclass Widget\n",
        "}\n",
        "Gadget make\n",
    );
    let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
    let target = method_target_with_access(src, 6, 8, &analysis);
    assert!(
        target.is_none_or(|(_, _, is_classmethod, _)| !is_classmethod),
        "an inherited `self method` is not the subclass's classmethod",
    );
}

// `method_target_with_access`'s class-body fallback — the idx-113 link
// gate.

/// TP: the cursor on a method's own declaration name is the member.
#[test]
fn method_target_fires_on_the_members_own_declaration_name() {
    let src = "oo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} { return [foo 42] }\n}\n";
    let analysis = analyse(src);
    let target = method_target_with_access(src, 1, 11, &analysis);
    assert_eq!(
        target,
        Some((
            "::Widget".to_owned(),
            "foo".to_owned(),
            false,
            crate::workspace_index::MethodAccess::Internal
        )),
        "cursor on the `foo` declaration must resolve to Widget's `foo`"
    );
}

/// TP: a bareword sibling call that `link` genuinely made callable is a
/// reference to the member — tclsh9.0-verified (`link foo` in the
/// constructor makes `[foo 42]` inside another method dispatch to
/// `Widget`'s `foo`; `link` does not exist in 8.6's `TclOO`).
#[test]
fn method_target_fires_on_a_linked_bareword_call() {
    let src = "oo::class create Widget {\n    constructor {} { link foo }\n    method foo {x} { return $x }\n    method bar {} { return [foo 42] }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
    let target = method_target_with_access(src, 3, 28, &analysis);
    assert_eq!(
        target,
        Some((
            "::Widget".to_owned(),
            "foo".to_owned(),
            false,
            crate::workspace_index::MethodAccess::Internal
        )),
        "a linked bareword sibling call must resolve to the member"
    );
}

/// FP guard: an **un-linked** bareword sibling call names
/// nothing — real Tcl raises `invalid command name "foo"` there
/// (tclsh9.0-verified) — so the resolver must abstain rather than hand
/// the word to the workspace resolver, which answers it
/// non-deterministically.
#[test]
fn method_target_abstains_on_an_unlinked_bareword_sibling_call() {
    let src = "oo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} { return [foo 42] }\n}\n";
    let analysis = analyse(src);
    assert_eq!(
        method_target_with_access(src, 2, 28, &analysis),
        None,
        "an un-linked bareword sibling call is not a reference to the method"
    );
}

/// FP guard: a *parameter* that happens to share a sibling method's name
/// is not that method either — the old fallback fired on any
/// member-name-shaped word anywhere in the class body span.
#[test]
fn method_target_abstains_on_a_parameter_sharing_a_method_name() {
    let src = "oo::class create Widget {\n    method foo {x} { return $x }\n    method bar {foo} { return $foo }\n}\n";
    let analysis = analyse(src);
    assert_eq!(
        method_target_with_access(src, 2, 16, &analysis),
        None,
        "a parameter word must not resolve to the same-named method"
    );
}

// property rename

/// Regression: renaming a property must rewrite every `my <property>`
/// call site alongside the declaration.  Properties have no `$obj
/// prop` dispatch and no override family — but they *are* read via
/// `my <property>` inside the class's own methods, and a bare
/// `<property>` occurrence is never valid `TclOO` dispatch (only `my
/// <property>` reads it), so the bare-head scan `rename_method` also
/// runs never matches this shape — without the dedicated `my`-aware
/// scan, only the declaration was renamed, leaving call sites pointing
/// at the old name.
#[test]
fn rename_property_rewrites_decl_and_my_dispatch_call_sites() {
    let src = "oo::class create C {\n    property color\n    method describe {} {\n        return [my color]\n    }\n}\n";
    // `property` is Tcl 9.0+ — the shared `analyse()` helper fixes the
    // dialect at 8.6, so this test analyses at 9.0 directly.
    let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
    // Cursor on the `color` property declaration (line 1, col 13).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        1,
        13,
        "shade",
        &analysis,
        None,
    );
    assert_eq!(edits.len(), 2, "decl + `my color` call site: {edits:?}");
    assert!(edits.iter().all(|e| e.new_text == "shade"));
    assert_eq!(edits[0].range.start_line, 1);
    assert_eq!(edits[1].range.start_line, 3);
}

/// FN→TP: a `my <property>` read nested
/// inside `if` control flow is renamed too — the property rename path
/// shares `scan_my_method_sites`'s control-flow recursion with methods.
#[test]
fn rename_property_rewrites_control_flow_nested_call_site() {
    let src = "oo::class create C {\n    property color\n    method describe {} {\n        if {1} {\n            return [my color]\n        }\n    }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.0").clone();
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        1,
        13,
        "shade",
        &analysis,
        None,
    );
    assert_eq!(
        edits.len(),
        2,
        "decl + nested `my color` call site: {edits:?}"
    );
    assert!(edits.iter().all(|e| e.new_text == "shade"));
    assert_eq!(edits[1].range.start_line, 4);
}

#[test]
fn rename_method_skipped_outside_class_body() {
    // Cursor on bare `greet` outside the class — no class
    // method named `greet` is visible, so the rename
    // returns empty.
    let src = "oo::class create C {\n    method greet {} {}\n}\ngreet\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        3,
        2,
        "salute",
        &analysis,
        None,
    );
    assert!(edits.is_empty(), "{edits:?}");
}

/// Renaming a method must
/// rewrite a `my method` call site nested inside `if` / `switch`
/// control flow, not just a top-level or `[...]`-nested call — `rename`
/// delegates to the same `method_references_for_class` resolver
/// `references`/the code lens use, so the fix there covers rename too.
#[test]
fn rename_method_rewrites_control_flow_nested_my_dispatch() {
    let src = "oo::class create C {\n    method getOptions {k} { return $k }\n    method get {k} {\n        if {1} {\n            switch -- $k {\n                default {\n                    my getOptions $k\n                }\n            }\n        }\n    }\n}\n";
    let analysis = analyse(src);
    // Cursor on the `getOptions` declaration (line 1, col 11).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        "fetchOptions",
        &analysis,
        None,
    );
    assert_eq!(edits.len(), 2, "decl + nested `my` site: {edits:?}");
    assert!(edits.iter().all(|e| e.new_text == "fetchOptions"));
    assert_eq!(edits[0].range.start_line, 1);
    // The nested call site, six lines further down.
    assert_eq!(edits[1].range.start_line, 6);
}

#[test]
fn rename_method_rejects_unsafe_new_name() {
    let src = "oo::class create C {\n    method greet {} {}\n}\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        "1bad",
        &analysis,
        None,
    );
    assert!(edits.is_empty(), "{edits:?}");
}

#[test]
fn prepare_rename_returns_range_for_method() {
    let src = "oo::class create C {\n    method greet {} {}\n}\n";
    let analysis = analyse(src);
    let p = prepare_rename(src, 1, 11, &analysis).expect("expected prepare_rename on method");
    assert_eq!(p.placeholder, "greet");
}

// external $obj method rename

#[test]
fn rename_method_from_decl_rewrites_external_obj_sites() {
    // Renaming `bark` from its declaration also rewrites the
    // external `$d bark` / `[$d bark]` call sites.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\nputs [$d bark]\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        "yip",
        &analysis,
        None,
    );
    // Declaration + 2 external sites = 3 edits, all "yip".
    assert!(edits.len() >= 3, "{edits:?}");
    for e in &edits {
        assert_eq!(e.new_text, "yip");
    }
    // One edit on line 4 (`$d bark`) and one on line 5
    // (`[$d bark]`).
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(lines.contains(&4) && lines.contains(&5), "{edits:?}");
}

#[test]
fn rename_method_rewrites_bare_created_instance_command_site() {
    // `Dog create rex` binds `rex` as an object command, so renaming
    // `bark` must also rewrite the bare `rex bark` dispatch — the same
    // shared resolver drives the peek, the lens, and the rename.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        "yip",
        &analysis,
        None,
    );
    for e in &edits {
        assert_eq!(e.new_text, "yip");
    }
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(lines.contains(&1), "decl not renamed: {edits:?}");
    assert!(
        lines.contains(&4),
        "bare `rex bark` site not renamed: {edits:?}"
    );
}

#[test]
fn rename_classmethod_from_call_site_rewrites_decl_and_inheriting_subclass_call() {
    // TP — renaming `find` with the cursor on the
    // `ActiveRecord find foo bar` call site must rewrite the
    // declaration, that same call, AND the inherited `Table find`
    // call — three edits, none missed (rename.rs has no separate
    // scan of its own; it delegates fully to
    // `references::method_references_for_class`, so this exercises
    // Part 2 + Part 3 together end-to-end).
    let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\noo::class create Table {\n    superclass ActiveRecord\n}\nTable find foo bar\nActiveRecord find foo bar\n";
    let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
    // Cursor on `find` in `ActiveRecord find foo bar` (line 7, col 13).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
        7,
        13,
        "lookup",
        &analysis,
        None,
    );
    assert_eq!(edits.len(), 3, "{edits:?}");
    for e in &edits {
        assert_eq!(e.new_text, "lookup");
    }
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(lines.contains(&1), "decl not renamed: {edits:?}");
    assert!(lines.contains(&6), "Table find call not renamed: {edits:?}");
    assert!(
        lines.contains(&7),
        "ActiveRecord find call not renamed: {edits:?}"
    );
}

#[test]
fn rename_method_from_cursor_on_bare_obj_command_call_site() {
    // Symmetry: triggering rename with the cursor ON the
    // `bark` token of a bare `rex bark` dispatch rewrites the declaration
    // and the call site — not only the declaration-triggered rename.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
    let analysis = analyse(src);
    // Line 4 `rex bark` — cursor on `bark` (col 4).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        4,
        "yip",
        &analysis,
        None,
    );
    assert!(edits.len() >= 2, "{edits:?}");
    for e in &edits {
        assert_eq!(e.new_text, "yip");
    }
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(lines.contains(&1), "decl not renamed: {edits:?}");
    assert!(lines.contains(&4), "call site not renamed: {edits:?}");
}

#[test]
fn rename_method_from_external_call_site() {
    // Triggering rename from the external `$d bark` site
    // rewrites the declaration + all sites.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\n";
    let analysis = analyse(src);
    // Cursor on `bark` in `$d bark` (line 4, col 3).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        3,
        "yip",
        &analysis,
        None,
    );
    assert!(edits.len() >= 2, "{edits:?}");
    for e in &edits {
        assert_eq!(e.new_text, "yip");
    }
    // Declaration (line 1) is rewritten too.
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(lines.contains(&1), "decl not renamed: {edits:?}");
}

#[test]
fn rename_method_external_rewrites_proc_body_sites() {
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nproc f {} { $d bark }\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        "yip",
        &analysis,
        None,
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    // Declaration (1) + proc-body call (4).
    assert!(lines.contains(&1) && lines.contains(&4), "{edits:?}");
}

#[test]
fn rename_var_from_definition_site_resolves_without_dollar() {
    // Cursor on the `x` in `set x 42` (no `$`) — the def-site
    // resolver must find the variable and rewrite decl + reads.
    let src = "set x 42\nputs $x\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        4,
        "newvar",
        &analysis,
        None,
    );
    let texts: std::collections::HashSet<&str> =
        edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(texts.contains("newvar"), "decl edit missing: {edits:?}");
    assert!(texts.contains("$newvar"), "ref edit missing: {edits:?}");
}

#[test]
fn rename_var_preserves_namespace_qualifier_at_decl() {
    let src = "set myns::count 0\nputs $myns::count\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        0,
        10,
        "total",
        &analysis,
        None,
    );
    let texts: std::collections::HashSet<&str> =
        edits.iter().map(|e| e.new_text.as_str()).collect();
    assert!(texts.contains("myns::total"), "decl ns lost: {edits:?}");
    assert!(texts.contains("$myns::total"), "ref ns lost: {edits:?}");
}

#[test]
fn rename_var_rejected_on_same_scope_collision() {
    let src = "proc demo {} {\n    set x 1\n    set y 2\n    puts $x\n}\n";
    let analysis = analyse(src);
    // Rename `x` (read site, line 3) to `y` which already exists in scope.
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        3,
        10,
        "y",
        &analysis,
        None,
    );
    assert!(edits.is_empty(), "collision not rejected: {edits:?}");
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

// method rename across overrides

#[test]
fn rename_method_renames_subclass_override() {
    // `Animal::speak` overridden by `Dog::speak`.  Renaming from the
    // base declaration must rewrite *both* declarations — leaving the
    // override behind would silently break polymorphic dispatch.
    let src = "oo::class create Animal {\n\
                       method speak {} { return x }\n\
                   }\n\
                   oo::class create Dog {\n\
                       superclass Animal\n\
                       method speak {} { return woof }\n\
                   }\n";
    let analysis = analyse(src);
    // Cursor on `speak` in Animal's declaration (line 1 col 7).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        "vocalise",
        &analysis,
        None,
    );
    assert!(edits.iter().all(|e| e.new_text == "vocalise"));
    // Both declarations rewritten: line 1 (Animal) and line 5 (Dog).
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(
        lines.contains(&1) && lines.contains(&5),
        "expected both Animal (l1) and Dog override (l5) renamed; got {edits:?}",
    );
}

#[test]
fn rename_method_renames_from_subclass_up_to_base() {
    // Symmetric to the above: renaming from the *subclass* override
    // must also rewrite the base declaration.
    let src = "oo::class create Animal {\n\
                       method speak {} { return x }\n\
                   }\n\
                   oo::class create Dog {\n\
                       superclass Animal\n\
                       method speak {} { return woof }\n\
                   }\n";
    let analysis = analyse(src);
    // Cursor on `speak` in Dog's declaration (line 5 col 7).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        5,
        7,
        "vocalise",
        &analysis,
        None,
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(
        lines.contains(&1) && lines.contains(&5),
        "expected base (l1) renamed from subclass override; got {edits:?}",
    );
}

#[test]
fn rename_method_renames_sibling_overrides_of_common_base() {
    // Two siblings override a common base method.  Renaming any one
    // must rewrite the base and *both* siblings — they are one
    // polymorphic name reachable via the base's static type.
    let src = "oo::class create Shape {\n\
                       method area {} { return 0 }\n\
                   }\n\
                   oo::class create Circle {\n\
                       superclass Shape\n\
                       method area {} { return 3 }\n\
                   }\n\
                   oo::class create Square {\n\
                       superclass Shape\n\
                       method area {} { return 4 }\n\
                   }\n";
    let analysis = analyse(src);
    // Cursor on `area` in Circle (line 5 col 7).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        5,
        7,
        "measure",
        &analysis,
        None,
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    // Shape (l1), Circle (l5), Square (l9) all rewritten.
    assert!(
        lines.contains(&1) && lines.contains(&5) && lines.contains(&9),
        "expected Shape + both siblings renamed; got {edits:?}",
    );
}

#[test]
fn rename_method_leaves_unrelated_same_name_method_untouched() {
    // Two classes in *disjoint* hierarchies both define `run`.
    // Renaming one must not touch the other.
    let src = "oo::class create Engine {\n\
                       method run {} { return e }\n\
                   }\n\
                   oo::class create Task {\n\
                       method run {} { return t }\n\
                   }\n";
    let analysis = analyse(src);
    // Cursor on `run` in Engine (line 1 col 7).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        7,
        "start",
        &analysis,
        None,
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(
        lines.contains(&1),
        "Engine::run should be renamed: {edits:?}"
    );
    assert!(
        !lines.contains(&4),
        "unrelated Task::run must NOT be renamed; got {edits:?}",
    );
}

#[test]
fn rename_inherited_method_from_external_obj_site() {
    // `$d speak` where `d` is a `Dog` that *inherits* `speak` from
    // `Animal` (no override).  Renaming from the external call site
    // must rewrite the base declaration.
    let src = "oo::class create Animal {\n\
                       method speak {} { return x }\n\
                   }\n\
                   oo::class create Dog {\n\
                       superclass Animal\n\
                   }\n\
                   set d [Dog new]\n\
                   $d speak\n";
    let analysis = analyse(src);
    // Cursor on `speak` in `$d speak` (line 7 col 3).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        7,
        3,
        "vocalise",
        &analysis,
        None,
    );
    assert!(
        !edits.is_empty(),
        "inherited-method rename produced nothing"
    );
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(
        lines.contains(&1),
        "expected base Animal::speak decl (l1) renamed; got {edits:?}",
    );
}

#[test]
fn rename_inherited_method_rewrites_subclass_instance_obj_sites() {
    // Renaming an inherited method from the *base declaration* must also
    // rewrite `$d method` sites where `$d` is an instance of a subclass
    // that inherits (does not override) it — an exact-class-equality
    // filter would drop these, leaving them pointing at the old name.
    let src = "oo::class create Animal {\n\
                       method speak {} { return x }\n\
                   }\n\
                   oo::class create Dog {\n\
                       superclass Animal\n\
                   }\n\
                   set d [Dog new]\n\
                   $d speak\n";
    let analysis = analyse(src);
    // Cursor on `speak` in Animal's declaration (line 1 col 11).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        "vocalise",
        &analysis,
        None,
    );
    assert!(edits.iter().all(|e| e.new_text == "vocalise"));
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(
        lines.contains(&1) && lines.contains(&7),
        "expected decl (l1) + inheriting-subclass instance site (l7); got {edits:?}",
    );
}

#[test]
fn rename_inherited_method_rewrites_pure_inheritor_my_sites() {
    // A subclass that inherits `speak` (no override) calls `my speak`
    // from one of its own method bodies.  That call dispatches to the
    // base definition, so renaming the base must rewrite it — even
    // though the subclass is not itself a rename family member.
    let src = "oo::class create Animal {\n\
                       method speak {} { return x }\n\
                   }\n\
                   oo::class create Dog {\n\
                       superclass Animal\n\
                       method describe {} { my speak }\n\
                   }\n";
    let analysis = analyse(src);
    // Cursor on `speak` in Animal's declaration (line 1 col 11).
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        "vocalise",
        &analysis,
        None,
    );
    assert!(edits.iter().all(|e| e.new_text == "vocalise"));
    let lines: Vec<u32> = edits.iter().map(|e| e.range.start_line).collect();
    assert!(
        lines.contains(&1) && lines.contains(&5),
        "expected decl (l1) + `my speak` in inheriting subclass (l5); got {edits:?}",
    );
}

/// TP — the `export` list is part of the rename.
///
/// Oracle (tclsh 9.0.4 and 8.6.16, identical): with `method Foo` +
/// `export Foo`, `[$a Foo]` prints `foo`.  Rename `Foo` → `Bar` touching
/// only the declaration and the call site, leaving `export Foo` behind,
/// and both interpreters answer
/// `unknown method "Bar": must be destroy` — the renamed method is no
/// longer exported.  A `method` whose name begins with an upper-case
/// letter is unexported by default (probed on 9.0.4: `$a Foo` errors,
/// `$a bar` works), which is exactly why the corpus writes the list.
#[test]
fn tp_rename_method_rewrites_its_export_list() {
    let src = "oo::class create A {\n    method Foo {} { return 1 }\n    export Foo\n}\nset a [A new]\nputs [$a Foo]\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        1,
        12,
        "Bar",
        &analysis,
        None,
    );
    let applied = apply_edits(src, &edits);
    assert!(
        applied.contains("export Bar"),
        "the export list must be renamed with the method: {applied}"
    );
    assert!(
        !applied.contains("Foo"),
        "no stale name may survive: {applied}"
    );
}

/// TP — an `export` written in a separate `oo::define` block is found
/// through the definer grammar, not the class's own body span.
#[test]
fn tp_rename_method_rewrites_an_export_in_a_separate_oo_define_block() {
    let src = "oo::class create A {\n    method Foo {} { return 1 }\n}\noo::define A {\n    export Foo\n}\n";
    let analysis = analyse(src);
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        1,
        12,
        "Bar",
        &analysis,
        None,
    );
    let applied = apply_edits(src, &edits);
    assert!(
        applied.contains("export Bar"),
        "an `oo::define` export list must rename too: {applied}"
    );
}

/// TP + TN, the object-command half.
///
/// Oracle (tclsh 9.0.4 / 8.6.16, identical): `::a::Factory create rex`
/// and `::b::Widget create rex` bind two different commands; `rex make`
/// inside `::a` prints `a-made` and inside `::b` prints `b-made`, and
/// `::a::rex make` / `::b::rex make` both work from the top level.
/// Renaming `::b::Widget::make` → `produce` and also rewriting `::a`'s
/// call site makes both interpreters fail with
/// `unknown method "produce": must be destroy or make` at `rex produce`
/// inside `::a` — which is exactly what the LSP emitted before this fix.
#[test]
fn tn_object_command_dispatch_is_scoped_to_its_creation_namespace() {
    let src = "namespace eval ::a {\n\
                   \x20   oo::class create Factory {\n\
                   \x20       method make {} { return \"a-made\" }\n\
                   \x20   }\n\
                   \x20   Factory create rex\n\
                   \x20   puts [rex make]\n\
                   }\n\
                   namespace eval ::b {\n\
                   \x20   oo::class create Widget {\n\
                   \x20       method make {} { return \"b-made\" }\n\
                   \x20   }\n\
                   \x20   Widget create rex\n\
                   \x20   puts [rex make]\n\
                   }\n";
    let analysis = analyse(src);
    // Cursor on `make` in `::b::Widget`'s declaration.
    let edits = rename(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        9,
        15,
        "produce",
        &analysis,
        None,
    );
    let applied = apply_edits(src, &edits);
    assert!(
        applied.contains("method produce {} { return \"b-made\" }"),
        "::b's declaration must rename: {applied}"
    );
    assert!(
        applied.contains("method make {} { return \"a-made\" }"),
        "::a's declaration must be untouched: {applied}"
    );
    // `::a`'s own call site must survive verbatim; `::b`'s must rename.
    let a_call = applied
        .split("namespace eval ::b")
        .next()
        .expect("split keeps the ::a half");
    assert!(
        a_call.contains("puts [rex make]"),
        "::a's `rex make` must not be rewritten (issue #981): {applied}"
    );
    assert!(
        applied
            .split("namespace eval ::b")
            .nth(1)
            .is_some_and(|b| b.contains("puts [rex produce]")),
        "::b's own `rex make` must rename (the finding's lost-site half): {applied}"
    );
}

/// TP — the namespace-variable cell's declaration, its proc-local
/// `variable` alias, that alias's unqualified reads, and every qualified
/// occurrence rename as one unit.
///
/// Oracle (tclsh 9.0.4 / 8.6.16, identical): the script prints `1` before
/// and after the complete rename; renaming only the declaration and the
/// qualified read leaves `p` reading a cell that no longer exists.
#[test]
fn colon_variable_rename_keeps_the_empty_cell_separate() {
    let source = "set {} EMPTY; set : COLON; puts ${}; puts $:";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut analyser = tcl_compiler::analyser::Analyser::new();
        let analysis = analyser.analyse(source, dialect);
        let edits = namespace_variable_rename_edits(source, &analysis, ":::", "total");
        let applied = apply_edits(source, &edits);
        assert_eq!(
            applied, "set {} EMPTY; set total COLON; puts ${}; puts $total",
            "{dialect}"
        );
    }
}

#[test]
fn tp_namespace_variable_rename_covers_declaration_alias_and_qualified_use() {
    let src = "namespace eval ::ns {\n\
                   \x20   variable v 1\n\
                   \x20   proc p {} {\n\
                   \x20       variable v\n\
                   \x20       return $v\n\
                   \x20   }\n\
                   }\n\
                   puts $::ns::v\n\
                   puts [::ns::p]\n";
    let analysis = analyse(src);
    let edits = namespace_variable_rename_edits(src, &analysis, "::ns::v", "total");
    let applied = apply_edits(src, &edits);
    assert!(applied.contains("variable total 1"), "{applied}");
    assert!(applied.contains("        variable total\n"), "{applied}");
    assert!(applied.contains("return $total"), "{applied}");
    assert!(applied.contains("puts $::ns::total"), "{applied}");
    assert!(
        !applied.contains("$v\n"),
        "no stale read may survive: {applied}"
    );
}

#[test]
fn namespace_variable_rename_requires_the_retained_consumer_image() {
    let source = "namespace eval N {variable x 1}\nputs $::N::x\n";
    let analysis = Analyser::new().analyse(source, "tcl");
    assert!(analysis.allows_lexical_declaration_advice());
    let edits = namespace_variable_rename_edits(source, &analysis, "::N::x", "total");
    assert_eq!(
        apply_edits(source, &edits),
        "namespace eval N {variable total 1}\nputs $::N::total\n"
    );
    let changed_source = source.replace(" x 1", " y 1").replace("::N::x", "::N::y");
    assert!(
        namespace_variable_rename_edits(&changed_source, &analysis, "::N::x", "total").is_empty(),
        "retained x spans cannot rename the changed document's y"
    );
}

/// FP — `namespace upvar ::ns v
/// local` names the cell in its *third* word; `local` is an independent
/// local spelling.  Rewriting the alias token and leaving the target word
/// behind re-points the alias at a cell that no longer exists.
///
/// Oracle (tclsh 9.0.4 / 8.6.16, byte-identical):
///   before                         -> `p -> 42`, `cell -> 42`, rc 0
///   `namespace upvar ::ns v total; … $total` (alias rewritten, target
///   word left)                     -> `can't read "total": no such
///                                     variable` at `"expr {$total + 41}"`,
///                                     rc 1
///   `namespace upvar ::ns total local; … $local` (target word rewritten,
///   local spelling kept)           -> `p -> 42`, `cell -> 42`, rc 0
#[test]
fn fp_namespace_variable_rename_keeps_a_differently_named_alias_spelling() {
    let src = "namespace eval ::ns {\n    variable v 1\n}\n\
                   proc p {} {\n\
                   \x20   namespace upvar ::ns v local\n\
                   \x20   set local [expr {$local + 41}]\n\
                   \x20   return $local\n\
                   }\n";
    let analysis = analyse(src);
    let edits = namespace_variable_rename_edits(src, &analysis, "::ns::v", "total");
    let applied = apply_edits(src, &edits);
    assert!(applied.contains("variable total 1"), "{applied}");
    assert!(
        applied.contains("namespace upvar ::ns total local"),
        "the word naming the cell is the word rewritten: {applied}"
    );
    assert!(
        applied.contains("set local [expr {$local + 41}]") && applied.contains("return $local"),
        "the local spelling and its reads must survive verbatim: {applied}"
    );
    assert!(
        !applied.contains("$total"),
        "no read may be re-spelled to the cell's new name: {applied}"
    );
}

/// TN (same finding) — a *same-spelled* `namespace upvar ::ns v v` is
/// still two independent words.  Both interpreters accept either answer
/// (`namespace upvar ::ns total v; … $v` and `namespace upvar ::ns total
/// total; … $total` both print `42` on 9.0.4 and 8.6.16), so the minimal
/// edit is taken: rewrite the word that names the cell, and leave the
/// local spelling — whose meaning the rename does not change — alone.
#[test]
fn tn_namespace_variable_rename_takes_the_minimal_edit_for_a_same_spelled_alias() {
    let src = "namespace eval ::ns {\n    variable v 1\n}\n\
                   proc p {} {\n\
                   \x20   namespace upvar ::ns v v\n\
                   \x20   return $v\n\
                   }\n";
    let analysis = analyse(src);
    let edits = namespace_variable_rename_edits(src, &analysis, "::ns::v", "total");
    let applied = apply_edits(src, &edits);
    assert!(applied.contains("variable total 1"), "{applied}");
    assert!(
        applied.contains("namespace upvar ::ns total v"),
        "only the cell-naming word changes: {applied}"
    );
    assert!(
        applied.contains("return $v"),
        "the local read is unaffected by the cell's name: {applied}"
    );
}

/// TP (same finding) — `upvar #0 ::ns::v local` is the other
/// differently-named shape, and resolves the same way: the `otherVar`
/// word is qualified, so the rewrite must preserve its qualifier.
#[test]
fn tp_namespace_variable_rename_rewrites_a_global_upvar_target_word() {
    let src = "namespace eval ::ns {\n    variable v 1\n}\n\
                   proc p {} {\n\
                   \x20   upvar #0 ::ns::v local\n\
                   \x20   return $local\n\
                   }\n";
    let analysis = analyse(src);
    let edits = namespace_variable_rename_edits(src, &analysis, "::ns::v", "total");
    let applied = apply_edits(src, &edits);
    assert!(applied.contains("variable total 1"), "{applied}");
    assert!(
        applied.contains("upvar #0 ::ns::total local"),
        "the qualified target word keeps its qualifier: {applied}"
    );
    assert!(
        applied.contains("return $local"),
        "the local spelling survives: {applied}"
    );
}

/// TP (same finding) — `global ::ns::v` is the *same*-word shape: its one
/// declaration word both names the cell and introduces the local alias
/// (whose name is the cell's tail), so the rename must rewrite the word
/// **and** every unqualified read it enables.
///
/// Oracle (9.0.4 / 8.6.16): leaving the reads behind gives `can't read
/// "v": no such variable`.
#[test]
fn tp_namespace_variable_rename_rewrites_a_global_alias_and_its_reads() {
    let src = "namespace eval ::ns {\n    variable v 1\n}\n\
                   proc p {} {\n\
                   \x20   global ::ns::v\n\
                   \x20   return $v\n\
                   }\n";
    let analysis = analyse(src);
    let edits = namespace_variable_rename_edits(src, &analysis, "::ns::v", "total");
    let applied = apply_edits(src, &edits);
    assert!(applied.contains("variable total 1"), "{applied}");
    assert!(applied.contains("global ::ns::total"), "{applied}");
    assert!(
        applied.contains("return $total"),
        "the local spelling is the cell's tail, so it travels: {applied}"
    );
}

/// TN — a same-tailed cell in a *different* namespace is a different
/// variable and must be untouched (`$other::v` never searches enclosing
/// namespaces; tclsh 9.0.4 / 8.6.16 keep the two independent).
#[test]
fn tn_namespace_variable_rename_leaves_a_same_tailed_sibling_cell_alone() {
    let src = "namespace eval ::a {\n    variable v 1\n}\n\
                   namespace eval ::b {\n    variable v 2\n}\n\
                   puts $::a::v\nputs $::b::v\n";
    let analysis = analyse(src);
    let edits = namespace_variable_rename_edits(src, &analysis, "::a::v", "total");
    let applied = apply_edits(src, &edits);
    assert!(applied.contains("puts $::a::total"), "{applied}");
    assert!(
        applied.contains("puts $::b::v"),
        "::b's cell must survive: {applied}"
    );
    assert!(
        applied.contains("namespace eval ::b {\n    variable v 2\n}"),
        "::b's declaration must survive: {applied}"
    );
}

/// FP guard — renaming a cell onto a name the same namespace already
/// declares would merge two distinct variables, so the whole rename
/// answers nothing (the same discipline the single-document variable
/// rename's scope-chain collision check applies).
#[test]
fn fp_namespace_variable_rename_refuses_a_collision_in_its_own_namespace() {
    let src = "namespace eval ::ns {\n    variable v 1\n    variable total 2\n}\nputs $::ns::v\n";
    let analysis = analyse(src);
    assert!(
        namespace_variable_rename_edits(src, &analysis, "::ns::v", "total").is_empty(),
        "renaming `v` onto the existing `total` must produce no edits"
    );
}

/// TN — an unqualified local that merely shares the tail name is not the
/// cell, so it is not renamed.
#[test]
fn tn_namespace_variable_rename_leaves_an_unrelated_local_alone() {
    let src = "namespace eval ::ns {\n    variable v 1\n}\n\
                   proc other {} {\n    set v 9\n    return $v\n}\n\
                   puts $::ns::v\n";
    let analysis = analyse(src);
    let edits = namespace_variable_rename_edits(src, &analysis, "::ns::v", "total");
    let applied = apply_edits(src, &edits);
    assert!(
        applied.contains("set v 9"),
        "the proc local must survive: {applied}"
    );
    assert!(applied.contains("return $v"), "{applied}");
    assert!(applied.contains("puts $::ns::total"), "{applied}");
}

/// FP guard, end of the in-document path: the untracked-receiver shape
/// refuses with a reason rather than emitting the declaration-only edit
/// set that breaks the program on both interpreters.
#[test]
fn fp_rename_refuses_the_untracked_receiver_dispatch() {
    let src = "oo::class create Vector3d {\n\
                   \x20   variable _x\n\
                   \x20   constructor {args} {\n\
                   \x20       set other [lindex $args 0]\n\
                   \x20       set _x [$other X]\n\
                   \x20   }\n\
                   \x20   method X {} { return $_x }\n\
                   }\n";
    let analysis = analyse(src);
    let err = rename_with_diagnosis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        6,
        11,
        "GetX",
        &analysis,
        None,
    )
    .expect_err("idx 79's shape must refuse");
    assert!(err.reason.contains("not tracked"), "{}", err.reason);
    assert!(err.range.is_some(), "the refusal must point at the site");
    // And the plain entry point must produce no edits at all — never a
    // partial set.
    assert_eq!(
        rename(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            6,
            11,
            "GetX",
            &analysis,
            None
        ),
        [] as [crate::rename::TextEdit; 0]
    );
}

// The gate must not depend on the trigger position.  A real editor's
// "rename symbol" gesture can be invoked from any occurrence, and an
// ungated one returns, from the two positions below, a live
// WorkspaceEdit rewriting only the declaration and the `export` word.
// Applying that edit and running it (tclsh 9.0.4 and 8.6.16, byte
// identical):
//
//   unknown method "X": must be Get, GetX, Y, Z or destroy
//
// at the constructor's own `[$args X]`.  Every position that names the
// same member must therefore reach the same verdict.

/// The exact source the trigger-position tests share — the copy-
/// constructor shape from nico-robert/tomato's `Vector3d.tcl`, with the
/// `export` list the finding's `WorkspaceEdit` also rewrote.
///
/// Line/column map (0-based):
///   4  col 23 — the `X` inside `[$other X]`   (untracked call site)
///   6  col 11 — the `X` of `method X`          (declaration)
///   8  col 11 — the `X` in `export X Get`      (export bareword)
const IDX79_HAZARD: &str = "oo::class create Vector3d {\n\
         \x20   variable _x _y\n\
         \x20   constructor {args} {\n\
         \x20       set other [lindex $args 0]\n\
         \x20       set _x [$other X]\n\
         \x20   }\n\
         \x20   method X {} { return $_x }\n\
         \x20   method Get {} { return \"$_x $_y\" }\n\
         \x20   export X Get\n\
         }\n";

/// The same class with **every** receiver tracked: `$v` is bound by
/// `Vector3d new`, so the reference scan rewrites its dispatch and there
/// is no hazard.  The true-negative control for the tests below — an
/// over-broad gate would refuse this too and make rename unusable.
///
/// Line/column map (0-based):
///   1  col 11 — the `X` of `method X`          (declaration)
///   2  col 31 — the `X` inside `[my X]`        (`my` dispatch)
///   3  col 11 — the `X` in `export X Get`      (export bareword)
///   6  col  9 — the `X` inside `[$v X]`        (tracked `$obj` dispatch)
const IDX79_SAFE: &str = "oo::class create Vector3d {\n\
         \x20   method X {} { return 1 }\n\
         \x20   method Get {} { return [my X] }\n\
         \x20   export X Get\n\
         }\n\
         set v [Vector3d new]\n\
         puts [$v X]\n";

/// Every trigger position that reaches the hazardous member refuses —
/// declaration, untracked call site, and `export` bareword alike.
#[test]
fn fp_rename_refuses_the_untracked_receiver_from_every_trigger_position() {
    let analysis = analyse(IDX79_HAZARD);
    for (line, character, what) in [
        (4, 23, "the untracked `[$other X]` call site"),
        (6, 11, "the `method X` declaration"),
        (8, 11, "the `export X Get` bareword"),
    ] {
        let err = rename_with_diagnosis(
            IDX79_HAZARD,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            line,
            character,
            "GetX",
            &analysis,
            None,
        )
        .map_or_else(
            |err| err,
            |edits| panic!("{what} must refuse; it returned {edits:?} instead"),
        );
        assert!(
            err.reason.contains("not tracked"),
            "{what}: the refusal must name the untracked receiver, got {}",
            err.reason
        );
        assert!(
            rename(
                IDX79_HAZARD,
                tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
                line,
                character,
                "GetX",
                &analysis,
                None
            )
            .is_empty(),
            "{what} must never yield a partial edit set"
        );
    }
}

/// TN control: with every receiver tracked the rename still succeeds from
/// each of the same trigger positions, and the applied result rewrites
/// the declaration, the dispatches, *and* the export word together —
/// the property that makes the edit safe to run.
#[test]
fn fn_guard_a_fully_tracked_member_renames_from_every_trigger_position() {
    let analysis = analyse(IDX79_SAFE);
    for (line, character, what) in [
        (1, 11, "the `method X` declaration"),
        (2, 31, "the `my X` dispatch"),
        (3, 11, "the `export X Get` bareword"),
        (6, 9, "the tracked `[$v X]` call site"),
    ] {
        let edits = rename_with_diagnosis(
            IDX79_SAFE,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            line,
            character,
            "GetX",
            &analysis,
            None,
        )
        .unwrap_or_else(|err| panic!("{what} must not refuse: {}", err.reason));
        let applied = apply_edits(IDX79_SAFE, &edits);
        assert!(
            applied.contains("method GetX {}"),
            "{what}: declaration must rename: {applied}"
        );
        assert!(
            applied.contains("[my GetX]"),
            "{what}: the `my` dispatch must rename: {applied}"
        );
        assert!(
            applied.contains("export GetX Get"),
            "{what}: the export word must rename: {applied}"
        );
        assert!(
            !applied.contains(" X]") && !applied.contains("method X "),
            "{what}: no occurrence of the old name may survive: {applied}"
        );
    }
}

/// The resolution the gate piggy-backs on answers for exactly the
/// positions the untargeted tier claims, and abstains elsewhere — so
/// broadening the gate cannot start refusing renames an earlier tier owns.
#[test]
fn untargeted_member_target_answers_only_inside_the_class_body() {
    let analysis = analyse(IDX79_HAZARD);
    let line_index = LineIndex::new(IDX79_HAZARD);
    let at =
        |line: u32, ch: u32| crate::definition::byte_offset_at(&line_index, IDX79_HAZARD, line, ch);
    assert_eq!(
        untargeted_member_rename_target(&analysis, "X", at(8, 11)),
        Some(("::Vector3d".to_owned(), "X".to_owned(), false)),
        "the export bareword names the member"
    );
    assert_eq!(
        untargeted_member_rename_target(&analysis, "X", at(4, 23)),
        Some(("::Vector3d".to_owned(), "X".to_owned(), false)),
        "the untracked call site's method word names the member"
    );
    assert_eq!(
        untargeted_member_rename_target(&analysis, "other", at(4, 18)),
        None,
        "a word that names no member of the enclosing class must abstain"
    );
    assert_eq!(
        untargeted_member_rename_target(&analysis, "X", 0),
        None,
        "outside any class body there is no member to resolve"
    );
}

#[test]
fn logical_rename_preserves_original_separator_runs_unicode_and_array_extent() {
    // naming.editor.logical-written-component-geometry
    // docs/design/analysis/name-resolution-proofs/logical-written-component-geometry.md
    // Exact source geometry only; edit permission stays with the caller.
    let config =
        tcl_lexer::LexerConfig::from_grammar(tcl_dialect::DialectProfile::plain_tcl().grammar);
    for (written, expected) in [
        ("${::::茶:::箱(槽)}", "${::::茶:::total(槽)}"),
        ("$::::lib:::box(槽)", "$::::lib:::total(槽)"),
        ("${茶:箱}", "${total}"),
    ] {
        let span = tcl_lexer::Span::new(0, u32::try_from(written.len()).unwrap());
        assert_eq!(
            build_var_ref_replacement(written, span, "total", config).as_deref(),
            Some(expected)
        );
        assert!(
            build_var_ref_replacement(written, tcl_lexer::Span::new(0, 1), "total", config)
                .is_none()
        );
    }
    let bare_unicode = "$::::茶:::箱(槽)";
    let bare_unicode_span = tcl_lexer::Span::new(0, u32::try_from(bare_unicode.len()).unwrap());
    assert!(build_var_ref_replacement(bare_unicode, bare_unicode_span, "total", config).is_none());
    for (written, expected) in [
        ("::::茶:::méthode", "::::茶:::changed"),
        ("茶::::", "茶::::changed"),
        ("茶:méthode", "changed"),
    ] {
        assert_eq!(
            invocation_replacement("unused", "unused", "changed", written),
            expected
        );
    }
    let source = "set 茶:::箱(槽) VALUE";
    let analysis = Analyser::new().analyse(source, "tcl");
    let start = source.find("茶").unwrap();
    let end = source.find(" VALUE").unwrap();
    let span = tcl_lexer::Span::new(u32::try_from(start).unwrap(), u32::try_from(end).unwrap());
    assert_eq!(
        build_var_replacement_in_analysis(source, &analysis, span, "total")
            .unwrap()
            .1,
        "茶:::total(槽)"
    );
    assert!(
        build_var_replacement_in_analysis("set 茶:::箱(槽) OTHER", &analysis, span, "total")
            .is_none()
    );
}
