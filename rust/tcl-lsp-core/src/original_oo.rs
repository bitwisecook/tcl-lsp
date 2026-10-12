// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original OO declaration assistance, separate from temporal dispatch.

use std::ops::ControlFlow;
use tcl_compiler::analyser::types::{MemberSide, OriginalSourceMethodMetadata};
use tcl_compiler::analyser::{AnalysisResult, ClassDef};
use tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata;
use tcl_compiler::signature_scan::types::SignatureCommandInvocation;
use tcl_lexer::{SourceImage, Span};

pub(crate) fn class_for_invocation<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    invocation: &SignatureCommandInvocation,
) -> ControlFlow<Option<&'a SourceDeclarationMetadata<ClassDef>>> {
    let Some(input) = invocation.original_name_input.as_ref() else {
        return ControlFlow::Continue(());
    };
    let Some(config) = analysis.body_lexer_config else {
        return ControlFlow::Break(None);
    };
    if !analysis.matches_original_source_image(&SourceImage::document(source), config) {
        return ControlFlow::Break(None);
    }
    let Some(lookup) = invocation
        .original_lookup
        .as_ref()
        .filter(|lookup| lookup.name_input() == input)
    else {
        return ControlFlow::Break(None);
    };
    if let Some(reference) = &invocation.resolved_command_reference {
        let Some(definition) = reference.definition() else {
            return ControlFlow::Break(None);
        };
        if definition.kind() != tcl_compiler::command_binding::SourceCommandDefinitionKind::Class {
            return ControlFlow::Break(None);
        }
        if !matches!(definition.allocation().site.source.kind(),
            tcl_compiler::command_binding::SourceOriginKind::Authored(bytes) if bytes.as_ref() == source.as_bytes())
        {
            return ControlFlow::Break(None);
        }
        let mut records = analysis
            .original_class_declarations()
            .filter(|record| record.declaration_site() == &definition.allocation().site);
        let first = records.next();
        return ControlFlow::Break(records.next().is_none().then_some(first).flatten());
    }
    // Call-site source applicability is independent of the final inventory.
    // A later move cannot change an earlier call, and a missing receipt cannot
    // recover a known terminal class through its canonical declaration label.
    let selected = tcl_compiler::registry_invocation::source_structure::source_constructor_call_at(
        source,
        analysis,
        lookup.site().offset,
    )
    .and_then(|call| call.class_declaration().source_class(analysis));
    ControlFlow::Break(selected)
}

pub(crate) fn class_at_cursor<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    cursor: u32,
) -> ControlFlow<Option<&'a SourceDeclarationMetadata<ClassDef>>> {
    let Some(config) = analysis.body_lexer_config else {
        return ControlFlow::Continue(());
    };
    if !analysis.matches_original_source_image(&SourceImage::document(source), config) {
        return ControlFlow::Break(None);
    }
    let mut declarations = analysis.original_class_declarations().filter(|record| {
        record.name_input().source_image() == &SourceImage::document(source)
            && record.name_input().span().start() <= cursor
            && cursor < record.name_input().span().end()
    });
    if let Some(first) = declarations.next() {
        return ControlFlow::Break(declarations.next().is_none().then_some(first));
    }
    // Relation operands retain their own original caller coordinates. A
    // nearby class name or a reporting superclass list cannot supply them.
    let image = SourceImage::document(source);
    let mut relations = analysis
        .original_class_declarations()
        .flat_map(|class| {
            class
                .metadata()
                .original_relations
                .effects()
                .flat_map(|effect| effect.values().unwrap_or_default().iter())
        })
        .filter(|relation| {
            relation
                .name_input()
                .original_word_key()
                .is_some_and(|key| {
                    key.source_image() == &image
                        && key.lexer_config() == config
                        && key.span().start() <= cursor
                        && cursor < key.span().end()
                })
        });
    if let Some(relation) = relations.next() {
        if relations.next().is_some() {
            return ControlFlow::Break(None);
        }
        let selected = relation.lookup().first_matching_publications(
            analysis
                .original_class_declarations()
                .map(|class| (class.name(), Some(class)))
                .chain(
                    analysis
                        .original_procedure_declarations()
                        .map(|proc| (proc.name(), None)),
                ),
        );
        return ControlFlow::Break(match selected.as_slice() {
            [Some(class)] => Some(*class),
            _ => None,
        });
    }
    analysis
        .command_invocations
        .iter()
        .find(|invocation| invocation.range.start() <= cursor && cursor < invocation.range.end())
        .map_or(ControlFlow::Continue(()), |invocation| {
            class_for_invocation(analysis, source, invocation)
        })
}

