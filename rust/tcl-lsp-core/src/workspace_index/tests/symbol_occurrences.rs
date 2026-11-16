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

//! Namespace declarations and references, namespace variables and cell aliases across documents.

use super::*;

#[test]
fn namespace_declarations_and_refs_are_indexed_across_documents() {
    // TP — the declaring `namespace eval` blocks live in
    // one document and the `namespace children ::mypkg` consumer in
    // another; both spellings name the one namespace.  Oracle (tclsh
    // 9.0.4 / 8.6.16, byte-identical): reopening `::mypkg` extends the
    // same namespace, and `namespace children ::mypkg` lists its
    // children rather than erroring.
    let decl = analyse("namespace eval mypkg {}\nnamespace eval mypkg {}\n");
    let user = analyse("set t [namespace children ::mypkg]\n");
    let index =
        WorkspaceIndex::from_documents([("file:///decl.tcl", &decl), ("file:///user.tcl", &user)]);
    assert_eq!(
        index
            .namespace_declarations_qualified("::mypkg", "")
            .iter()
            .map(|n| n.uri.as_str())
            .collect::<Vec<_>>(),
        vec!["file:///decl.tcl", "file:///decl.tcl"],
        "both declaring blocks are definition sites",
    );
    assert_eq!(
        index
            .namespace_refs_of("::mypkg", "")
            .iter()
            .map(|n| n.uri.as_str())
            .collect::<Vec<_>>(),
        vec!["file:///user.tcl"],
    );
    // The declaring document is excluded on request, which is how the
    // cross-document definition tier avoids re-reporting local answers.
    assert_eq!(
        index.namespace_declarations_qualified("::mypkg", "file:///decl.tcl"),
        [] as [&crate::workspace_index::WorkspaceNamespaceRef; 0],
    );
}

/// Declaring rows whose name is a **strict descendant** of
/// the cell: the workspace half of the implicit-parent answer.
///
/// tclsh-proof (9.0.4 / 8.6.16, byte-identical): `namespace eval
/// ::p::q::r {}` leaves `namespace exists ::p::q` -> 1 and `namespace
/// exists ::p` -> 1, while `namespace eval ::pq::r {}` leaves `namespace
/// exists ::p` -> 0.
#[test]
fn namespace_declarations_under_finds_implicit_parent_rows() {
    let decl = analyse("namespace eval ::p::q::r {}\n");
    let other = analyse("namespace eval ::pq::r {}\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///decl.tcl", &decl),
        ("file:///other.tcl", &other),
    ]);
    // TP — both ancestors are answered by the one deeper block.
    assert_eq!(
        index
            .namespace_declarations_under("::p::q", "")
            .iter()
            .map(|n| n.qualified_name.as_str())
            .collect::<Vec<_>>(),
        vec!["::p::q::r"],
    );
    assert_eq!(index.namespace_declarations_under("::p", "").len(), 1);
    // TN — an exact match is a real declaration, not an implicit one.
    assert_eq!(
        index.namespace_declarations_under("::p::q::r", ""),
        [] as [&crate::workspace_index::WorkspaceNamespaceRef; 0],
    );
    // TN — a segment prefix is a different namespace entirely.
    assert!(
        index
            .namespace_declarations_under("::p", "file:///decl.tcl")
            .is_empty(),
        "excluding the declaring document leaves only `::pq::r`, which is not under `::p`",
    );
    // TN — the global namespace is created by the interpreter, so nothing
    // implicitly creates it.
    assert_eq!(
        index.namespace_declarations_under("::", ""),
        [] as [&crate::workspace_index::WorkspaceNamespaceRef; 0]
    );
    assert_eq!(
        index.namespace_declarations_under("", ""),
        [] as [&crate::workspace_index::WorkspaceNamespaceRef; 0]
    );
}

#[test]
fn namespace_rows_are_dropped_with_their_document() {
    // A re-index (`remove_document` then `add_document`) must not leave
    // stale namespace rows behind — the same discipline every other
    // table follows.
    let a = analyse("namespace eval gone {}\n");
    let mut index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
    assert_eq!(
        index.namespace_declarations_qualified("::gone", "").len(),
        1
    );
    index.remove_document("file:///a.tcl");
    assert_eq!(
        index.namespace_declarations_qualified("::gone", ""),
        [] as [&crate::workspace_index::WorkspaceNamespaceRef; 0]
    );
    assert!(index.namespace_refs().next().is_none());
}

