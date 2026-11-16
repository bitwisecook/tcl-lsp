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
use crate::vfs::SourceStore;
use tcl_compiler::analyser::Analyser;
use tcl_compiler::signature_scan::scope::SignatureSourceCommand;

fn original_analysis(source: &str) -> AnalysisResult {
    Analyser::new().analyse(source, "tcl8.6").clone()
}

fn clear_reporting(analysis: &mut AnalysisResult) {
    analysis.all_procs.clear();
    analysis.superseded_procs.clear();
    analysis.all_classes.clear();
    analysis.superseded_classes.clear();
    for invocation in &mut analysis.command_invocations {
        invocation.name = "reporting-only".to_owned();
        invocation.resolved_qualified_name = Some("::wrong".to_owned());
        invocation.resolution_candidates = vec!["::wrong".to_owned()];
    }
}

fn original_names(analysis: &AnalysisResult) -> Vec<SignatureSourceCommand> {
    analysis
        .original_procedure_declarations()
        .map(|declaration| declaration.name().clone())
        .collect()
}

#[test]
fn original_source_procedure_candidates_share_slots_and_reindex_currency() {
    // naming.source.original-procedure-publications
    // docs/design/analysis/name-resolution-proofs/source-original-procedure-publications.md
    let source = "proc old {value} {}; proc deleted {} {}; rename old moved; rename deleted {}";
    let mut analysis = original_analysis(source);
    clear_reporting(&mut analysis);
    let uri = "file:///source-procedures.tcl";
    let mut index = WorkspaceIndex::from_documents([(uri, &analysis)]);
    let candidates = index
        .original_procedure_source_candidates()
        .collect::<Vec<_>>();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].0, uri);
    assert_eq!(candidates[0].1.name_input().bytes(), b"old");
    assert_eq!(candidates[0].2.simple.as_bytes(), b"moved");
    let caller = original_analysis("moved; old; deleted");
    for invocation in &caller.command_invocations {
        let input = invocation.original_name_input.as_ref().unwrap();
        let lookup = tcl_compiler::signature_scan::scope::SignatureSourceLookup::from_input(
            tcl_compiler::signature_scan::scope::SignatureNamespaceScope::C(
                tcl_core_types::ByteNamespacePath::root(),
            ),
            input,
        )
        .unwrap();
        let matching = index.original_procedure_candidates(&lookup);
        assert_eq!(matching.len(), usize::from(input.bytes() == b"moved"));
        if let Some((owner, declaration)) = matching.first() {
            assert_eq!(*owner, uri);
            assert_eq!(declaration.name_input().bytes(), b"old");
        }
    }
    let dependency = index.docs[0].settlement_dependencies();
    let replacement = original_analysis("proc old {value} {}; rename old other");
    index.replace_document(uri, &replacement);
    assert!(dependency != index.docs[0].settlement_dependencies());
    let names = index
        .original_procedure_source_candidates()
        .map(|(_, _, slot, _)| slot.simple.as_bytes().to_vec())
        .collect::<Vec<_>>();
    assert_eq!(names, [b"other".to_vec()]);
    index.remove_document(uri);
    index.add_document("file:///reused.tcl", &AnalysisResult::default());
    assert!(
        index
            .original_procedure_source_candidates()
            .next()
            .is_none()
    );
}