/// An original class-command completion position. The retained invocation owns
/// the head; parsing only identifies its first argument geometry. Declaration
/// matching supplies candidates, never an installed class or future dispatch.
pub(crate) fn class_completion_context<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    cursor: u32,
) -> ControlFlow<Option<&'a SourceDeclarationMetadata<ClassDef>>> {
    let Some(config) = analysis.body_lexer_config else {
        return ControlFlow::Continue(());
    };
    if !analysis.matches_original_source_image(&SourceImage::document(source), config)
        && analysis
            .command_invocations
            .iter()
            .any(|invocation| invocation.original_name_input.is_some())
    {
        return ControlFlow::Break(None);
    }
    let invocation = analysis
        .command_invocations
        .iter()
        .filter(|invocation| invocation.range.start() < cursor)
        .max_by_key(|invocation| invocation.range.start());
    let Some(invocation) = invocation else {
        return ControlFlow::Continue(());
    };
    let Some(input) = invocation.original_name_input.as_ref() else {
        return ControlFlow::Continue(());
    };
    let span = input
        .original_word_key()
        .map_or(invocation.range, |key| key.original_word().word_span());
    if span.end() > cursor {
        return ControlFlow::Continue(());
    }
    let image = SourceImage::document(source);
    let Some(end) = u32::try_from(source.len()).ok() else {
        return ControlFlow::Break(None);
    };
    let Ok(plan) = tcl_lexer::native_script_words_in(image, Span::new(span.start(), end), config)
    else {
        return ControlFlow::Continue(());
    };
    let Some(command) = plan.commands.first() else {
        return ControlFlow::Continue(());
    };
    if command.words.first().is_none_or(|word| {
        word.word_span().start() != span.start()
            || input
                .original_word_key()
                .is_some_and(|key| word.word_span() != key.original_word().word_span())
    }) {
        return ControlFlow::Continue(());
    }
    if let Some(selector) = command
        .words
        .get(1)
        .filter(|word| word.word_span().start() <= cursor)
    {
        if selector.word_span().end() < cursor {
            return ControlFlow::Continue(());
        }
    } else {
        let gap = source
            .get(span.end() as usize..cursor as usize)
            .unwrap_or("");
        if gap
            .bytes()
            .any(|byte| !matches!(byte, b' ' | b'\t' | b'\r'))
        {
            return ControlFlow::Continue(());
        }
    }
    if let Some(call) =
        tcl_compiler::registry_invocation::source_structure::source_constructor_call_at(
            source,
            analysis,
            invocation.range.start(),
        )
    {
        // Captured selectors already occupy the effective method position.
        // Written source geometry alone cannot offer another selector slot.
        if call.argument_word(0).is_some() && call.written_argument(0) != Some(0) {
            return ControlFlow::Break(None);
        }
    }
    class_for_invocation(analysis, source, invocation)
}

pub(crate) fn method_selector_replacement(
    analysis: &AnalysisResult,
    source: &str,
    cursor: u32,
    wanted: &[u8],
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> Option<(Span, String)> {
    let config = analysis.body_lexer_config?;
    let image = SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, config) {
        return None;
    }
    let invocation = analysis
        .command_invocations
        .iter()
        .filter(|invocation| invocation.range.start() < cursor)
        .max_by_key(|invocation| invocation.range.start())?;
    let head = invocation.original_name_input.as_ref()?;
    if head.policy() != policy {
        return None;
    }
    let start = head
        .original_word_key()
        .map_or(invocation.range.start(), |key| {
            key.original_word().word_span().start()
        });
    let plan = tcl_lexer::native_script_words_in(
        image.clone(),
        Span::new(start, u32::try_from(source.len()).ok()?),
        config,
    )
    .ok()?;
    let command = plan.commands.first()?;
    if let Some(word) = command
        .words
        .get(1)
        .filter(|word| word.word_span().start() <= cursor && cursor <= word.word_span().end())
    {
        let key =
            tcl_compiler::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                word,
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
                policy,
            )?;
        let input =
            tcl_compiler::signature_scan::scope::SignatureSourceNameInput::OriginalWord(key);
        let edits = crate::original_name_edit::original_name_input_edits(
            &image,
            &[(input, tcl_core_types::NameBytes::from(wanted))],
        )?;
        let [edit] = edits.as_slice() else {
            return None;
        };
        return Some((edit.span(), edit.text().to_owned()));
    }
    let gap = source.get(command.words.first()?.word_span().end() as usize..cursor as usize)?;
    if !gap.bytes().all(|byte| matches!(byte, b' ' | b'\t' | b'\r')) {
        return None;
    }
    Some((
        Span::empty(cursor),
        tcl_syntax::backslash::native_literal_source_word(
            wanted,
            image.channel(),
            config,
            policy.string_protocol(),
        )?,
    ))
}

pub(crate) mod instance_completion;
pub(crate) use instance_completion::{
    instance_completion_source, instance_method_completion_record,
};

