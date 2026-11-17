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

//! Procedure and class declarations: inventory ownership, literal-colon namespace keys and lookups across documents.

use super::*;

#[test]
fn original_declaration_inventory_retains_bytes_and_document_ownership() {
    // Proof naming.workspace.original-declaration-transport:
    // docs/design/analysis/name-resolution-proofs/workspace-original-declaration-transport.md
    // Implementation contract: opaque naming units, declaration metadata
    // and workspace document ownership survive removal of display maps.
    // Native naming outcomes are established by the separate byte probes.
    let source = r"proc p\uD800 {} {return FIRST}
proc p\uD801 {} {return SECOND}
p\uD800
p\uD801";
    let mut analysis = analyse(source);
    assert_eq!(analysis.original_procedure_declarations().count(), 2);
    analysis.all_procs.clear();
    analysis.superseded_procs.clear();
    let mut index = WorkspaceIndex::from_documents([
        ("file:///first.tcl", &analysis),
        ("file:///second.tcl", &analysis),
    ]);
    let declarations = index.original_procedure_declarations().collect::<Vec<_>>();
    assert_eq!(declarations.len(), 4);
    let input = tcl_compiler::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
        declarations[0].1.name_input().clone(),
    );
    let lookup = tcl_compiler::signature_scan::scope::SignatureSourceLookup::from_input(
        analysis.original_namespace_scope_at(0).unwrap().clone(),
        &input,
    )
    .unwrap();
    let candidates = index.original_procedure_candidates(&lookup);
    assert_eq!(candidates.len(), 2);
    assert!(
        candidates
            .iter()
            .all(|(_, declaration)| { declaration.name_input().bytes() == b"p\xed\xa0\x80" })
    );
    for uri in ["file:///first.tcl", "file:///second.tcl"] {
        let entries = declarations
            .iter()
            .filter(|(owner, _)| *owner == uri)
            .map(|(_, declaration)| *declaration)
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name_input().bytes(), b"p\xed\xa0\x80");
        assert_eq!(entries[1].name_input().bytes(), b"p\xed\xa0\x81");
        assert_ne!(entries[0].name().slot(), entries[1].name().slot());
    }
    assert_eq!(
        index
            .invocations()
            .filter(|invocation| {
                invocation
                    .original_name_input
                    .as_ref()
                    .is_some_and(|input| input.bytes() == b"p\xed\xa0\x80")
            })
            .count(),
        2
    );
    index.remove_document("file:///first.tcl");
    assert!(
        index
            .original_procedure_declarations()
            .all(|(uri, _)| uri == "file:///second.tcl")
    );
    index.add_document("file:///first.tcl", &analyse("proc plain {} {}"));
    assert_eq!(index.original_procedure_declarations().count(), 3);
    assert!(
        index
            .original_procedure_declarations()
            .filter(|(uri, _)| *uri == "file:///first.tcl")
            .all(|(_, declaration)| declaration.name_input().bytes() == b"plain")
    );
}

#[test]
fn literal_colon_namespace_keys_keep_definitions_and_variables_distinct() {
    let nested = analyse("namespace eval : {proc p {} {}; variable v 1}\n");
    let global = analyse("proc :p {} {}\nvariable :v 2\n");
    let index = WorkspaceIndex::from_documents([
        ("file:///nested.tcl", &nested),
        ("file:///global.tcl", &global),
    ]);
    let nested_proc = index.proc_definitions_qualified(":::::p", "");
    let global_proc = index.proc_definitions_qualified(":::p", "");
    assert_eq!(nested_proc.len(), 1);
    assert_eq!(global_proc.len(), 1);
    assert_eq!(nested_proc[0].uri, "file:///nested.tcl");
    assert_eq!(global_proc[0].uri, "file:///global.tcl");
    let nested_var = index.variable_definitions_qualified(":::::v", "");
    assert_eq!(nested_var.len(), 1);
    assert_eq!(nested_var[0].uri, "file:///nested.tcl");
    let global_var = index.variable_definitions_qualified(":::v", "");
    assert_eq!(global_var.len(), 1);
    assert_eq!(global_var[0].uri, "file:///global.tcl");
    assert_eq!(index.documents_in_namespace(":::"), ["file:///nested.tcl"]);
    assert_eq!(index.documents_in_namespace("::"), ["file:///global.tcl"]);
    let namespaces = index.observable_namespaces();
    assert!(namespaces.contains(":"));
    assert!(namespaces.contains(""));
}

#[test]
fn indexes_procs_from_multiple_documents() {
    let a = analyse("proc greet {name} {}\n");
    let b = analyse("proc farewell {} {}\nproc greet2 {x y} {}\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    assert_eq!(index.procs().count(), 3);
    // Param counts captured.
    let greet = index.procs().find(|p| p.name == "greet").unwrap();
    assert_eq!(greet.param_count, 1);
    assert_eq!(greet.uri, "file:///a.tcl");
}

#[test]
fn procs_matching_excludes_current_doc_and_filters_prefix() {
    let a = analyse("proc alpha {} {}\n");
    let b = analyse("proc alphabet {} {}\nproc beta {} {}\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    // From a.tcl's perspective, only b.tcl procs with `alph`.
    let got = index.procs_matching("alph", "file:///a.tcl");
    let names: Vec<&str> = got.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, vec!["alphabet"]);
}

#[test]
fn proc_definitions_resolves_cross_document() {
    let a = analyse("proc helper {} {}\n");
    let b = analyse("helper\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
    // From b.tcl, `helper` resolves to a.tcl's definition.
    let defs = index.proc_definitions("helper", "file:///b.tcl");
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].uri, "file:///a.tcl");
    // The same-document exclusion drops it when querying
    // from a.tcl itself.
    assert_eq!(
        index.proc_definitions("helper", "file:///a.tcl"),
        [] as [&crate::workspace_index::WorkspaceProc; 0]
    );
}

#[test]
fn indexes_classes() {
    let a = analyse("oo::class create Widget {}\n");
    let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
    let defs = index.class_definitions("Widget", "file:///other.tcl");
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].qualified_name, "::Widget");
}

/// A `superclass` word whose class another document declares selects that
/// declaration through the word's own lookup (current namespace, then
/// global); a word naming no declared class selects nothing.
#[test]
fn a_cross_file_superclass_word_selects_the_declaring_document() {
    let base = analyse("namespace eval ::ns {\n    oo::class create Base {}\n}\n");
    let sub_source = "namespace eval ::app {\n    oo::class create Sub {\n        superclass ::ns::Base\n    }\n}\n";
    let sub = analyse(sub_source);
    let index =
        WorkspaceIndex::from_documents([("file:///base.tcl", &base), ("file:///sub.tcl", &sub)]);
    let cursor = u32::try_from(sub_source.find("::ns::Base").unwrap() + 4).unwrap();
    let targets = index.class_relation_targets_at("file:///sub.tcl", sub_source, &sub, cursor);
    assert_eq!(targets.len(), 1, "{targets:?}");
    assert_eq!(targets[0].0, "file:///base.tcl");
    let other = sub_source.replace("::ns::Base", "::ns::Other");
    let other_analysis = analyse(&other);
    let index = WorkspaceIndex::from_documents([
        ("file:///base.tcl", &base),
        ("file:///sub.tcl", &other_analysis),
    ]);
    assert_eq!(
        index.class_relation_targets_at("file:///sub.tcl", &other, &other_analysis, cursor),
        []
    );
}
