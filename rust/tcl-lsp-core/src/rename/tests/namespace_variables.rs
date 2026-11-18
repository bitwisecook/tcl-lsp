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

//! Namespace-variable renames: declarations, aliases, qualified uses and collisions.

use super::*;

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
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    assert!(!analysis.allows_lexical_declaration_advice());
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
