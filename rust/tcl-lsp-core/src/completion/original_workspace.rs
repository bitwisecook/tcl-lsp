// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Workspace source candidates retain the declaring bytes and the consumer's
//! independently selected source protocol. These suggestions grant no live
//! command existence, visibility, successful dispatch or edit capability.

use super::{CompletionItem, CompletionKind};
use crate::workspace_index::WorkspaceIndex;
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::signature_scan::scope::SignatureSourceNameInput;
use tcl_lexer::SourceImage;

pub(super) fn items(
    source: &str,
    cursor: u32,
    analysis: &AnalysisResult,
    workspace: &WorkspaceIndex,
    partial: &str,
) -> Vec<CompletionItem> {
    let Some(config) = analysis.body_lexer_config else {
        return Vec::new();
    };
    let image = SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, config) {
        return Vec::new();
    }
    if analysis.allows_lexical_declaration_advice() {
        if !workspace.allows_lexical_rename_advice() {
            return Vec::new();
        }
        let mut items = Vec::new();
        for proc in workspace.procs_matching(partial, "") {
            if analysis
                .all_procs
                .values()
                .any(|local| local.qualified_name == proc.qualified_name)
            {
                continue;
            }
            let Some(spelling) =
                tcl_syntax::word_rules::lexical_ascii_source_word(&proc.qualified_name, config)
            else {
                continue;
            };
            items.push(CompletionItem {
                label: proc.name.clone(),
                insert_text: spelling,
                kind: CompletionKind::Function,
                detail: Some(format!("{} params (workspace)", proc.param_count)),
                sort_text: Some(format!("C0_{}", proc.name)),
                ..CompletionItem::default()
            });
        }
        items.sort_by(|left, right| left.insert_text.cmp(&right.insert_text));
        items.dedup_by(|left, right| {
            left.insert_text == right.insert_text && left.detail == right.detail
        });
        return items;
    }
    // A hosted producer remains a separate purpose, including when its units
    // are unavailable. It cannot acquire a C or Jim protocol from a sibling.
    if analysis.has_original_vendor_source_names() {
        return vendor_items(source, cursor, analysis, workspace, partial);
    }
    let mut policy = None;
    for invocation in &analysis.command_invocations {
        if invocation.range.start() > cursor
            || invocation.range.end() < cursor
            || invocation.callback_arity.is_some()
            || invocation.is_mathfunc_call
        {
            continue;
        }
        let Some(input @ SignatureSourceNameInput::OriginalWord(_)) =
            invocation.original_name_input.as_ref()
        else {
            return Vec::new();
        };
        if !crate::original_name_edit::original_input_matches_source(
            source,
            analysis,
            input,
            invocation.range,
        ) {
            return Vec::new();
        }
        if policy
            .replace(input.policy())
            .is_some_and(|previous| previous != input.policy())
        {
            return Vec::new();
        }
    }
    let Some(policy) = policy else {
        return Vec::new();
    };
    let mut items = Vec::new();
    for (uri, declaration, slot, selected_policy) in
        workspace.original_procedure_source_candidates()
    {
        let input = declaration.name_input();
        let Some(context) = workspace.diagnostic_source_context(uri) else {
            continue;
        };
        if selected_policy != policy
            || input.policy() != policy
            || input.source_image() != context.image()
            || input.lexer_config() != context.config()
        {
            continue;
        }
        // Avoid repeating an identical local insertion candidate. Distinct
        // counted slots and independent foreign declarations stay separate.
        if analysis
            .original_procedure_declarations()
            .any(|local| local == declaration)
        {
            continue;
        }
        let Some(spelling) = tcl_syntax::naming::native_command_source_word(
            policy.recipe(),
            slot,
            image.channel(),
            config,
        ) else {
            continue;
        };
        let metadata = declaration.metadata();
        // Labels display the counted slot; only the separately checked source
        // word above can be inserted into a command.
        let label = tcl_syntax::native_string::resident_name_label(slot.simple.as_bytes());
        if !label.starts_with(partial) && !spelling.starts_with(partial) {
            continue;
        }
        items.push(CompletionItem {
            label: label.clone(),
            insert_text: spelling,
            kind: CompletionKind::Function,
            detail: Some(format!(
                "Source declaration: {} (workspace: {uri})",
                super::proc_signature_str(metadata)
            )),
            sort_text: Some(format!("C0_{label}")),
            ..CompletionItem::default()
        });
    }
    items.sort_by(|left, right| {
        left.insert_text
            .cmp(&right.insert_text)
            .then_with(|| left.detail.cmp(&right.detail))
    });
    items
}

