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
use std::sync::Arc;
use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};
use tcl_registry::definer::DefinerFamily;

fn analyse(source: &str, conflict: bool, class_grammar: bool) -> AnalysisResult {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let mut registry = tcl_registry::CommandRegistry::build_default();
    if conflict {
        registry.insert(tcl_registry::CommandSpec {
            name: "Target::p",
            ..tcl_registry::CommandSpec::DEFAULT
        });
    }
    if class_grammar {
        let mut spec = registry.get("itcl::class").unwrap().clone();
        spec.name = "source_class";
        spec.required_package = None;
        registry.insert(spec);
    }
    let context = tcl_registry::model::ingress::context_for_profile(profile)
        .with_command_store(Arc::new(registry));
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        Arc::new(context),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name)
}

#[test]
fn indexed_class_family_retains_its_actual_store_and_unknown_stays_unknown() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let source = "source_class C {proc make {} {return OK}}";
    let mut analysis = analyse(source, false, true);
    let uri = "file:///class.tcl";
    let index = WorkspaceIndex::from_documents([(uri, &analysis)]);
    let class = index.classes_named("::C")[0];
    assert_eq!(class.definer_family(), Some(DefinerFamily::Itcl));
    assert!(class.is_itcl(tcl_dialect::DialectProfile::find("tcl9.1").unwrap()));
    assert!(
        crate::registry_for_dialect_profile(tcl_dialect::DialectProfile::plain_tcl())
            .get(&class.metaclass)
            .is_none()
    );
    assert_eq!(
        index.clone().classes_named("::C")[0].definer_family(),
        class.definer_family()
    );
    analysis.resolved_input = None;
    let unknown = WorkspaceIndex::from_documents([(uri, &analysis)]);
    assert_eq!(unknown.classes_named("::C")[0].definer_family(), None);
    assert!(unknown.registry_for(uri).is_none());
}

#[test]
fn actual_import_metadata_store_change_invalidates_warmed_same_profile_settlement() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    // Readonly authored import metadata; no installed native import is issued.
    let library_source = "namespace eval Mine {proc p {} {}; namespace export p}\n";
    let caller_source = "namespace eval Target {namespace import ::Mine::*; proc run {} {p}}\n";
    let library = analyse(library_source, false, false);
    let first = analyse(caller_source, false, false);
    let second = analyse(caller_source, true, false);
    assert_eq!(first.dialect, second.dialect);
    let uri = "file:///caller.tcl";
    let mut index =
        WorkspaceIndex::from_documents([("file:///library.tcl", &library), (uri, &first)]);
    let before = index.docs[*index.slots.get(uri).unwrap()].settlement_dependencies();
    assert_eq!(index.linked_invocations_of("::Mine::p", "").len(), 1);
    index.replace_document(uri, &second);
    let after = index.docs[*index.slots.get(uri).unwrap()].settlement_dependencies();
    assert!(before != after);
    assert!(
        index
            .registry_for(uri)
            .unwrap()
            .declares_command_at("::Target::p")
    );
    assert_eq!(
        index.linked_invocations_of("::Mine::p", ""),
        [] as [&crate::workspace_index::WorkspaceInvocation; 0]
    );
    index.replace_document(uri, &first);
    assert_eq!(index.linked_invocations_of("::Mine::p", "").len(), 1);
}

#[test]
fn indexed_registry_borrow_rejects_foreign_input_and_cleared_slot() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let source = "proc p {} {}; p";
    let original = analyse(source, false, false);
    let foreign = analyse(source, true, false);
    let uri = "file:///source.tcl";
    let mut index = WorkspaceIndex::from_documents([(uri, &original)]);
    assert_eq!(
        index.registry_for(uri).unwrap().snapshot().semantic_key(),
        original
            .resolved_registry()
            .unwrap()
            .snapshot()
            .semantic_key()
    );
    let slot = *index.slots.get(uri).unwrap();
    index.docs[slot].original_source_input = foreign.resolved_input;
    assert!(index.registry_for(uri).is_none());
    index.replace_document(uri, &original);
    assert!(index.registry_for(uri).is_some());
    index.remove_document(uri);
    index.add_document("file:///reused.tcl", &AnalysisResult::default());
    assert!(index.registry_for("file:///reused.tcl").is_none());
    assert!(index.registry_for(uri).is_none());
}
