// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Property option candidates from original declarations and their selected
//! accessor protocol. This does not establish property installation or lookup.

use super::{CompletionEdit, CompletionItem, CompletionKind};
use tcl_compiler::analyser::types::MemberSide;
use tcl_compiler::analyser::{AnalysisResult, ClassDef};
use tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata;
use tcl_compiler::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameKey};
use tcl_core_types::NameBytes;
use tcl_lexer::{SourceImage, Span};
use tcl_registry::commands::tcl::TclOoPropertyKind;
use tcl_registry::definer::SourcePropertyCompletionRole;

pub(super) fn items(
    source: &str,
    cursor: u32,
    line: u32,
    analysis: &AnalysisResult,
    partial: &str,
) -> Option<Vec<CompletionItem>> {
    let config = analysis.body_lexer_config?;
    let image = SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, config) {
        return None;
    }
    let receiver = crate::original_oo::instance_completion_source(analysis, source, cursor)?;
    let root = receiver.class;
    let selector_word = receiver.argument_word(0)?;
    if receiver.is_written_argument(0) && selector_word.word_span().end() >= cursor {
        return None;
    }
    let selector = receiver.argument_input(0)?;
    let arguments = receiver.argument_count()?.checked_sub(1)?;
    let current = (1..=arguments).find_map(|ordinal| {
        let word = receiver.argument_word(ordinal)?;
        (receiver.is_written_argument(ordinal)
            && word.word_span().start() <= cursor
            && cursor <= word.word_span().end())
        .then_some((ordinal - 1, word))
    });
    let (ordinal, replacement) = if let Some((ordinal, word)) = current {
        let key = SignatureSourceNameKey::from_original_native_word(
            word,
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
            selector.policy(),
        )?;
        (ordinal, Some(SignatureSourceNameInput::OriginalWord(key)))
    } else {
        let end = receiver.original_words().last()?.word_span().end();
        if !crate::original_oo::instance_completion::completion_gap(source, end, cursor, config) {
            return None;
        }
        (arguments, None)
    };
    candidate_items(
        source,
        analysis,
        root,
        &selector,
        (cursor, line),
        (ordinal, arguments),
        (replacement.as_ref(), partial),
    )
}