#[test]
fn original_source_class_candidates_share_current_slots_and_canonical_navigation() {
    // naming.source.original-class-publications
    // docs/design/analysis/name-resolution-proofs/source-original-class-publications.md
    let source =
        "oo::class create Old {}; oo::class create Removed {}; rename Old New; rename Removed {}";
    let mut analysis = original_analysis(source);
    assert!(analysis.original_completed_command_world().is_none());
    clear_reporting(&mut analysis);
    let uri = "file:///source-classes.tcl";
    let mut index = WorkspaceIndex::from_documents([(uri, &analysis)]);
    assert_eq!(index.original_class_declarations().count(), 2);
    let candidates = index.original_class_source_candidates().collect::<Vec<_>>();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].0, uri);
    assert_eq!(candidates[0].1.name_input().bytes(), b"Old");
    assert_eq!(candidates[0].2.simple.as_bytes(), b"New");
    let canonical = candidates[0].1.name().clone();
    let caller = original_analysis("New; Old; Removed");
    for invocation in &caller.command_invocations {
        let input = invocation.original_name_input.as_ref().unwrap();
        let lookup = tcl_compiler::signature_scan::scope::SignatureSourceLookup::from_input(
            tcl_compiler::signature_scan::scope::SignatureNamespaceScope::C(
                tcl_core_types::ByteNamespacePath::root(),
            ),
            input,
        )
        .unwrap();
        let selected = input.bytes() == b"New";
        let matching = index.original_class_candidates(&lookup);
        assert_eq!(matching.len(), usize::from(selected));
        for follow_links in [false, true] {
            assert_eq!(
                index.original_assistance_target(
                    invocation.original_lookup.as_ref().unwrap(),
                    follow_links,
                ),
                selected.then(|| SettledCommandTarget::OriginalSlot(
                    canonical.slot().clone(),
                    canonical.policy(),
                )),
            );
        }
    }
    let dependencies = index.docs[0].settlement_dependencies();
    let replacement = original_analysis("oo::class create Old {}; rename Old Other");
    index.replace_document(uri, &replacement);
    assert!(dependencies != index.docs[0].settlement_dependencies());
    assert_eq!(
        index
            .original_class_source_candidates()
            .next()
            .unwrap()
            .2
            .simple
            .as_bytes(),
        b"Other",
    );
    index.remove_document(uri);
    index.add_document("file:///reused-class.tcl", &AnalysisResult::default());
    assert!(index.original_class_source_candidates().next().is_none());
    assert!(index.docs[0].original_source_input.is_none());
    assert!(index.docs[0].original_source_classes.is_none());
}

#[test]
fn original_source_class_dependencies_retain_mutation_premises_and_known_barriers() {
    // naming.source.original-class-publications
    // docs/design/analysis/name-resolution-proofs/source-original-class-publications.md
    let uri = "file:///source-classes.tcl";
    let source = "oo::class create C {}";
    let analysis = original_analysis(source);
    let mut index = WorkspaceIndex::from_documents([(uri, &analysis)]);
    let dependencies = index.docs[0].settlement_dependencies();
    let uncertain = original_analysis("oo::class create C {}; unknown_effect");
    assert!(uncertain.original_completed_command_world().is_none());
    index.replace_document(uri, &uncertain);
    let changed = index.docs[0].settlement_dependencies();
    assert_eq!(changed.original_source_class_candidates.len(), 1);
    assert!(changed.original_source_class_candidates[0].3.contains(
        &tcl_compiler::command_binding::SourceCommandTransitionObligation::UnknownEarlierMutation,
    ));
    assert!(dependencies != changed);
    for source in [
        "oo::class create C {}; rename C {}",
        "oo::class create C {}; proc C {} {}",
    ] {
        let deleted = original_analysis(source);
        index.replace_document(uri, &deleted);
        assert_eq!(index.original_class_declarations().count(), 1);
        assert!(index.original_class_source_candidates().next().is_none());
        assert_eq!(
            index.docs[0]
                .settlement_dependencies()
                .original_source_class_candidates,
            [] as [(
                tcl_core_types::CommandSlot<
                    tcl_core_types::NameBytes,
                    tcl_core_types::ByteNamespacePath,
                >,
                tcl_syntax::naming::NamePolicyProtocol,
                tcl_core_types::CommandSlot<
                    tcl_core_types::NameBytes,
                    tcl_core_types::ByteNamespacePath,
                >,
                std::vec::Vec<tcl_compiler::command_binding::SourceCommandTransitionObligation>
            ); 0]
        );
    }
}

#[test]
fn original_source_class_candidates_require_the_retained_input_and_availability() {
    // naming.source.original-class-publications
    // docs/design/analysis/name-resolution-proofs/source-original-class-publications.md
    let source = "oo::class create C {}";
    let uri = "file:///source-classes.tcl";
    let analysis = original_analysis(source);
    let mut index = WorkspaceIndex::from_documents([(uri, &analysis)]);
    assert_eq!(index.original_class_source_candidates().count(), 1);
    let input = analysis.resolved_input.as_ref().unwrap();
    let generation = input.context_registry();
    let foreign =
        Arc::new(generation.with_command_store(generation.commands().snapshot().shared_registry()));
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    let mut changed = analysis.clone();
    changed.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        foreign,
        input.lexer_config(),
    ));
    let mut wrong_config = analysis.clone();
    wrong_config
        .body_lexer_config
        .as_mut()
        .unwrap()
        .strict_quoting = !analysis.body_lexer_config.unwrap().strict_quoting;
    for refused in [missing, changed, wrong_config] {
        index.replace_document(uri, &refused);
        assert_eq!(index.original_class_declarations().count(), 1);
        assert!(index.original_class_source_candidates().next().is_none());
    }
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(generation.commands())),
    );
    let supplied = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        older,
        input.lexer_config(),
    );
    let unavailable = Analyser::new()
        .with_resolved_input(supplied)
        .analyse(source, "tcl8.6");
    index.replace_document(uri, &unavailable);
    assert!(index.original_class_source_candidates().next().is_none());
    assert_eq!(index.original_class_declarations().count(), 0);
    assert!(index.docs[0].original_source_classes.is_some());
}

