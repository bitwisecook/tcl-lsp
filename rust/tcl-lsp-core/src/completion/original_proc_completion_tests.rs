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

#[test]
fn logical_proc_completion_inserts_original_headers_after_report_labels_change() {
    let source = "namespace eval N {proc helper {arg} {return $arg}}";
    let mut analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl");
    let original = proc_completions(&analysis, "hel", &FxHashMap::default());
    assert_eq!(original.len(), 1);
    assert_eq!(original[0].label, "helper");
    assert_eq!(original[0].insert_text, "::N::helper");
    let procedure = analysis.all_procs.get_mut("::N::helper").unwrap();
    procedure.name = "display".into();
    procedure.qualified_name = "::foreign::display".into();
    let current = proc_completions(&analysis, "hel", &FxHashMap::default());
    assert_eq!(current, original);
    analysis
        .all_procs
        .get_mut("::N::helper")
        .unwrap()
        .params
        .clear();
    assert_eq!(
        proc_completions(&analysis, "hel", &FxHashMap::default()),
        [] as [crate::completion::CompletionItem; 0]
    );
}
use tcl_compiler::analyser::Analyser;

fn inserted_slot(
    item: &CompletionItem,
    config: tcl_lexer::LexerConfig,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> tcl_core_types::ByteCommandSlot {
    let image = tcl_lexer::SourceImage::document(&item.insert_text);
    let command = tcl_lexer::native_script_words_in(
        image,
        tcl_lexer::Span::new(0, u32::try_from(item.insert_text.len()).unwrap()),
        config,
    )
    .unwrap();
    assert_eq!(command.commands.len(), 1);
    assert_eq!(command.commands[0].words.len(), 1);
    let key = tcl_names::word::SignatureSourceNameKey::from_original_native_word(
        &command.commands[0].words[0],
        tcl_syntax::word_rules::WordValueRules::from_config(&config),
        policy,
    )
    .unwrap();
    tcl_compiler::signature_scan::scope::SignatureSourceCommand::procedure_from_key(
        &tcl_names::namespace::SignatureNamespaceScope::C(
            tcl_core_types::ByteNamespacePath::root(),
        ),
        &key,
    )
    .unwrap()
    .slot()
    .clone()
}

#[test]
fn opaque_completion_names_survive_ui_collapse_and_insert_distinct_native_words() {
    let source = r"proc p\uD800 {} {}; proc p\uD801 {} {}";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    let expected = analysis
        .original_procedure_declarations()
        .map(|declaration| declaration.name().slot().clone())
        .collect::<Vec<_>>();
    assert_eq!(expected.len(), 2);
    let policy = analysis
        .original_procedure_declarations()
        .next()
        .unwrap()
        .name()
        .policy();
    analysis.all_procs.clear();
    analysis.superseded_procs.clear();
    let items = proc_completions(&analysis, "", &FxHashMap::default());
    assert_eq!(items.len(), 2);
    let actual = items
        .iter()
        .map(|item| inserted_slot(item, analysis.body_lexer_config.unwrap(), policy))
        .collect::<Vec<_>>();
    assert_ne!(actual[0], actual[1]);
    assert!(expected.iter().all(|slot| actual.contains(slot)));
}

#[test]
fn moved_and_deleted_completions_use_conditional_source_publication_slots() {
    let source = "proc old {value} {}; proc deleted {} {}; rename old moved; rename deleted {}";
    let mut analysis = Analyser::new().analyse(source, "tcl8.6");
    // naming.source.original-procedure-publications
    // docs/design/analysis/name-resolution-proofs/source-original-procedure-publications.md
    let inventory =
        tcl_compiler::registry_invocation::source_structure::source_procedure_publications(
            source, &analysis,
        )
        .unwrap();
    assert_eq!(inventory.publications().len(), 1);
    assert!(inventory.publications()[0].obligations().contains(
        &tcl_compiler::command_binding::SourceCommandTransitionObligation::UnavailableActualLookup
    ));
    analysis.all_procs.clear();
    let items = proc_completions(&analysis, "", &FxHashMap::default());
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].label, "moved");
    assert_eq!(items[0].detail.as_deref(), Some("value"));
    assert_eq!(
        proc_completions(&analysis, "old", &FxHashMap::default()),
        [] as [crate::completion::CompletionItem; 0]
    );
    let policy = analysis
        .original_procedure_declarations()
        .next()
        .unwrap()
        .name()
        .policy();
    assert_eq!(
        inserted_slot(&items[0], analysis.body_lexer_config.unwrap(), policy)
            .simple
            .as_bytes(),
        b"moved"
    );
}

#[test]
fn document_completion_unicode_uses_the_selected_ingress_recipe() {
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let analysis = Analyser::new().analyse("proc café😀 {} {}", engine);
        let declaration = analysis.original_procedure_declarations().next().unwrap();
        let items = proc_completions(&analysis, "", &FxHashMap::default());
        assert_eq!(items.len(), 1, "{engine}");
        assert_eq!(
            inserted_slot(
                &items[0],
                declaration.name_input().lexer_config(),
                declaration.name().policy()
            ),
            *declaration.name().slot(),
            "{engine}"
        );
    }
}