#[test]
fn a_relative_namespace_word_is_indexed_rooted() {
    // TP — the analyser roots a relative spelling against its own
    // namespace before the index sees it, so a sibling document can
    // match it by exact qualified name.  Oracle: inside `namespace eval
    // ::outer`, `namespace children inner` means `::outer::inner`; the
    // same words at global scope mean `::inner` (both interpreters).
    let a = analyse(
        "namespace eval ::outer {\n    namespace eval inner {}\n    namespace children inner\n}\n",
    );
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
    assert_eq!(
        index
            .namespace_declarations_qualified("::outer::inner", "")
            .len(),
        1,
    );
    assert_eq!(index.namespace_refs_of("::outer::inner", "").len(), 1);
    assert_eq!(
        index.namespace_refs_of("::inner", ""),
        [] as [&crate::workspace_index::WorkspaceNamespaceRef; 0]
    );
}

/// TP — a document whose only stake
/// in `::ns::v` is an alias written from *outside* `::ns` is found by the
/// alias table, and by nothing else.
///
/// A global `proc p {} { namespace upvar ::ns v local; … }` declares
/// nothing in `::ns`, holds no namespace-scoped declaration of the cell,
/// and writes no qualified occurrence of it — so all three of the older
/// candidate sources miss it, while the alias it holds breaks the moment
/// the declaration moves without it (tclsh 9.0.4 / 8.6.16: `can't read
/// "local": no such variable`).
#[test]
fn indexes_a_cell_alias_written_from_another_namespace() {
    let decl = analyse("namespace eval ns {\n    variable v 1\n}\n");
    let aliaser = analyse("proc p {} {\n    namespace upvar ::ns v local\n    return $local\n}\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///decl.tcl", &decl);
    index.add_document("file:///aliaser.tcl", &aliaser);

    assert_eq!(
        index.documents_aliasing_variable("::ns::v"),
        vec![
            "file:///aliaser.tcl".to_string(),
            "file:///decl.tcl".to_string()
        ],
        "both the `variable v` declaration and the out-of-namespace \
             `namespace upvar` are aliases of the one cell",
    );
    assert!(
        !index
            .documents_in_namespace("::ns")
            .contains(&"file:///aliaser.tcl".to_string()),
        "the aliasing document declares nothing in ::ns — which is why \
             the alias table is needed",
    );
    assert!(
        index
            .variable_refs_of("::ns::v", "")
            .iter()
            .all(|r| r.uri != "file:///aliaser.tcl"),
        "and it writes no qualified occurrence either",
    );
}

/// TN (same finding) — the alias table is keyed on the cell, so an alias
/// of a *different* cell never drags its document into another cell's
/// rename.
#[test]
fn cell_alias_index_does_not_match_a_different_cell() {
    let other = analyse("proc p {} {\n    namespace upvar ::ns other local\n}\n");
    let index = WorkspaceIndex::from_documents([("file:///other.tcl", &other)]);
    assert_eq!(
        index.documents_aliasing_variable("::ns::v"),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        index.documents_aliasing_variable("::ns::other"),
        vec!["file:///other.tcl".to_string()]
    );
}

/// TP — a namespace variable declared in one
/// document and read, qualified, from another is matched across the two by
/// its one `::`-rooted cell name.
#[test]
fn indexes_namespace_variables_and_their_qualified_occurrences() {
    let decl = analyse("namespace eval app::colors {\n    variable palette red\n}\n");
    let user = analyse("puts $app::colors::palette\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///decl.tcl", &decl);
    index.add_document("file:///user.tcl", &user);

    let defs = index.variable_definitions_qualified("::app::colors::palette", "");
    assert_eq!(defs.len(), 1, "one declaration: {defs:?}");
    assert_eq!(defs[0].uri, "file:///decl.tcl");
    assert_eq!(defs[0].name, "palette");

    let refs = index.variable_refs_of("::app::colors::palette", "file:///decl.tcl");
    assert_eq!(
        refs.iter().map(|r| r.uri.as_str()).collect::<Vec<_>>(),
        vec!["file:///user.tcl"],
        "the sibling document's qualified read is a reference: {refs:?}",
    );

    // TN: an unrelated cell name matches nothing.
    assert_eq!(
        index.variable_definitions_qualified("::app::colors::other", ""),
        [] as [&crate::workspace_index::WorkspaceVariable; 0],
    );
    index.remove_document("file:///decl.tcl");
    assert!(
        index
            .variable_definitions_qualified("::app::colors::palette", "")
            .is_empty(),
        "removing the declaring document drops its variables",
    );
}

/// TN — a proc **local** never enters the variable table: it has no
/// qualified name a sibling document could spell.
#[test]
fn proc_locals_are_not_indexed_as_namespace_variables() {
    let a = analyse("proc ns::f {} {\n    set localOnly 1\n}\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    assert!(
        !index.variables().any(|v| v.name == "localOnly"),
        "indexed variables: {:?}",
        index.variables().collect::<Vec<_>>(),
    );
}
