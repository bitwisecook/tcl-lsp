// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original alias-name templates, independently of executed variable links.

use super::AnalysisResult;
use crate::command_binding::{ExecutedScriptMapping, SourceOriginalVariableFrame};
use crate::signature_scan::scope::SignatureSourceNameInput;
use crate::signature_scan::variable_symbol::{
    OriginalVariableAliasAdvice, OriginalVariableAliasReceipt, OriginalVariableAliasTemplate,
    OriginalVariableSymbolReceiver, original_variable_alias_local_name,
};
use std::collections::{HashMap, HashSet};
use tcl_lexer::{LexerConfig, SourceImage, Span};

struct SourceAliasAnchor {
    span: Span,
    input: SignatureSourceNameInput,
    receiver: OriginalVariableSymbolReceiver,
}

impl AnalysisResult {
    /// Conditional alias-name relationships in this complete current source.
    /// Every anchor retains its real lexical declaration frame and original
    /// name producer. Unknown target indices, entered links, successful stores,
    /// mutation continuity and rename coverage remain independent obligations.
    #[must_use]
    pub fn original_variable_alias_templates_in_source(
        &self,
        image: &SourceImage,
        config: LexerConfig,
    ) -> Option<Vec<OriginalVariableAliasTemplate>> {
        // Implementation contract: naming.variable.original-alias-source-template
        // docs/design/analysis/name-resolution-proofs/original-alias-source-template.md
        self.matches_original_source_image(image, config)
            .then_some(())?;
        let realm = self.retained_command_realm()?;
        let bindings = realm.source_bindings_ref();
        let input = self.resolved_input.as_ref()?;
        let source_declarations = if input.has_logical_source_name_context() {
            self.original_variable_alias_source_operands
                .iter()
                .filter_map(|source| source.logical_declarations(input, realm))
                .flatten()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let source_templates = source_declarations
            .iter()
            .filter_map(super::super::registry_invocation::declaration_assistance::OriginalLogicalAliasDeclaration::template_source)
            .filter(|source| source.matches_source(image, input))
            .collect::<Vec<_>>();
        let mut anchors = self.original_alias_source_anchors();
        anchors.extend(source_templates.iter().map(|source| {
            let declaration = OriginalVariableAliasAdvice::from_receipt(source.relation());
            SourceAliasAnchor {
                span: declaration.span(),
                input: declaration.original_name_input().clone(),
                receiver: OriginalVariableSymbolReceiver::Operand(
                    tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined,
                ),
            }
        }));
        let mut direct = HashMap::<SourceOriginalVariableFrame, Option<HashSet<u32>>>::new();
        let mut templates = Vec::new();
        for anchor in anchors {
            let logical_scope = if input.has_logical_source_name_context() {
                let Some(scope) = realm.original_logical_variable_scope_at(
                    input,
                    image,
                    anchor.span,
                    &anchor.input,
                ) else {
                    continue;
                };
                Some(scope)
            } else {
                None
            };
            let frame = if let Some(scope) = &logical_scope {
                scope.alias_frame()
            } else {
                bindings.original_variable_frame_at_span(anchor.span, config)
            };
            let Some(frame) = frame else {
                continue;
            };
            let mut candidates = self
                .original_variable_alias_receipts
                .iter()
                .chain(source_templates.iter().map(super::super::registry_invocation::declaration_assistance::OriginalLogicalAliasTemplateSource::relation))
                .filter(|alias| {
                    alias.frame() == &frame
                        && alias.invocation_offset() <= anchor.span.start()
                        && if let Some(scope) = &logical_scope {
                            scope.matches_alias_template(
                                anchor.span,
                                &anchor.input,
                                anchor.receiver,
                                alias,
                            )
                        } else {
                            bindings.original_variable_alias_template_matches_at_span(
                                anchor.span,
                                &anchor.input,
                                anchor.receiver,
                                alias,
                                config,
                            )
                        }
                })
                .collect::<Vec<_>>();
            candidates.sort_by_key(|alias| alias.invocation_offset());
            let Some(alias) = candidates.pop() else {
                continue;
            };
            if candidates.iter().any(|other| {
                other.invocation_offset() == alias.invocation_offset() && *other != alias
            }) || self.original_alias_source_has_later_blocker(image, config, alias, anchor.span)
                || (logical_scope.is_some()
                    && self
                        .original_variable_alias_source_operands
                        .iter()
                        .any(|source| {
                            source.blocks_logical_template(input, realm, alias, anchor.span)
                        }))
            {
                continue;
            }
            let declaration = OriginalVariableAliasAdvice::from_receipt(alias);
            // An immediate top-level declaration can introduce a later source
            // template. A nested conditional alias requires its separate path
            // owner; its declaration itself still names the written relation.
            if anchor.span != declaration.span()
                && !direct
                    .entry(frame.clone())
                    .or_insert_with(|| original_frame_command_sites(&frame))
                    .as_ref()
                    .is_some_and(|sites| sites.contains(&alias.invocation_offset()))
            {
                continue;
            }
            let mut sources = source_templates
                .iter()
                .filter(|source| source.relation() == alias);
            let template = if let Some(source) = sources.next() {
                if sources.any(|other| other != source) {
                    continue;
                }
                OriginalVariableAliasTemplate::from_logical_source(
                    anchor.span,
                    anchor.input,
                    source.clone(),
                )
            } else {
                OriginalVariableAliasTemplate::new(anchor.span, anchor.input, alias.clone())
            };
            if !templates.contains(&template) {
                templates.push(template);
            }
        }
        templates.sort_by_key(|template| (template.span().start(), template.span().end()));
        Some(templates)
    }

    fn original_alias_source_anchors(&self) -> Vec<SourceAliasAnchor> {
        let mut anchors = self
            .original_variable_symbols
            .iter()
            .map(|occurrence| SourceAliasAnchor {
                span: occurrence.span(),
                input: occurrence.original_name_input().clone(),
                receiver: occurrence.receiver(),
            })
            .collect::<Vec<_>>();
        anchors.extend(self.original_variable_write_advice.iter().map(|advice| {
            SourceAliasAnchor {
                span: advice.span(),
                input: advice.original_name_input().clone(),
                receiver: OriginalVariableSymbolReceiver::Operand(advice.receiver_form()),
            }
        }));
        anchors.extend(
            self.original_variable_roots
                .iter()
                .map(|root| SourceAliasAnchor {
                    span: root.source_span(),
                    input: SignatureSourceNameInput::OriginalVariableRoot(root.clone()),
                    receiver: OriginalVariableSymbolReceiver::LexicalRoot,
                }),
        );
        anchors.extend(self.original_variable_alias_receipts.iter().map(|alias| {
            let declaration = OriginalVariableAliasAdvice::from_receipt(alias);
            SourceAliasAnchor {
                span: declaration.span(),
                input: declaration.original_name_input().clone(),
                receiver: OriginalVariableSymbolReceiver::Operand(
                    tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined,
                ),
            }
        }));
        anchors
    }

    fn original_alias_source_has_later_blocker(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        alias: &OriginalVariableAliasReceipt,
        anchor: Span,
    ) -> bool {
        let Some(realm) = self.retained_command_realm() else {
            return true;
        };
        let Some(input) = self.resolved_input.as_ref() else {
            return true;
        };
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&config);
        self.original_variable_alias_operands
            .iter()
            .any(|&(site, local, _, purpose)| {
                if site <= alias.invocation_offset()
                    || site > anchor.start()
                {
                    return false;
                }
                let local_input = if input.has_logical_source_name_context() {
                    realm.original_written_word_at_span_in_source(image, local, config)
                        .and_then(|(word, _)| crate::signature_scan::scope::SignatureSourceNameKey::from_original_logical_source_word(
                            &word, rules, alias.target().policy(),
                        ))
                        .map(SignatureSourceNameInput::OriginalWord)
                } else { realm
                    .original_written_name_input_at_span_in_source(image, local, config)
                    .or_else(|| {
                        realm
                            .original_source_name_at_span(
                                local,
                                config,
                                rules,
                                alias.target().policy(),
                            )
                            .map(|name| {
                                SignatureSourceNameInput::OriginalWord(name.name_input().clone())
                            })
                    }) };
                let frame = if input.has_logical_source_name_context() {
                    let Some(local_input) = local_input.as_ref() else { return true; };
                    let Some(scope) = realm.original_logical_variable_scope_at(input, image, local, local_input) else { return true; };
                    let Some(frame) = scope.alias_frame() else { return true; };
                    Some(frame)
                } else { realm.source_bindings_ref().original_variable_frame_at_span(local, config) };
                if frame.as_ref() != Some(alias.frame()) { return false; }
                local_input
                    .and_then(|input| original_variable_alias_local_name(&input, purpose))
                    .is_none_or(|local| &local == alias.local())
            })
    }
}

