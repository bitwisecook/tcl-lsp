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

mod class_dispatch;
mod command_settlement;
mod declarations;
mod forced_import_shadow;
mod import_conflicts;
mod import_lifecycle;
mod import_resolution;
mod load_order;
mod symbol_occurrences;
mod wildcard_index;

fn analyse(source: &str) -> AnalysisResult {
    analyse_as(
        source,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
}

fn analyse_as(source: &str, dialect: &'static tcl_dialect::DialectProfile) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, dialect.name).clone()
}

/// A call site at the end of `uri` — the offset only matters to the
/// import-lifecycle gate (a `namespace forget` / source deletion earlier
/// in the *same* document), so tests with no such event may use it
/// freely.
/// Byte offset of `needle` in a test source.
fn app_src_offset(src: &str, needle: &str) -> usize {
    src.find(needle).expect("needle present")
}

fn call_from(uri: &str) -> CallSite<'_> {
    call_at(uri, u32::MAX)
}

/// A **top-level** call site at `at` in `uri` — the shape every ordering
/// test below uses, since a load-level call carries no enclosing body.
fn call_at(uri: &str, at: u32) -> CallSite<'_> {
    CallSite {
        uri,
        at,
        enclosing_body: None,
    }
}

#[test]
fn remove_document_drops_its_entries() {
    let a = analyse("proc a {} {}\n");
    let b = analyse("proc b {} {}\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    index.add_document("file:///b.tcl", &b);
    assert_eq!(index.procs().count(), 2);
    index.remove_document("file:///a.tcl");
    assert_eq!(index.procs().count(), 1);
    assert_eq!(index.procs().next().map(|p| p.name.as_str()), Some("b"));
}

/// A removal is scoped to the removed document.  Every other
/// document's records survive it untouched, in their original order —
/// which is what lets the removal cost that one document's rows instead of
/// a pass over every table in the workspace.
#[test]
fn remove_document_leaves_the_other_documents_alone() {
    let a = analyse("proc a {} {}\na\n");
    let b = analyse("proc b {} {}\nb\n");
    let c = analyse("proc c {} {}\nc\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    index.add_document("file:///b.tcl", &b);
    index.add_document("file:///c.tcl", &c);
    index.remove_document("file:///b.tcl");
    let names: Vec<&str> = index.procs().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["a", "c"]);
    let call_uris: std::collections::BTreeSet<&str> =
        index.invocations().map(|i| i.uri.as_str()).collect();
    assert_eq!(
        call_uris,
        ["file:///a.tcl", "file:///c.tcl"].into_iter().collect()
    );
    assert_eq!(
        index.document_uris(),
        vec!["file:///a.tcl", "file:///c.tcl"]
    );
}

/// The remove-then-add every diagnostics publish performs must
/// neither grow the index nor shuffle the workspace — the document goes
/// back into the slot it just vacated.
#[test]
fn re_indexing_a_document_keeps_its_size_and_its_place() {
    let a = analyse("proc a {} {}\n");
    let b = analyse("proc b {} {}\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    index.add_document("file:///b.tcl", &b);
    for _ in 0..5 {
        index.remove_document("file:///a.tcl");
        index.add_document("file:///a.tcl", &a);
    }
    let names: Vec<&str> = index.procs().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["a", "b"]);
}

/// Removing a URI the index never held changes nothing.
#[test]
fn removing_an_unindexed_document_changes_nothing() {
    let a = analyse("proc a {} {}\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    index.remove_document("file:///never-indexed.tcl");
    assert_eq!(index.procs().count(), 1);
}

/// Several analyses of one URI — one re-homed view per source-site
/// namespace — accumulate under that URI, and one removal drops the whole
/// set.
#[test]
fn several_views_of_one_document_accumulate_and_drop_together() {
    let a = analyse("proc helper {} {}\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    index.add_document("file:///a.tcl", &a);
    assert_eq!(index.procs().count(), 2);
    index.remove_document("file:///a.tcl");
    assert_eq!(index.procs().count(), 0);
}

// The `source` graph orders what the file boundary cannot.
//
// Sourcing a file inlines its whole
// body at the `source` statement's position, so the DFS of the source
// forest *is* the run order. Oracle for every case below, byte-identical
// on tclsh 8.6.14 and 9.0.4 — with
//
//   # exp.tcl:  namespace eval ::mymod { namespace export helper }
//   # imp.tcl:  namespace eval ::app   { namespace import ::mymod::* }
//   # mod.tcl:  namespace eval ::mymod { proc helper {} {return HELP} }
//
//   # app.tcl:  source mod.tcl; source exp.tcl; source imp.tcl
//   ::app::helper   ->  HELP
//   # app.tcl:  source mod.tcl; source imp.tcl; source exp.tcl
//   ::app::helper   ->  invalid command name "::app::helper"

/// The installed server resolver's source-plan and inventory contract,
/// with the file URI mapped to an explicit document filename.
fn test_resolve(
    parent_uri: &str,
    _raw_path: &str,
    expression: Option<&tcl_compiler::auto_path_eval::OriginalSourcePathExpression>,
    source_offset: u32,
    raw_constants: &tcl_compiler::auto_path_eval::PathConstantAssignments,
    imported: &tcl_compiler::auto_path_eval::FoldedPathConstants,
) -> Option<String> {
    let parent = parent_uri.strip_prefix("file://")?;
    let dir = std::path::Path::new(parent).parent()?;
    let expression = expression?;
    if !expression.matches_assignments(raw_constants)
        || expression.source_span().start() > source_offset
        || source_offset >= expression.source_span().end()
    {
        return None;
    }
    let constants = tcl_compiler::auto_path_eval::fold_constant_assignments_with_imports(
        raw_constants,
        Some(parent),
        imported,
    );
    let raw = expression.evaluate(Some(parent), &|name| {
        tcl_compiler::auto_path_eval::PathConstantLookup::path_constant(
            &constants.at(source_offset),
            name,
        )
    })?;
    let child = crate::source_graph::resolve_under(dir, &raw);
    Some(format!("file://{}", child.display()))
}

/// The server's constant folder, in miniature — the counterpart of
/// [`test_resolve`], same URI-to-path convention.
fn test_fold(
    uri: &str,
    writes: &tcl_compiler::auto_path_eval::PathConstantAssignments,
    imported: &tcl_compiler::auto_path_eval::FoldedPathConstants,
) -> tcl_compiler::auto_path_eval::FoldedPathConstants {
    let path = uri.strip_prefix("file://");
    tcl_compiler::auto_path_eval::fold_constant_assignments_with_imports(writes, path, imported)
}

/// An index over `documents` with the `source`-path resolver installed —
/// the shape the real server builds ([`WorkspaceIndex::set_source_resolver`]).
fn sourced_index<'a>(
    documents: impl IntoIterator<Item = (&'a str, &'a AnalysisResult)>,
) -> WorkspaceIndex {
    let mut index = WorkspaceIndex::new();
    index.set_source_resolver(test_resolve, test_fold);
    for (uri, analysis) in documents {
        index.add_document(uri, analysis);
    }
    index
}

/// The three module documents every source-order test below shares.
fn import_order_modules() -> (AnalysisResult, AnalysisResult, AnalysisResult) {
    (
        analyse("namespace eval ::mymod { proc helper {} { return HELP } }\n"),
        analyse("namespace eval ::mymod { namespace export helper }\n"),
        analyse("namespace eval ::app { namespace import ::mymod::* }\n"),
    )
}

/// The generation counter must advance on every mutation, so a consumer
/// can tell the index changed without diffing it.
#[test]
fn generation_advances_on_every_mutation() {
    let a = analyse("proc helper {} {}\n");
    let mut index = WorkspaceIndex::new();
    let start = index.generation();
    index.add_document("file:///a.tcl", &a);
    let after_add = index.generation();
    assert_ne!(start, after_add, "add_document must bump the generation");
    index.remove_document("file:///a.tcl");
    assert_ne!(
        after_add,
        index.generation(),
        "remove_document must bump the generation",
    );
}

#[test]
fn document_revision_changes_only_with_that_document() {
    let a = analyse("proc alpha {} {}\n");
    let b = analyse("proc beta {} {}\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    let a_revision = index.document_revision("file:///a.tcl").unwrap();

    index.add_document("file:///b.tcl", &b);
    assert_eq!(
        index.document_revision("file:///a.tcl"),
        Some(a_revision),
        "an unrelated mutation must not invalidate a source snapshot",
    );
    index.replace_document("file:///a.tcl", &a);
    assert_ne!(
        index.document_revision("file:///a.tcl"),
        Some(a_revision),
        "replacing the document must invalidate its source snapshot",
    );
    index.remove_document("file:///a.tcl");
    assert_eq!(index.document_revision("file:///a.tcl"), None);
}

#[test]
fn conditional_revision_removal_preserves_a_newer_replacement() {
    let old = analyse("proc old {} {}\n");
    let new = analyse("proc new {} {}\n");
    let mut index = WorkspaceIndex::new();
    let old_revision = index.replace_document_with_revision("file:///a.tcl", &old);
    let new_revision = index.replace_document_with_revision("file:///a.tcl", &new);

    assert!(
        !index.remove_document_if_revision("file:///a.tcl", old_revision),
        "an obsolete publisher must not remove a newer replacement",
    );
    assert_eq!(index.document_revision("file:///a.tcl"), Some(new_revision));
    assert!(index.workspace_command_exists("::new"));
    assert!(!index.workspace_command_exists("::old"));
    assert!(index.remove_document_if_revision("file:///a.tcl", new_revision));
    assert!(!index.contains_document("file:///a.tcl"));
}

#[test]
fn document_revision_survives_slot_reuse_aba_1854() {
    let a = analyse("proc alpha {} {}\n");
    let b = analyse("proc beta {} {}\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    let old_revision = index.document_revision("file:///a.tcl").unwrap();

    index.remove_document("file:///a.tcl");
    index.add_document("file:///b.tcl", &b);
    index.add_document("file:///a.tcl", &a);

    assert_ne!(
        index.document_revision("file:///a.tcl"),
        Some(old_revision),
        "re-adding a URI must not recycle its old token after slot churn",
    );
}

#[test]
fn remove_document_drops_invocations_too() {
    let a = analyse("helper\n");
    let mut index = WorkspaceIndex::new();
    index.add_document("file:///a.tcl", &a);
    assert!(index.invocations().next().is_some());
    index.remove_document("file:///a.tcl");
    assert!(index.invocations().next().is_none());
}