#[test]
fn original_reverse_index_keeps_opaque_slots_after_reporting_is_cleared() {
    let mut library = original_analysis("proc p\\uD800 {} {}\nproc p\\uD801 {} {}\n");
    let mut caller = original_analysis("p\\uD800\np\\uD801\n");
    let names = original_names(&library);
    assert_eq!(names.len(), 2);
    assert_ne!(names[0].slot(), names[1].slot());
    clear_reporting(&mut library);
    clear_reporting(&mut caller);
    let index = WorkspaceIndex::from_documents([
        ("file:///library.tcl", &library),
        ("file:///caller.tcl", &caller),
    ]);
    for name in &names {
        let rows = index.original_invocations_of(name, "", false);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].uri, "file:///caller.tcl");
        assert_eq!(
            rows[0].original_name_input.as_ref().unwrap().bytes(),
            name.slot().simple.as_bytes()
        );
        assert_eq!(
            index.original_invocations_of(name, "file:///caller.tcl", false),
            [] as [&crate::workspace_index::WorkspaceInvocation; 0]
        );
    }
    assert_eq!(index.settled_invocations[0].settled_documents(), 2);
}

#[test]
fn lexical_rename_advice_requires_every_captured_provider_and_refuses_vendor_contexts() {
    // Implementation contract: naming.workspace.lexical-rename-advice-eligibility
    // docs/design/analysis/name-resolution-proofs/workspace-lexical-rename-advice-eligibility.md
    let point = tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79);
    let profile = tcl_dialect::DialectProfile::projected_from_point(
        "indexed-logical-jim079",
        &[],
        "Logical Jim source advice",
        point,
    )
    .intern();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::context_for_profile(profile),
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    let logical = Analyser::new()
        .with_resolved_input(input)
        .analyse("proc lexical {} {}", profile.name);
    assert!(logical.allows_lexical_declaration_advice());
    assert!(!WorkspaceIndex::new().allows_lexical_rename_advice());
    let mut index = WorkspaceIndex::from_documents([("file:///logical.tcl", &logical)]);
    assert!(index.allows_lexical_rename_advice());
    assert!(index.clone().allows_lexical_rename_advice());
    let mut native = Analyser::new().analyse(r"proc p\uD800 {} {}", "tcl8.6");
    native.dialect = profile.name.to_owned();
    index.add_document("file:///native.tcl", &native);
    assert!(!index.allows_lexical_rename_advice());
    index.remove_document("file:///native.tcl");
    assert!(index.allows_lexical_rename_advice());
    for source in ["proc vendor {} {}", ""] {
        let mut vendor = Analyser::new().analyse(source, "f5-irules");
        vendor.dialect = profile.name.to_owned();
        index.add_document("file:///vendor.tcl", &vendor);
        assert!(!index.allows_lexical_rename_advice(), "{source:?}");
        index.remove_document("file:///vendor.tcl");
        assert!(index.allows_lexical_rename_advice());
    }
    index.add_document("file:///unknown.tcl", &AnalysisResult::default());
    assert!(!index.allows_lexical_rename_advice());
    index.remove_document("file:///unknown.tcl");
    assert!(index.allows_lexical_rename_advice());
}