fn candidate_items(
    source: &str,
    analysis: &AnalysisResult,
    root: &SourceDeclarationMetadata<ClassDef>,
    selector: &SignatureSourceNameInput,
    (cursor, line): (u32, u32),
    (ordinal, arguments): (usize, usize),
    (replacement, partial): (Option<&SignatureSourceNameInput>, &str),
) -> Option<Vec<CompletionItem>> {
    let config = analysis.body_lexer_config?;
    let image = SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, config) {
        return Some(Vec::new());
    }
    let providers = tcl_compiler::analyser::class_hierarchy::original_metadata::original_instance_metadata_order(analysis, root)?;
    let mut seen_names = Vec::new();
    let mut seen_options = Vec::new();
    let mut items = Vec::new();
    let mut recognised = false;
    for provider in providers {
        let properties = provider
            .metadata()
            .original_properties
            .properties(MemberSide::Instance)?;
        for property in properties {
            let input = property.original_name_input();
            let name = (NameBytes::from(input.bytes()), input.policy());
            if seen_names.contains(&name) {
                continue;
            }
            seen_names.push(name);
            let Some(role) = property.accessor_completion_role(selector, ordinal, arguments) else {
                continue;
            };
            recognised = true;
            let allowed = match role {
                SourcePropertyCompletionRole::Readable => matches!(
                    property.kind(),
                    TclOoPropertyKind::Readable | TclOoPropertyKind::ReadWrite
                ),
                SourcePropertyCompletionRole::Writable => matches!(
                    property.kind(),
                    TclOoPropertyKind::Writable | TclOoPropertyKind::ReadWrite
                ),
                SourcePropertyCompletionRole::ReadOrWrite => true,
            };
            if !allowed {
                continue;
            }
            if !property.matches_original_declaration_source(source, analysis, provider) {
                continue;
            }
            let option = property.option_name()?;
            let bytes = option.selected();
            let identity = (NameBytes::from(bytes), input.policy());
            if seen_options.contains(&identity) {
                continue;
            }
            let spelling = tcl_syntax::backslash::native_literal_source_word(
                bytes,
                image.channel(),
                config,
                input.policy().string_protocol(),
            )?;
            let label = std::str::from_utf8(bytes).map_or_else(|_| spelling.clone(), str::to_owned);
            if !label.starts_with(partial) && !spelling.starts_with(partial) {
                continue;
            }
            let (span, text) = if let Some(replacement) = replacement {
                let edits = crate::original_name_edit::original_name_input_edits(
                    &image,
                    &[(replacement.clone(), NameBytes::from(bytes))],
                )?;
                let [edit] = edits.as_slice() else {
                    return Some(Vec::new());
                };
                (edit.span(), edit.text().to_owned())
            } else {
                (Span::empty(cursor), spelling.clone())
            };
            let index = tcl_lexer::LineIndex::new(source);
            let begin = index.position_at_utf16(span.start(), source);
            let end = index.position_at_utf16(span.end(), source);
            if begin.line != line || end.line != line {
                continue;
            }
            seen_options.push(identity);
            items.push(CompletionItem {
                label,
                insert_text: spelling,
                kind: CompletionKind::EnumValue,
                detail: Some("property option — source declaration candidate".to_owned()),
                text_edit: Some(CompletionEdit {
                    start_char: begin.character.get(),
                    end_char: end.character.get(),
                    new_text: text,
                }),
                ..CompletionItem::default()
            });
        }
    }
    items.sort_by(|left, right| left.label.cmp(&right.label));
    recognised.then_some(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_property_options_keep_opaque_names_and_selected_read_write_roles() {
        let source = r"oo::configurable create C {property p\uD800 -kind readable p\uD801 -kind writable shared}";
        let mut analysis = Analyser::new().analyse(source, "tcl9.0").clone();
        analysis.all_classes.clear();
        let root = analysis.original_class_declarations().next().unwrap();
        // naming.tcloo.original-property-accessor-source-advice
        // docs/design/analysis/name-resolution-proofs/tcloo-original-property-accessor-source-advice.md
        let properties = root
            .metadata()
            .original_properties
            .properties(MemberSide::Instance)
            .unwrap();
        assert_eq!(properties.len(), 3);
        assert!(
            properties
                .iter()
                .all(|property| property
                    .matches_original_declaration_source(source, &analysis, root))
        );
        assert!(
            properties
                .iter()
                .all(|property| !property.matches_original_declaration_source(
                    &format!("#{source}"),
                    &analysis,
                    root
                ))
        );
        let foreign = Analyser::new().analyse(source, "tcl9.0");
        let foreign_root = foreign.original_class_declarations().next().unwrap();
        assert!(
            properties
                .iter()
                .all(|property| !property.matches_original_declaration_source(
                    source,
                    &foreign,
                    foreign_root
                ))
        );
        let policy = root.name_input().policy();
        let selector_image = SourceImage::document("configure");
        let config = analysis.body_lexer_config.unwrap();
        let plan =
            tcl_lexer::native_script_words_in(selector_image, Span::new(0, 9), config).unwrap();
        let selector = SignatureSourceNameInput::OriginalWord(
            SignatureSourceNameKey::from_original_native_word(
                &plan.commands[0].words[0],
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
                policy,
            )
            .unwrap(),
        );
        let read = candidate_items(
            source,
            &analysis,
            root,
            &selector,
            (0, 0),
            (0, 1),
            (None, ""),
        )
        .unwrap();
        let write = candidate_items(
            source,
            &analysis,
            root,
            &selector,
            (0, 0),
            (2, 2),
            (None, ""),
        )
        .unwrap();
        assert_eq!(read.len(), 2, "{read:?}");
        assert_eq!(write.len(), 2, "{write:?}");
        let option_bytes = |item: &CompletionItem| {
            let image = SourceImage::document(&item.insert_text);
            let plan = tcl_lexer::native_script_words_in(
                image.clone(),
                Span::new(0, u32::try_from(image.len()).unwrap()),
                config,
            )
            .unwrap();
            SignatureSourceNameKey::from_original_native_word(
                &plan.commands[0].words[0],
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
                policy,
            )
            .unwrap()
            .bytes()
            .to_vec()
        };
        let read_bytes: Vec<_> = read.iter().map(option_bytes).collect();
        let write_bytes: Vec<_> = write.iter().map(option_bytes).collect();
        assert!(read_bytes.contains(&b"-p\xed\xa0\x80".to_vec()));
        assert!(!read_bytes.contains(&b"-p\xed\xa0\x81".to_vec()));
        assert!(write_bytes.contains(&b"-p\xed\xa0\x81".to_vec()));
        assert!(!write_bytes.contains(&b"-p\xed\xa0\x80".to_vec()));
        assert!(
            candidate_items(
                source,
                &analysis,
                root,
                &selector,
                (0, 0),
                (1, 2),
                (None, "")
            )
            .is_none()
        );
        assert!(
            candidate_items(
                &format!("#{source}"),
                &analysis,
                root,
                &selector,
                (0, 0),
                (0, 1),
                (None, "")
            )
            .unwrap()
            .is_empty()
        );
        for item in read.iter().chain(&write) {
            let rendered = SourceImage::document(&item.insert_text);
            let plan = tcl_lexer::native_script_words_in(
                rendered.clone(),
                Span::new(0, u32::try_from(rendered.len()).unwrap()),
                config,
            )
            .unwrap();
            let key = SignatureSourceNameKey::from_original_native_word(
                &plan.commands[0].words[0],
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
                policy,
            )
            .unwrap();
            assert!(
                [b"-p\xed\xa0\x80".as_slice(), b"-p\xed\xa0\x81", b"-shared"]
                    .contains(&key.bytes()),
                "{:?}",
                key.bytes()
            );
        }
    }

    #[test]
    fn original_property_options_require_receiver_selector_currency_and_do_not_retag_values() {
        // naming.tcloo.original-property-accessor-source-advice
        // docs/design/analysis/name-resolution-proofs/tcloo-original-property-accessor-source-advice.md
        let source = "oo::configurable create C {property readable -kind readable writable -kind writable}\nset object [C new]\n$object configure \n";
        let analysis = Analyser::new().analyse(source, "tcl9.0").clone();
        let cursor = u32::try_from(source.len() - 1).unwrap();
        let index = tcl_lexer::LineIndex::new(source);
        let line = index.position_at(cursor).line;
        assert!(
            crate::original_oo::instance_completion_source(&analysis, source, cursor).is_some(),
            "the genuine receiver source must be selected independently of property candidates"
        );
        let completed = items(source, cursor, line, &analysis, "").unwrap();
        assert_eq!(completed.len(), 2, "{completed:?}");
        assert!(items(&format!("#{source}"), cursor, line, &analysis, "").is_none());
        let no_receiver = "oo::configurable create C {property readable}\nunknown configure \n";
        let analysis = Analyser::new().analyse(no_receiver, "tcl9.0").clone();
        let cursor = u32::try_from(no_receiver.len() - 1).unwrap();
        assert!(items(no_receiver, cursor, 1, &analysis, "").is_none());
        let old = Analyser::new().analyse(source, "tcl8.6").clone();
        assert!(
            items(
                source,
                u32::try_from(source.len() - 1).unwrap(),
                line,
                &old,
                ""
            )
            .is_none()
        );
    }
}

