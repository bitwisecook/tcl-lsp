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
        if self.original_variable_alias_receipts.is_empty() {
            return Some(Vec::new());
        }
        let bindings = self.retained_command_realm()?.source_bindings_ref();
        let mut direct = HashMap::<SourceOriginalVariableFrame, Option<HashSet<u32>>>::new();
        let mut templates = Vec::new();
        for anchor in self.original_alias_source_anchors() {
            let Some(frame) = bindings.original_variable_frame_at_span(anchor.span, config) else {
                continue;
            };
            let mut candidates = self
                .original_variable_alias_receipts
                .iter()
                .filter(|alias| {
                    alias.frame() == &frame
                        && alias.invocation_offset() <= anchor.span.start()
                        && bindings.original_variable_alias_template_matches_at_span(
                            anchor.span,
                            &anchor.input,
                            anchor.receiver,
                            alias,
                            config,
                        )
                })
                .collect::<Vec<_>>();
            candidates.sort_by_key(|alias| alias.invocation_offset());
            let Some(alias) = candidates.pop() else {
                continue;
            };
            if candidates.iter().any(|other| {
                other.invocation_offset() == alias.invocation_offset() && *other != alias
            }) || self.original_alias_source_has_later_blocker(image, config, alias, anchor.span)
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
            let template =
                OriginalVariableAliasTemplate::new(anchor.span, anchor.input, alias.clone());
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
        let rules = tcl_registry::InvocationDialect::of_profile(input.unit_profile()).word_values;
        self.original_variable_alias_operands
            .iter()
            .any(|&(site, local, _, purpose)| {
                if site <= alias.invocation_offset()
                    || site > anchor.start()
                    || realm
                        .source_bindings_ref()
                        .original_variable_frame_at_span(local, config)
                        .as_ref()
                        != Some(alias.frame())
                {
                    return false;
                }
                let local_input = realm
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
                    });
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