#[test]
fn host_roster_must_be_complete_before_lexical_rename_advice_is_allowed() {
    let point = tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79);
    let profile = tcl_dialect::DialectProfile::projected_from_point(
        "inventory-logical-jim079",
        &[],
        "Logical source advice",
        point,
    )
    .intern();
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        tcl_registry::model::ingress::context_for_profile(profile),
        tcl_lexer::LexerConfig::for_profile(Some(profile)),
    );
    let analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse("proc lexical {} {}", profile.name);
    let uri = "file:///workspace/lexical.tcl";
    let mut index = WorkspaceIndex::from_documents([(uri, &analysis)]);
    let store = crate::vfs::MemoryStore::new();
    let root = std::path::Path::new("/workspace");

    store.set_workspace_source_inventory(
        root,
        "selection-v1",
        crate::vfs::WorkspaceSourceInventoryCompleteness::Incomplete,
    );
    index.set_source_inventory_snapshot(
        crate::vfs::WorkspaceSourceInventoryProvider::HostReported,
        vec![store.workspace_source_inventory(root)],
    );
    assert!(!index.allows_lexical_rename_advice());
    assert!(!index.source_inventory_matches_store(&store));

    store.set_workspace_source_inventory(
        root,
        "selection-v1",
        crate::vfs::WorkspaceSourceInventoryCompleteness::Complete,
    );
    index.set_source_inventory_snapshot(
        crate::vfs::WorkspaceSourceInventoryProvider::HostReported,
        vec![store.workspace_source_inventory(root)],
    );
    assert!(index.allows_lexical_rename_advice());
    assert!(index.source_inventory_matches_store(&store));
    index.set_source_inventory_snapshot_with_scan_status(
        crate::vfs::WorkspaceSourceInventoryProvider::HostReported,
        vec![store.workspace_source_inventory(root)],
        false,
    );
    assert!(
        !index.allows_lexical_rename_advice(),
        "a complete host roster cannot close a truncated server index",
    );
    assert!(
        !index.source_inventory_matches_store(&store),
        "current host bytes do not prove the server indexed the full roster",
    );
    index.set_source_inventory_snapshot_with_scan_status(
        crate::vfs::WorkspaceSourceInventoryProvider::HostReported,
        vec![store.workspace_source_inventory(root)],
        true,
    );
    assert!(index.allows_lexical_rename_advice());
    store.set_workspace_source_inventory(
        root,
        "selection-v2",
        crate::vfs::WorkspaceSourceInventoryCompleteness::Complete,
    );
    assert!(index.allows_lexical_rename_advice());
    assert!(
        !index.source_inventory_matches_store(&store),
        "the final gate must reject a new host selection not captured by this index",
    );
}

#[test]
fn logical_declaration_source_views_use_original_occurrences_and_current_input() {
    let source = "proc helper {} {}; proc helper {} {}; proc ::fixed {} {}";
    let standalone = Analyser::new().analyse(source, "tcl");
    assert!(standalone.allows_retained_logical_declaration_advice());
    assert_eq!(standalone.original_procedure_declarations().count(), 3);
    let first = Analyser::new().analyse_with_source_namespace(source, "tcl", "::x");
    let second = Analyser::new().analyse_with_source_namespace(source, "tcl", "::y");
    let uri = "file:///source-views.tcl";
    let mut index = WorkspaceIndex::from_documents([(uri, &first), (uri, &second)]);
    for offset in [5, 24] {
        assert_eq!(
            index.logical_declaration_source_names_at(uri, source, &standalone, offset),
            Some(vec!["::x::helper".to_owned(), "::y::helper".to_owned()])
        );
    }
    assert_eq!(
        index.logical_declaration_source_names_at(uri, source, &standalone, 44),
        Some(vec!["::fixed".to_owned()])
    );
    assert_eq!(
        index.logical_declaration_source_names_at("file:///other.tcl", source, &standalone, 5),
        Some(Vec::new())
    );
    assert_eq!(
        index.logical_declaration_source_names_at(uri, "proc helper {} {changed}", &standalone, 5),
        Some(Vec::new())
    );
    let native = Analyser::new().analyse(source, "tcl8.6");
    assert_eq!(
        index.logical_declaration_source_names_at(uri, source, &native, 5),
        Some(Vec::new())
    );
    index.replace_document(
        uri,
        &Analyser::new().analyse("proc helper {} {changed}", "tcl"),
    );
    assert_eq!(
        index.logical_declaration_source_names_at(uri, source, &standalone, 5),
        Some(Vec::new())
    );
    index.remove_document(uri);
    assert_eq!(
        index.logical_declaration_source_names_at(uri, source, &standalone, 5),
        Some(Vec::new())
    );
}