/// Current own-table route, with canonical source metadata retained separately.
pub(crate) struct OwnObjectMethodCandidate<'a> {
    pub(crate) input: tcl_compiler::signature_scan::scope::SignatureSourceNameInput,
    pub(crate) metadata: &'a OriginalSourceMethodMetadata,
    pub(crate) exported: bool,
}

/// Original own-object candidates at the exact positioned head. A missing
/// own-table receipt does not prove absence; a failed canonical join cannot
/// borrow a class table or reporting object ledger.
pub(crate) fn own_object_completion_methods<'a>(
    analysis: &'a AnalysisResult,
    source: &str,
    cursor: u32,
) -> ControlFlow<Option<Vec<OwnObjectMethodCandidate<'a>>>> {
    let Some(config) = analysis.body_lexer_config else {
        return ControlFlow::Continue(());
    };
    if !analysis.matches_original_source_image(&SourceImage::document(source), config) {
        return ControlFlow::Break(None);
    }
    let Some(invocation) = analysis
        .command_invocations
        .iter()
        .filter(|invocation| invocation.range.start() < cursor)
        .max_by_key(|invocation| invocation.range.start())
    else {
        return ControlFlow::Continue(());
    };
    let Some(input) = invocation.original_name_input.as_ref() else {
        return ControlFlow::Continue(());
    };
    let Some(lookup) = invocation
        .original_lookup
        .as_ref()
        .filter(|lookup| lookup.name_input() == input)
    else {
        return ControlFlow::Break(None);
    };
    let Some(realm) = analysis.retained_command_realm() else {
        return ControlFlow::Break(None);
    };
    let binding = realm.invocation_at_source(&invocation.name, lookup.site().offset);
    if binding.original_recorded_head_name_input().as_ref() != Some(input) {
        return ControlFlow::Break(None);
    }
    let Some((receiver, entries)) = binding.own_object_method_entries_at_dispatch() else {
        return ControlFlow::Continue(());
    };
    let mut candidates = Vec::new();
    for entry in entries.values() {
        let Some(metadata) = crate::receiver_identity::original_own_method_metadata(
            analysis,
            source,
            receiver.allocation(),
            entry,
        ) else {
            return ControlFlow::Break(None);
        };
        candidates.push(OwnObjectMethodCandidate {
            input: entry.original_name_input().clone(),
            metadata,
            exported: entry.is_exported(),
        });
    }
    ControlFlow::Break(Some(candidates))
}

/// Source method names, kept distinct even when their display text collides.
pub(crate) fn method_word(method: &OriginalSourceMethodMetadata) -> Option<String> {
    let input = method.original_name_input();
    let declaration = method.declaration().original_word();
    if !method
        .name_purpose()
        .admits(input.policy().recipe(), input.bytes())
    {
        return None;
    }
    tcl_syntax::backslash::native_literal_source_word(
        input.bytes(),
        declaration.image().channel(),
        declaration.config(),
        input.policy().string_protocol(),
    )
}

pub(crate) fn method_label(method: &OriginalSourceMethodMetadata) -> Option<String> {
    Some(tcl_syntax::native_string::resident_name_label(
        method.original_name_input().bytes(),
    ))
}