#[test]
fn original_hosted_proc_completion_preserves_source_candidates_without_a_native_recipe() {
    let mut analysis = Analyser::new().analyse("proc greet {value} {return $value}", "f5-irules");
    assert!(!analysis.allows_lexical_declaration_advice());
    assert!(analysis.has_original_vendor_source_names());
    assert!(analysis.original_procedure_declarations().next().is_none());
    assert_eq!(analysis.original_vendor_procedure_declarations().count(), 1);
    analysis.all_procs.clear();
    analysis.global_scope.procs.clear();
    let items = proc_completions(&analysis, "gre", &FxHashMap::default());
    assert_eq!(items.len(), 1, "{items:?}");
    assert_eq!(items[0].label, "greet");
    assert_eq!(items[0].insert_text, "greet");
    assert!(
        items[0]
            .detail
            .as_deref()
            .unwrap()
            .starts_with("Source declaration:")
    );
}

#[test]
fn original_vendor_command_completion_keeps_owned_unknown_names_out_of_suggestions() {
    // Implementation contract: naming.vendor.original-source-declaration-consumers
    // docs/design/analysis/name-resolution-proofs/vendor-original-source-declaration-consumers.md
    let source = "proc helper {argument} {return $argument}\nproc p\\uD800 {} {}\nhel";
    let mut analysis = Analyser::new().analyse(source, "f5-iapps");
    assert_eq!(analysis.original_vendor_procedure_declarations().count(), 2);
    analysis.all_procs.clear();
    analysis.global_scope.procs.clear();
    let profile = crate::profile_for_analysis(&analysis);
    let items = completions(source, 2, 3, &analysis, None, None, profile);
    let helper = items.iter().find(|item| item.label == "helper").unwrap();
    assert_eq!(helper.insert_text, "helper");
    assert!(
        helper
            .detail
            .as_deref()
            .unwrap()
            .starts_with("Source declaration:")
    );
    assert!(items.iter().all(|item| !item.label.contains("D800")));
    assert_eq!(
        completions(
            &format!("# displaced\n{source}"),
            2,
            3,
            &analysis,
            None,
            None,
            profile
        ),
        [] as [crate::completion::CompletionItem; 0]
    );
}

#[test]
fn eda_proc_completion_uses_the_selected_authored_tcl_layer() {
    let mut analysis =
        Analyser::new().analyse("proc greet {value} {return $value}", "xilinx-eda-tcl");
    assert!(!analysis.allows_lexical_declaration_advice());
    let declaration = analysis.original_procedure_declarations().next().unwrap();
    let policy = declaration.name().policy();
    assert_eq!(
        policy.authority(),
        tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
    );
    assert_eq!(
        policy.recipe(),
        tcl_syntax::naming::NativeNameProtocol::C(tcl_dialect::TclVersion::V8_5)
    );
    let expected = declaration.name().slot().clone();
    analysis.all_procs.clear();
    let items = proc_completions(&analysis, "gre", &FxHashMap::default());
    assert_eq!(items.len(), 1, "{items:?}");
    assert_eq!(items[0].label, "greet");
    assert_eq!(
        inserted_slot(&items[0], analysis.body_lexer_config.unwrap(), policy),
        expected
    );
}

#[test]
fn byte_policy_failure_cannot_fall_back_to_display_declaration_advice() {
    let mut analysis = Analyser::new().analyse(r"proc p\uD800 {} {}", "tcl8.6");
    assert!(!analysis.allows_lexical_declaration_advice());
    analysis.original_procedure_metadata.clear();
    for proc_def in analysis.all_procs.values_mut() {
        proc_def.source_name = None;
    }
    assert_eq!(
        proc_completions(&analysis, "", &FxHashMap::default()),
        [] as [crate::completion::CompletionItem; 0]
    );
    let mut unsupported = Analyser::new().analyse("proc café {} {}", "f5-irules");
    assert!(!unsupported.allows_lexical_declaration_advice());
    assert!(unsupported.has_original_vendor_source_names());
    assert_eq!(
        unsupported.original_vendor_procedure_declarations().count(),
        1
    );
    assert!(
        unsupported
            .original_vendor_procedure_declarations()
            .next()
            .unwrap()
            .name_input()
            .literal_units(tcl_syntax::naming::VendorSourceNamePurpose::ProcedureName)
            .is_none()
    );
    assert_eq!(
        proc_completions(&unsupported, "", &FxHashMap::default()),
        [] as [crate::completion::CompletionItem; 0]
    );
    unsupported.body_lexer_config = None;
    assert_eq!(
        proc_completions(&unsupported, "", &FxHashMap::default()),
        [] as [crate::completion::CompletionItem; 0]
    );
}