fn original_frame_command_sites(frame: &SourceOriginalVariableFrame) -> Option<HashSet<u32>> {
    let source = frame.source();
    let ExecutedScriptMapping::Contiguous { base } = source.mapping else {
        return None;
    };
    let end = base.checked_add(u32::try_from(source.text.bytes().len()).ok()?)?;
    let plan = tcl_lexer::native_script_words_in(
        source.origin.source_image().clone(),
        Span::new(base, end),
        frame.lexer_config(),
    )
    .ok()?;
    if plan.fatal_tail.is_some() {
        return None;
    }
    plan.commands
        .iter()
        .map(|command| Some(command.words.first()?.span().start()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;
    use crate::signature_scan::variable_symbol::{
        OriginalVariableAliasTemplateObligation, SignatureSourceVariableSlot,
    };

    #[test]
    fn logical_alias_templates_keep_actual_procedure_and_namespace_source_frames() {
        for source in [
            "proc p {i} {upvar ::N::array($i) local; puts $local}; proc other {} {puts $local}",
            "namespace eval N {namespace upvar ::other remote local; puts $local}; namespace eval N {puts $local}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl");
            let image = SourceImage::document(source);
            let config = analysis.body_lexer_config.unwrap();
            let templates = analysis
                .original_variable_alias_templates_in_source(&image, config)
                .unwrap();
            let read = u32::try_from(source.find("$local").unwrap()).unwrap();
            let other = u32::try_from(source.rfind("$local").unwrap()).unwrap();
            let template = templates
                .iter()
                .find(|template| template.span().start() == read)
                .unwrap_or_else(|| panic!("genuine Logical source alias: {source}: {templates:?}"));
            assert_eq!(template.local_name().as_bytes(), b"local");
            assert!(!template.obligations().is_empty());
            assert!(
                templates
                    .iter()
                    .all(|template| template.span().start() != other)
            );
            assert_source_guards(source, &analysis, config, &image);
        }
        let source = "proc p {} {upvar ::one value; upvar ::two value; puts $value}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let templates = analysis
            .original_variable_alias_templates_in_source(
                &SourceImage::document(source),
                analysis.body_lexer_config.unwrap(),
            )
            .unwrap();
        let read = u32::try_from(source.find("$value").unwrap()).unwrap();
        let template = templates
            .iter()
            .find(|template| template.span().start() == read)
            .unwrap();
        assert_eq!(
            template
                .target_symbol()
                .checked_c_namespace_key()
                .as_deref(),
            Some("::two")
        );
    }

    #[test]
    fn logical_alias_templates_reborrow_source_cards_without_ordinary_alias_authority() {
        let source = "proc p {i} {upvar ::n::a($i) data; puts $data}; proc q {i} {upvar ::n::b($i) data; puts $data}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let image = SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        assert!(analysis.original_variable_alias_receipts.is_empty());
        let templates = analysis
            .original_variable_alias_templates_in_source(&image, config)
            .unwrap();
        let reads = [
            source.find("$data").unwrap(),
            source.rfind("$data").unwrap(),
        ]
        .map(|offset| {
            templates
                .iter()
                .find(|template| template.span().start() == u32::try_from(offset).unwrap())
                .unwrap()
        });
        assert_ne!(reads[0].original_frame(), reads[1].original_frame());
        for (read, target) in reads.into_iter().zip(["::n::a", "::n::b"]) {
            assert_eq!(
                read.target_symbol().checked_c_namespace_key().as_deref(),
                Some(target)
            );
            assert!(read.target_name_input().original_word_key().is_none());
            assert!(read.target_name_input().produced_value().is_none());
            assert_eq!(read.obligations().len(), 3);
            let ordinary = analysis
                .original_variable_symbol_in_source(&image, config, read.span().start())
                .unwrap();
            assert_ne!(ordinary.symbol(), read.target_symbol());
            assert!(ordinary.original_alias_declaration_span().is_none());
        }
        let mut missing_source = analysis.clone();
        missing_source
            .original_variable_alias_source_operands
            .clear();
        assert!(
            missing_source
                .original_variable_alias_templates_in_source(&image, config)
                .unwrap()
                .is_empty()
        );
        assert_source_guards(source, &analysis, config, &image);
    }

    #[test]
    fn logical_alias_templates_veto_newer_unknown_locals_and_dynamic_targets() {
        for body in [
            "upvar ::n::a data; upvar ::n::b $which; puts $data",
            "upvar ::n::a data; upvar $target data; puts $data",
            "upvar ::n::a data; if 1 {upvar ::n::b data}; puts $data",
        ] {
            let source = format!("proc p {{which target}} {{{body}}}");
            let analysis = Analyser::new().analyse(&source, "tcl");
            let templates = analysis
                .original_variable_alias_templates_in_source(
                    &SourceImage::document(&source),
                    analysis.body_lexer_config.unwrap(),
                )
                .unwrap();
            let read = u32::try_from(source.find("$data").unwrap()).unwrap();
            assert!(
                templates
                    .iter()
                    .all(|template| template.span().start() != read),
                "{body}"
            );
            assert!(analysis.original_variable_alias_receipts.is_empty());
        }
    }

    #[test]
    fn original_alias_source_templates_keep_unknown_elements_and_source_order_separate() {
        // Implementation contract: naming.variable.original-alias-source-template
        // docs/design/analysis/name-resolution-proofs/original-alias-source-template.md
        let source = "proc p {i} {upvar ::n::a($i) data; lappend data value; puts $data}; proc sibling {} {puts $data}";
        for version in tcl_dialect::TclVersion::ALL {
            let analysis = Analyser::new().analyse(source, version.dialect_name());
            let config = analysis.body_lexer_config.unwrap();
            let image = SourceImage::document(source);
            let templates = analysis
                .original_variable_alias_templates_in_source(&image, config)
                .unwrap_or_else(|| panic!("{}", version.dialect_name()));
            let expected = [
                source.find(" data;").unwrap() + 1,
                source.find("data value").unwrap(),
                source.find("$data").unwrap(),
            ];
            assert_eq!(
                templates
                    .iter()
                    .map(|template| usize::try_from(template.span().start()).unwrap())
                    .collect::<Vec<_>>(),
                expected,
                "{}",
                version.dialect_name()
            );
            for template in &templates {
                assert_eq!(template.local_name().as_bytes(), b"data");
                let SignatureSourceVariableSlot::C { namespace, simple } =
                    template.target_symbol().slot()
                else {
                    panic!("wrong naming domain");
                };
                assert_eq!(
                    namespace.as_segments(),
                    [tcl_core_types::NameBytes::from(&b"n"[..])]
                );
                assert_eq!(simple.as_bytes(), b"a");
                assert!(template.target_name_input().original_word_key().is_none());
                assert!(template.target_name_input().produced_value().is_none());
                assert!(
                    template
                        .obligations()
                        .contains(&OriginalVariableAliasTemplateObligation::UnavailableTargetCell)
                );
            }
            assert!(templates[0].is_declaration());
            assert!(!templates[1].is_declaration() && !templates[2].is_declaration());
            assert_source_guards(source, &analysis, config, &image);
            assert_source_order_guards(version);
        }
    }
    #[test]
    fn original_alias_templates_join_qualified_targets_in_their_current_source_frame() {
        // naming.variable.original-alias-source-template
        // docs/design/analysis/name-resolution-proofs/original-alias-source-template.md
        let source = "namespace eval ns {variable foo 1}; proc p {} {global ::ns::foo; puts $foo; puts $::ns::foo; puts $::other::foo}; proc sibling {} {puts $foo}";
        let logical = tcl_dialect::DialectProfile::plain_tcl();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            logical,
            logical,
            tcl_registry::model::resolve_environment("tcl9.0").default_context_registry(),
            LexerConfig::for_file_grammar(logical.grammar),
        );
        let mut analyses = tcl_dialect::TclVersion::ALL
            .into_iter()
            .map(|version| Analyser::new().analyse(source, version.dialect_name()))
            .collect::<Vec<_>>();
        analyses.push(
            Analyser::new()
                .with_resolved_input(input)
                .analyse(source, logical.name),
        );
        let declaration =
            u32::try_from(source.find("global ::ns::foo").unwrap() + "global ".len()).unwrap();
        let local = u32::try_from(source.find("$foo").unwrap()).unwrap();
        let qualified = u32::try_from(source.find("$::ns::foo").unwrap()).unwrap();
        let other = u32::try_from(source.find("$::other::foo").unwrap()).unwrap();
        let sibling = u32::try_from(source.rfind("$foo").unwrap()).unwrap();
        let image = SourceImage::document(source);
        for analysis in analyses {
            let config = analysis.body_lexer_config.unwrap();
            let templates = analysis
                .original_variable_alias_templates_in_source(&image, config)
                .expect("current complete source");
            for offset in [declaration, local, qualified] {
                let template = templates
                    .iter()
                    .find(|template| template.span().start() == offset)
                    .unwrap_or_else(|| {
                        panic!("{} missing {offset}: {:?}", analysis.dialect, templates)
                    });
                assert_eq!(template.declaration_span().start(), declaration);
                let SignatureSourceVariableSlot::C { namespace, simple } =
                    template.target_symbol().slot()
                else {
                    panic!("source namespace geometry");
                };
                assert_eq!(
                    namespace.as_segments(),
                    [tcl_core_types::NameBytes::from(&b"ns"[..])]
                );
                assert_eq!(simple.as_bytes(), b"foo");
            }
            assert!(
                templates
                    .iter()
                    .all(|template| ![other, sibling].contains(&template.span().start()))
            );
            assert_source_guards(source, &analysis, config, &image);
            let mut missing = analysis.clone();
            missing.resolved_input = None;
            assert!(
                missing
                    .original_variable_alias_templates_in_source(&image, config)
                    .is_none()
            );
            let mut unavailable = analysis;
            unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
                environment: "tcl9.0".to_owned(),
                overlay: u64::MAX,
            });
            assert!(
                unavailable
                    .original_variable_alias_templates_in_source(&image, config)
                    .is_none()
            );
        }
    }

    fn assert_source_guards(
        source: &str,
        analysis: &AnalysisResult,
        config: LexerConfig,
        image: &SourceImage,
    ) {
        assert!(
            analysis
                .original_variable_alias_templates_in_source(
                    &SourceImage::document(&format!("{source}\n")),
                    config
                )
                .is_none()
        );
        let mut changed_config = config;
        changed_config.leading_bom = match config.leading_bom {
            tcl_lexer::LeadingBom::Content => tcl_lexer::LeadingBom::Skip,
            tcl_lexer::LeadingBom::Skip => tcl_lexer::LeadingBom::Content,
        };
        assert!(
            analysis
                .original_variable_alias_templates_in_source(image, changed_config)
                .is_none()
        );
        let mut foreign = analysis.clone();
        foreign.resolved_input = Some(
            Analyser::new()
                .analyse(source, "jim")
                .resolved_input
                .unwrap(),
        );
        assert!(
            foreign
                .original_variable_alias_templates_in_source(image, config)
                .is_none()
        );
    }

    fn assert_source_order_guards(version: tcl_dialect::TclVersion) {
        for transition in ["upvar 0 $target data", "if {$flag} {upvar ::n::b($i) data}"] {
            let source = format!(
                "proc p {{i target flag}} {{upvar ::n::a($i) data; {transition}; lappend data after}}"
            );
            let analysis = Analyser::new().analyse(&source, version.dialect_name());
            let templates = analysis
                .original_variable_alias_templates_in_source(
                    &SourceImage::document(&source),
                    analysis.body_lexer_config.unwrap(),
                )
                .unwrap();
            let after = u32::try_from(source.find("data after").unwrap()).unwrap();
            assert!(
                templates
                    .iter()
                    .all(|template| template.span().start() != after),
                "{}: {transition}",
                version.dialect_name()
            );
        }
        let source =
            "proc p {i} {upvar ::n::a($i) data; upvar ::n::b($i) data; lappend data after}";
        let analysis = Analyser::new().analyse(source, version.dialect_name());
        let templates = analysis
            .original_variable_alias_templates_in_source(
                &SourceImage::document(source),
                analysis.body_lexer_config.unwrap(),
            )
            .unwrap();
        let after = u32::try_from(source.find("data after").unwrap()).unwrap();
        let template = templates
            .iter()
            .find(|template| template.span().start() == after)
            .expect("latest direct source alias");
        let SignatureSourceVariableSlot::C { simple, .. } = template.target_symbol().slot() else {
            panic!("wrong target");
        };
        assert_eq!(simple.as_bytes(), b"b");
    }
}