fn vendor_items(
    source: &str,
    cursor: u32,
    analysis: &AnalysisResult,
    workspace: &WorkspaceIndex,
    partial: &str,
) -> Vec<CompletionItem> {
    let Some(config) = analysis.body_lexer_config else {
        return Vec::new();
    };
    let image = SourceImage::document(source);
    let mut policy = None;
    for occurrence in analysis.original_vendor_source_names() {
        let input = occurrence.name_input();
        if input.span().start() > cursor || input.span().end() < cursor {
            continue;
        }
        if occurrence.original_words().first() != Some(input.original_word()) {
            continue;
        }
        if !input.matches_source(&image, config)
            || input
                .literal_units(tcl_syntax::naming::VendorSourceNamePurpose::CommandHead)
                .is_none()
            || policy
                .replace(input.policy())
                .is_some_and(|previous| previous != input.policy())
        {
            return Vec::new();
        }
    }
    let Some(policy) = policy else {
        return Vec::new();
    };
    let mut items = Vec::new();
    for (uri, context, record) in workspace.original_vendor_declarations() {
        let card = record.declaration();
        let Some(metadata) = card.procedure_metadata() else {
            continue;
        };
        let input = card.input();
        if input.policy() != policy || !input.matches_source(context.image(), context.config()) {
            continue;
        }
        if analysis
            .original_vendor_procedure_declarations()
            .any(|local| local.name_input() == input && local.metadata() == metadata)
        {
            continue;
        }
        let Some(spelling) = crate::vendor_declaration::source_word(input, card.purpose(), config)
        else {
            continue;
        };
        let Some(label) = crate::vendor_declaration::source_label(input, card.purpose()) else {
            continue;
        };
        if !label.starts_with(partial) && !spelling.starts_with(partial) {
            continue;
        }
        items.push(CompletionItem {
            label: label.clone(),
            insert_text: spelling,
            kind: CompletionKind::Function,
            detail: Some(format!(
                "Hosted source declaration: {} (workspace: {uri})",
                super::proc_signature_str(metadata)
            )),
            sort_text: Some(format!("C0_{label}")),
            ..CompletionItem::default()
        });
    }
    items.sort_by(|left, right| {
        left.insert_text
            .cmp(&right.insert_text)
            .then_with(|| left.detail.cmp(&right.detail))
    });
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    // Implementation contract: naming.core.original-workspace-procedure-completion
    // docs/design/analysis/name-resolution-proofs/original-workspace-procedure-completion.md
    #[test]
    fn original_workspace_completion_uses_shared_conditional_procedure_slots() {
        // naming.source.original-procedure-publications
        // docs/design/analysis/name-resolution-proofs/source-original-procedure-publications.md
        // naming.core.original-workspace-procedure-completion
        // docs/design/analysis/name-resolution-proofs/original-workspace-procedure-completion.md
        let source = "m\n";
        let consumer = Analyser::new().analyse(source, "tcl8.6");
        let producer_source = "proc original {value} {}; proc removed {} {}; rename original moved; rename removed {}";
        let mut producer = Analyser::new().analyse(producer_source, "tcl8.6");
        assert!(producer.original_completed_command_world().is_none());
        producer.all_procs.clear();
        producer.superseded_procs.clear();
        let mut index = WorkspaceIndex::from_documents([("file:///producer.tcl", &producer)]);
        let suggestions = items(source, 1, &consumer, &index, "");
        assert_eq!(suggestions.len(), 1, "{suggestions:?}");
        assert_eq!(suggestions[0].label, "moved");
        assert!(suggestions[0].detail.as_ref().unwrap().contains("value"));
        let replacement =
            Analyser::new().analyse("proc original {value} {}; rename original other", "tcl8.6");
        index.replace_document("file:///producer.tcl", &replacement);
        let suggestions = items(source, 1, &consumer, &index, "");
        assert_eq!(suggestions.len(), 1, "{suggestions:?}");
        assert_eq!(suggestions[0].label, "other");
        index.remove_document("file:///producer.tcl");
        assert!(items(source, 1, &consumer, &index, "").is_empty());
    }

    #[test]
    fn original_workspace_completion_keeps_opaque_slots_and_source_spelling_after_ui_clear() {
        // Implementation contract: naming.core.original-workspace-procedure-completion
        // docs/design/analysis/name-resolution-proofs/original-workspace-procedure-completion.md
        let source = "p\n";
        let mut consumer = Analyser::new().analyse(source, "tcl8.6");
        let producer_source = r"proc p\uD800 {} {}; proc p\uD801 {} {}; proc {p $[literal]} {} {}";
        let mut producer = Analyser::new().analyse(producer_source, "tcl8.6");
        let expected = producer
            .original_procedure_declarations()
            .map(|row| row.name().slot().clone())
            .collect::<Vec<_>>();
        assert_eq!(expected.len(), 3);
        consumer.all_procs.clear();
        producer.all_procs.clear();
        producer.global_scope.procs.clear();
        let index = WorkspaceIndex::from_documents([("file:///producer.tcl", &producer)]);
        let suggestions = items(source, 1, &consumer, &index, "p");
        assert_eq!(suggestions.len(), 3, "{suggestions:?}");
        let expected_labels = expected
            .iter()
            .map(|slot| tcl_syntax::native_string::resident_name_label(slot.simple.as_bytes()))
            .collect::<Vec<_>>();
        assert!(
            suggestions
                .iter()
                .all(|item| expected_labels.contains(&item.label))
        );
        assert!(suggestions.iter().all(|item| item.label.starts_with('p')));
        assert_ne!(suggestions[0].label, suggestions[1].label);
        let policy = producer
            .original_procedure_declarations()
            .next()
            .unwrap()
            .name()
            .policy();
        let config = consumer.body_lexer_config.unwrap();
        let actual = suggestions.iter().map(|item| {
            let image = SourceImage::document(&item.insert_text);
            let plan = tcl_lexer::native_script_words_in(image,tcl_lexer::Span::new(0,u32::try_from(item.insert_text.len()).unwrap()),config).unwrap();
            assert_eq!(plan.commands.len(),1); assert_eq!(plan.commands[0].words.len(),1);
            let key = tcl_compiler::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                &plan.commands[0].words[0], tcl_syntax::word_rules::WordValueRules::from_config(&config),policy).unwrap();
            tcl_compiler::signature_scan::scope::SignatureSourceCommand::procedure_from_key(
                &tcl_compiler::signature_scan::scope::SignatureNamespaceScope::C(tcl_core_types::ByteNamespacePath::root()),&key).unwrap().slot().clone()
        }).collect::<Vec<_>>();
        assert!(expected.iter().all(|slot| actual.contains(slot)));
        assert_ne!(actual[0], actual[1]);
        assert!(items("# changed\np\n", 1, &consumer, &index, "p").is_empty());
        consumer.command_invocations.clear();
        assert!(items(source, 1, &consumer, &index, "p").is_empty());
    }

    // Implementation contract: naming.core.original-workspace-procedure-completion
    // docs/design/analysis/name-resolution-proofs/original-workspace-procedure-completion.md
    #[test]
    fn original_workspace_completion_requires_the_consumers_own_protocol() {
        // Implementation contract: naming.core.original-workspace-procedure-completion
        // docs/design/analysis/name-resolution-proofs/original-workspace-procedure-completion.md
        let producer = Analyser::new().analyse("proc public {} {}", "tcl8.6");
        let index = WorkspaceIndex::from_documents([("file:///producer.tcl", &producer)]);
        for dialect in ["jimtcl", "tcl9.0", "f5-irules"] {
            let consumer = Analyser::new().analyse("p\n", dialect);
            assert!(
                items("p\n", 1, &consumer, &index, "p").is_empty(),
                "{dialect}"
            );
        }
        let consumer = Analyser::new().analyse("p\n", "tcl8.6");
        let suggestions = items("p\n", 1, &consumer, &index, "p");
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].label, "public");
        assert!(
            suggestions[0]
                .detail
                .as_ref()
                .unwrap()
                .contains("Source declaration")
        );
    }
    // Implementation contract: naming.core.original-workspace-procedure-completion
    // docs/design/analysis/name-resolution-proofs/original-workspace-procedure-completion.md
    #[test]
    fn original_workspace_hosted_completion_preserves_its_own_source_policy_and_unknown_inputs() {
        // Implementation contract: naming.core.original-workspace-procedure-completion
        // docs/design/analysis/name-resolution-proofs/original-workspace-procedure-completion.md
        let source = "p\n";
        let mut consumer = Analyser::new().analyse(source, "f5-irules");
        let mut producer = Analyser::new().analyse(
            r"proc public {value} {return $value}; proc p\uD800 {} {}",
            "f5-irules",
        );
        assert_eq!(producer.original_vendor_procedure_declarations().count(), 2);
        assert!(
            producer
                .original_vendor_procedure_declarations()
                .any(|row| row.name_input().literal_units(row.purpose()).is_none())
        );
        consumer.all_procs.clear();
        producer.all_procs.clear();
        producer.global_scope.procs.clear();
        let index = WorkspaceIndex::from_documents([("file:///producer.tcl", &producer)]);
        let suggestions = items(source, 1, &consumer, &index, "p");
        assert_eq!(suggestions.len(), 1, "{suggestions:?}");
        assert_eq!(suggestions[0].insert_text, "public");
        assert!(
            suggestions[0]
                .detail
                .as_ref()
                .unwrap()
                .contains("Hosted source declaration")
        );
        let foreign = Analyser::new().analyse(source, "f5-iapps");
        assert!(items(source, 1, &foreign, &index, "p").is_empty());
        assert!(items("# displaced\np\n", 1, &consumer, &index, "p").is_empty());
    }
}