#[test]
fn workspace_symbols_require_current_source_context_before_report_advice() {
    // naming.editor.original-indexed-source-location
    // docs/design/analysis/name-resolution-proofs/original-indexed-source-location.md
    // naming.minifier.complete-logical-metadata
    // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
    let source = "proc visible {} {}";
    let uri = "file:///symbols.tcl";
    let logical = Analyser::new().analyse(source, "tcl");
    assert!(logical.allows_lexical_declaration_advice());
    let mut index = WorkspaceIndex::from_documents([(uri, &logical)]);
    let symbols = index.symbols_matching("visible", 100);
    assert_eq!(symbols.len(), 1);
    assert!(symbols[0].original_location.is_none());
    let logical_source = symbols[0]
        .logical_source
        .as_ref()
        .expect("Logical report candidate retains its source image and config");
    assert_eq!(logical_source.uri(), uri);
    assert_eq!(logical_source.span(), symbols[0].name_span);
    assert_eq!(
        logical_source.document_revision(),
        index.document_revision(uri).unwrap()
    );
    assert!(logical_source.matches_source(uri, source));
    assert!(!logical_source.matches_source(uri, "proc other {} {}"));
    assert!(!logical_source.matches_source("file:///other.tcl", source));
    let native = Analyser::new().analyse(source, "tcl8.6");
    index.replace_document(uri, &native);
    let symbols = index.symbols_matching("visible", 100);
    assert_eq!(symbols.len(), 1);
    assert!(symbols[0].original_location.is_some());
    assert!(symbols[0].logical_source.is_none());
    for mut refused in [logical, native] {
        assert!(!refused.all_procs.is_empty());
        refused.resolved_input = None;
        index.replace_document(uri, &refused);
        assert!(index.diagnostic_source_context(uri).is_none());
        assert_eq!(
            index.symbols_matching("visible", 100),
            [] as [crate::workspace_symbols::IndexedWorkspaceSymbol; 0]
        );
    }
    let mut stale_config = Analyser::new().analyse(source, "tcl8.6");
    let config = stale_config.body_lexer_config.as_mut().unwrap();
    config.strict_quoting = !config.strict_quoting;
    index.replace_document(uri, &stale_config);
    assert!(index.diagnostic_source_context(uri).is_none());
    assert_eq!(
        index.symbols_matching("visible", 100),
        [] as [crate::workspace_symbols::IndexedWorkspaceSymbol; 0]
    );
    index.remove_document(uri);
    index.add_document(uri, &AnalysisResult::default());
    assert_eq!(
        index.symbols_matching("", 100),
        [] as [crate::workspace_symbols::IndexedWorkspaceSymbol; 0]
    );
}

#[test]
fn diagnostic_source_context_retains_currency_and_clears_reused_slots() {
    // Implementation contract: naming.editor.original-diagnostic-invalidation
    // docs/design/analysis/name-resolution-proofs/original-diagnostic-invalidation.md
    let uri = "file:///provider.tcl";
    let source = "proc p\\uD800 {} {}";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    let mut index = WorkspaceIndex::from_documents([(uri, &analysis)]);
    let context = index.diagnostic_source_context(uri).unwrap();
    assert_eq!(context.image(), &tcl_lexer::SourceImage::document(source));
    assert_eq!(context.config(), analysis.body_lexer_config.unwrap());
    assert!(context.uses_original_names());
    assert_eq!(index.clone().diagnostic_source_context(uri), Some(context));
    index.remove_document(uri);
    assert!(index.diagnostic_source_context(uri).is_none());
    index.add_document("file:///reused.tcl", &AnalysisResult::default());
    assert!(
        index
            .diagnostic_source_context("file:///reused.tcl")
            .is_none()
    );
    let mut stale_config = analysis.clone();
    let mut config = stale_config.body_lexer_config.unwrap();
    config.strict_quoting = !config.strict_quoting;
    stale_config.body_lexer_config = Some(config);
    index.replace_document(uri, &stale_config);
    assert!(index.diagnostic_source_context(uri).is_none());
}