pub(crate) fn own_method_labels(class: &ClassDef, side: MemberSide) -> Option<Vec<String>> {
    let mut labels = class
        .original_members
        .methods(side)?
        .iter()
        .filter_map(method_label)
        .collect::<Vec<_>>();
    labels.sort();
    Some(labels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_class_head_selection_keeps_earlier_call_horizons_and_known_terminals() {
        // naming.source.original-class-publications
        // docs/design/analysis/name-resolution-proofs/source-original-class-publications.md
        let source = "oo::class create C {}; C; rename C M; C; M; interp alias {} A {} M method; A; rename M {}; M; A";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(analysis.original_completed_command_world().is_none());
        analysis.all_classes.clear();
        let mut selected = Vec::new();
        for invocation in &analysis.command_invocations {
            let Some(input) = invocation.original_name_input.as_ref() else {
                continue;
            };
            if !matches!(input.bytes(), b"C" | b"M" | b"A") {
                continue;
            }
            let receipt =
                tcl_compiler::registry_invocation::source_structure::source_constructor_call_at(
                    source,
                    &analysis,
                    invocation.range.start(),
                );
            let found = class_for_invocation(&analysis, source, invocation);
            assert_eq!(
                matches!(found, ControlFlow::Break(Some(_))),
                receipt.is_some()
            );
            if let ControlFlow::Break(Some(class)) = found {
                assert_eq!(class.name_input().bytes(), b"C");
            }
            selected.push(receipt.is_some());
        }
        assert_eq!(selected, [true, false, true, true, false, false]);
    }

    #[test]
    fn original_class_completion_declines_a_captured_selector_position() {
        // naming.source.original-class-publications
        // docs/design/analysis/name-resolution-proofs/source-original-class-publications.md
        for (source, expected) in [
            ("oo::class create C {}; interp alias {} A {} C; A ", true),
            (
                "oo::class create C {}; interp alias {} A {} C method; A ",
                false,
            ),
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.rfind("A ").unwrap()).unwrap();
            let call =
                tcl_compiler::registry_invocation::source_structure::source_constructor_call_at(
                    source, &analysis, offset,
                )
                .unwrap();
            assert_eq!(call.argument_word(0).is_none(), expected);
            assert_eq!(
                matches!(
                    class_completion_context(
                        &analysis,
                        source,
                        u32::try_from(source.len()).unwrap()
                    ),
                    ControlFlow::Break(Some(_)),
                ),
                expected
            );
        }
    }

    #[test]
    fn original_class_head_selection_keeps_opaque_candidates_and_source_currency() {
        let source = "oo::class create C\\uD800 {}\noo::class create C\\uD801 {}\nC\\uD800\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6").clone();
        analysis.all_classes.clear();
        analysis.superseded_classes.clear();
        let invocation = analysis
            .command_invocations
            .iter()
            .find(|invocation| {
                invocation
                    .original_name_input
                    .as_ref()
                    .is_some_and(|input| input.bytes() == b"C\xed\xa0\x80")
            })
            .unwrap();
        let ControlFlow::Break(Some(record)) = class_for_invocation(&analysis, source, invocation)
        else {
            panic!("exact class declaration candidate");
        };
        assert_eq!(record.name_input().bytes(), b"C\xed\xa0\x80");
        assert!(matches!(
            class_for_invocation(&analysis, &format!("#{source}"), invocation),
            ControlFlow::Break(None)
        ));
    }

    #[test]
    // Implementation contract: naming.core.readonly-member-source-candidates
    // docs/design/analysis/name-resolution-proofs/readonly-member-source-candidates.md
    fn original_readonly_loop_method_candidates_keep_values_separate_from_worker_geometry() {
        let source =
            "oo::class create C {foreach item {m\\uD800 m\\uD801} {method $item {} {return body}}}";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let record = analysis.original_class_declarations().next().unwrap();
        let methods = record
            .metadata()
            .original_members
            .methods(MemberSide::Instance)
            .unwrap();
        assert_eq!(methods.len(), 2);
        let mut bytes = Vec::new();
        for method in &methods {
            assert!(method.declaration().static_occurrence().is_none());
            assert!(
                method
                    .declaration()
                    .original_name_input()
                    .original_word_key()
                    .is_none()
            );
            assert_eq!(
                method.declaration().original_word().image(),
                &SourceImage::document(source)
            );
            let word = method_word(method).unwrap();
            let config = method.declaration().original_word().config();
            let plan = tcl_lexer::native_script_words_in(
                SourceImage::document(&word),
                Span::new(0, u32::try_from(word.len()).unwrap()),
                config,
            )
            .unwrap();
            let key = tcl_compiler::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                &plan.commands[0].words[0], tcl_syntax::word_rules::WordValueRules::from_config(&config),
                method.original_name_input().policy()).unwrap();
            assert_eq!(key.bytes(), method.original_name_input().bytes());
            bytes.push(key.bytes().to_vec());
        }
        bytes.sort();
        assert_eq!(
            bytes,
            [b"m\xed\xa0\x80".to_vec(), b"m\xed\xa0\x81".to_vec()]
        );
    }

    #[test]
    fn original_own_method_labels_render_counted_names_after_ui_tables_are_cleared() {
        let source = "oo::class create C {method m\\uD800 {} {}; method m\\uD801 {} {}}";
        let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
        let mut class = analysis
            .original_class_declarations()
            .next()
            .unwrap()
            .metadata()
            .clone();
        class.methods.clear();
        class.class_methods.clear();
        let labels = own_method_labels(&class, MemberSide::Instance).unwrap();
        assert_eq!(labels.len(), 2);
        assert_ne!(labels[0], labels[1]);
        let methods = class
            .original_members
            .methods(MemberSide::Instance)
            .unwrap();
        for method in methods {
            let word = method_word(&method).unwrap();
            let key = method
                .declaration()
                .static_occurrence()
                .unwrap()
                .name_input();
            let plan = tcl_lexer::native_script_words_in(
                SourceImage::document(&word),
                Span::new(0, u32::try_from(word.len()).unwrap()),
                key.lexer_config(),
            )
            .unwrap();
            let emitted = tcl_compiler::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                &plan.commands[0].words[0], key.word_value_rules(), key.policy()).unwrap();
            assert_eq!(emitted.bytes(), method.original_name_input().bytes());
        }
    }
}