#[cfg(test)]
mod source_instance_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_property_options_share_symbolic_and_captured_receiver_source_argv() {
        // naming.tcloo.original-property-accessor-source-advice
        // docs/design/analysis/name-resolution-proofs/tcloo-original-property-accessor-source-advice.md
        // naming.core.original-source-instance-completion
        // docs/design/analysis/name-resolution-proofs/core-original-source-instance-completion.md
        for source in [
            "oo::configurable create C {property readable -kind readable writable -kind writable}; set o [C new]; $o configure ",
            "oo::configurable create C {property readable -kind readable writable -kind writable}; C create object; interp alias {} read {} object configure; read ",
        ] {
            let mut analysis = Analyser::new().analyse(source, "tcl9.0");
            analysis.all_classes.clear();
            analysis.global_scope.classes.clear();
            analysis.instance_classes.clear();
            analysis.created_instance_commands.clear();
            let cursor = u32::try_from(source.len()).unwrap();
            let options = items(source, cursor, 0, &analysis, "").unwrap();
            assert_eq!(
                options
                    .iter()
                    .map(|item| item.label.as_str())
                    .collect::<Vec<_>>(),
                ["-readable", "-writable"]
            );
            assert!(options.iter().all(|item| {
                item.text_edit
                    .as_ref()
                    .is_some_and(|edit| edit.start_char == cursor && edit.end_char == cursor)
            }));
            assert!(
                items(source, cursor, 0, &analysis, "ordinary-value")
                    .unwrap()
                    .is_empty()
            );
        }
    }
}