#[test]
fn original_reverse_index_does_not_replace_missing_lookup_with_reports() {
    let mut library = original_analysis("proc p\\uD800 {} {}\n");
    let mut caller = original_analysis("p\\uD800\n");
    let name = original_names(&library).remove(0);
    let invocation = caller
        .command_invocations
        .iter_mut()
        .find(|invocation| {
            invocation
                .original_name_input
                .as_ref()
                .is_some_and(|input| input.bytes() == b"p\xed\xa0\x80")
        })
        .unwrap();
    assert!(invocation.original_lookup.is_some());
    invocation.original_lookup = None;
    invocation.resolved_command_reference = None;
    invocation.resolved_definition = None;
    invocation.resolved_qualified_name = Some("::p\u{fffd}".to_owned());
    invocation.resolution_candidates = vec!["::p\u{fffd}".to_owned()];
    clear_reporting(&mut library);
    let index = WorkspaceIndex::from_documents([
        ("file:///library.tcl", &library),
        ("file:///caller.tcl", &caller),
    ]);
    assert_eq!(
        index.original_invocations_of(&name, "", false),
        [] as [&crate::workspace_index::WorkspaceInvocation; 0]
    );
    assert_eq!(
        index.original_invocations_of(&name, "", true),
        [] as [&crate::workspace_index::WorkspaceInvocation; 0]
    );
}

#[test]
fn original_reverse_index_rejects_input_from_another_retained_source() {
    let library = original_analysis("proc p\\uD800 {} {}\n");
    let mut caller = original_analysis("p\\uD800\n");
    let other = original_analysis("p\\uD801\n");
    let name = original_names(&library).remove(0);
    let other_input = other
        .command_invocations
        .iter()
        .find_map(|invocation| invocation.original_name_input.clone())
        .unwrap();
    let invocation = caller
        .command_invocations
        .iter_mut()
        .find(|invocation| invocation.original_lookup.is_some())
        .unwrap();
    assert_ne!(invocation.original_name_input.as_ref(), Some(&other_input));
    invocation.original_name_input = Some(other_input);
    invocation.resolved_command_reference = None;
    invocation.resolved_definition = None;
    let index = WorkspaceIndex::from_documents([
        ("file:///library.tcl", &library),
        ("file:///caller.tcl", &caller),
    ]);
    assert_eq!(
        index.original_invocations_of(&name, "", false),
        [] as [&crate::workspace_index::WorkspaceInvocation; 0]
    );
}

#[test]
fn original_body_only_replacement_keeps_minimal_settlement_dependencies() {
    let mut library = original_analysis("proc p\\uD800 {} {return FIRST}\nproc p\\uD801 {} {}\n");
    let mut caller = original_analysis("p\\uD800\np\\uD801\n");
    let names = original_names(&library);
    clear_reporting(&mut library);
    clear_reporting(&mut caller);
    let mut index = WorkspaceIndex::from_documents([
        ("file:///library.tcl", &library),
        ("file:///caller.tcl", &caller),
    ]);
    assert!(library.original_completed_command_world().is_some());
    for name in &names {
        assert_eq!(index.original_invocations_of(name, "", false).len(), 1);
    }
    let cold = index.settled_invocations[0].settled_documents();
    let old_dependencies = index.docs[0].settlement_dependencies();
    let mut replacement =
        original_analysis("proc p\\uD800 {} {return SECOND}\nproc p\\uD801 {} {}\n");
    clear_reporting(&mut replacement);
    index.replace_document("file:///library.tcl", &replacement);
    assert!(
        old_dependencies == index.docs[0].settlement_dependencies(),
        "body/source-image changes cannot enter the settlement dependency header"
    );
    for name in &names {
        assert_eq!(index.original_invocations_of(name, "", false).len(), 1);
    }
    assert_eq!(
        index.settled_invocations[0].settled_documents(),
        cold + 1,
        "only the changed document needs settlement when exact published slots are unchanged"
    );
    let rebuilt = WorkspaceIndex::from_documents([
        ("file:///library.tcl", &replacement),
        ("file:///caller.tcl", &caller),
    ]);
    for name in &names {
        assert_eq!(
            index.original_invocations_of(name, "", false).len(),
            rebuilt.original_invocations_of(name, "", false).len()
        );
    }
    let mut removed = original_analysis("proc p\\uD801 {} {}\n");
    clear_reporting(&mut removed);
    index.replace_document("file:///library.tcl", &removed);
    assert_eq!(
        index.original_invocations_of(&names[0], "", false),
        [] as [&crate::workspace_index::WorkspaceInvocation; 0]
    );
    assert_eq!(index.original_invocations_of(&names[1], "", false).len(), 1);
    assert_eq!(
        index.settled_invocations[0].settled_documents(),
        cold + 3,
        "changing an original publication re-settles the caller too"
    );
}
