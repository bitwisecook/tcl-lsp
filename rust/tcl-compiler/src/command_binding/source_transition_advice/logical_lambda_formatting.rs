// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional decoded lambda source phases for readonly formatting roles.

use super::{
    AdviceGraph, AdviceInvocation, AdviceInvocationContext, AdviceNamingPolicy, Arc, BTreeMap,
    BTreeSet, NativeWord, OriginalSourceCommandTransitionAdvice,
    OriginalSourceTransitionAdviceTape, SourceCommandTransitionObligation, body_effects,
    logical_navigation, source_class,
};

/// A configured lambda field with actual parent graph and distinct source origin.
/// No Document root, entered frame, Native interpretation or edit is issued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalConditionalLambdaFormattingBody {
    input: crate::analyser::ResolvedAnalysisInput,
    parent: Arc<OriginalSourceCommandTransitionAdvice>,
    word: NativeWord,
    origin: Arc<crate::command_binding::SourceOriginId>,
    fields: Vec<crate::signature_scan::name_value::LogicalSourceListNameInput>,
    argument_count: tcl_registry::InvocationArgumentCount,
    advice: BTreeMap<u32, OriginalSourceCommandTransitionAdvice>,
    nested: BTreeMap<u32, Option<Arc<Self>>>,
}
impl OriginalConditionalLambdaFormattingBody {
    /// Reborrow the actual conditional field phase retained by this whole source.
    /// A decoded string or another original allocation cannot supply it.
    #[must_use]
    pub fn capture(
        realm: &crate::realm::CommandBindingRealm,
        input: &crate::analyser::ResolvedAnalysisInput,
        word: &NativeWord,
    ) -> Option<Arc<Self>> {
        realm.original_conditional_lambda_formatting_body(word, input)
    }
    /// Decoded body bytes, in their own conditional source instance.
    #[must_use]
    pub fn image(&self) -> &tcl_lexer::SourceImage {
        self.origin.source_image()
    }
    /// Conditional roles at an unchanged complete command in this exact phase.
    #[must_use]
    pub fn words(
        &self,
        segment: &crate::segmenter::SegmentedCommand,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<crate::registry_invocation::source_structure::OriginalRegistryWords> {
        if input != &self.input
            || !input.has_logical_source_name_context()
            || self.fields.get(1)?.original_field().value().as_bytes() != self.image().bytes()
        {
            return None;
        }
        let advice = self.advice.get(&segment.argv.first()?.span.start())?;
        let context = input.borrowed_context_registry();
        crate::registry_invocation::source_structure::conditional_lambda_formatting_words(
            self.image().try_text().ok()?,
            input.lexer_config(),
            context,
            segment,
            advice,
        )
    }
    fn matches_parent(
        phase: &Self,
        word: &NativeWord,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> bool {
        if &phase.input != input
            || &phase.word != word
            || !phase
                .parent
                .original_words()
                .iter()
                .any(|parent| parent == word)
        {
            return false;
        }
        let context = input.borrowed_context_registry();
        let Ok(source) = phase.parent.site().source.source_image().try_text() else {
            return false;
        };
        crate::registry_invocation::source_structure::source_transition_words_from_advice(
            source,
            input.lexer_config(),
            context,
            phase.parent.as_ref().clone(),
        )
        .and_then(|words| {
            words
                .with_source_schema(
                    context,
                    crate::registry_invocation::source_schema_queries::lambda_call,
                )
                .flatten()
        })
        .is_some_and(|call| call.count == phase.argument_count)
    }
    /// Reborrow an actual nested field phase using its complete original word.
    #[must_use]
    pub fn child(
        &self,
        word: &NativeWord,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Arc<Self>> {
        let child = self
            .nested
            .values()
            .flatten()
            .find(|child| Self::matches_parent(child, word, input))?;
        Some(Arc::clone(child))
    }
}
impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn decoded_lambda_formatting_phase(
        &self,
        word: &NativeWord,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Arc<OriginalConditionalLambdaFormattingBody>> {
        let phase = self
            .decoded_lambda_phases
            .values()
            .flatten()
            .find(|phase| {
                OriginalConditionalLambdaFormattingBody::matches_parent(phase, word, input)
            })?;
        Some(Arc::clone(phase))
    }
}
impl AdviceInvocationContext<'_> {
    pub(super) fn retain_logical_lambda_formatting_phase(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        scope: Option<&Arc<crate::command_binding::OriginalDeclaredLogicalBodyContext>>,
    ) {
        let Some(call) = schema.authored_source_lambda_call() else {
            return;
        };
        let Some(head) = invocation.native.first() else {
            return;
        };
        let phase = self.logical_lambda_formatting_phase(graph, invocation, schema, call, scope);
        source_class::merge_receipt(
            &mut tape.decoded_lambda_phases,
            head.span().start(),
            phase.map(Arc::new),
        );
    }
    fn logical_lambda_formatting_phase(
        &self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        call: tcl_registry::AuthoredSourceLambdaCall,
        scope: Option<&Arc<crate::command_binding::OriginalDeclaredLogicalBodyContext>>,
    ) -> Option<OriginalConditionalLambdaFormattingBody> {
        // naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let AdviceNamingPolicy::Logical(input) = self.policy else {
            return None;
        };
        let mut depth = 0usize;
        let mut ancestor = self.origin.as_ref();
        while let crate::command_binding::SourceOriginKind::ConditionalField { parent, .. } =
            ancestor.kind()
        {
            depth = depth.checked_add(1)?;
            if depth >= 64 {
                return None;
            }
            ancestor = &parent.source;
        }
        let mut parent = self.schema_advice(invocation, graph, schema)?;
        parent.logical_scope = scope.cloned();
        parent.logical_body = scope.map(|scope| Arc::clone(scope.body_arc()));
        let parent = Arc::new(parent);
        let words = body_effects::boxed_region_words(self, &parent)?;
        let word = words.original_argument_word(call.lambda_argument)?.clone();
        if !invocation.native.iter().any(|original| original == &word) {
            return None;
        }

        let policy = self.dialect.authored_name_policy()?;
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                input, None, policy,
            );
        let fields = purpose.logical_list_inputs(
            &word,
            tcl_syntax::word_rules::WordValueRules::from_config(&self.config),
        )?;
        if !(2..=3).contains(&fields.len()) {
            return None;
        }
        if crate::lambda_literal::split_original_lambda_literal(&word)
            .and_then(|lambda| lambda.braced_body())
            .and_then(|span| word.image().try_text().ok()?.get(span.as_range()))
            == Some(fields.get(1)?.original_field().value())
        {
            // The existing original-body owner already retains unchanged text.
            return None;
        }
        let crate::signature_scan::scope::SignatureNamespaceScope::C(namespace) =
            crate::signature_scan::scope::SignatureNamespaceScope::from_logical_lambda_list(
                &fields,
            )?
        else {
            return None;
        };
        let image = tcl_lexer::SourceImage::document(fields.get(1)?.original_field().value());
        let origin = Arc::new(crate::command_binding::SourceOriginId::from_kind(
            crate::command_binding::SourceOriginKind::ConditionalField {
                parent: crate::command_binding::CommandAllocationSite {
                    source: Arc::clone(self.origin),
                    offset: invocation.native.first()?.span().start(),
                },
                argument: call.lambda_argument,
                field: 1,
                source: image.clone(),
            },
        ));
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            image.clone(),
            tcl_lexer::Span::new(0, u32::try_from(image.bytes().len()).ok()?),
            self.config,
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .ok()?;
        if plan.fatal_tail.is_some() {
            return None;
        }
        let context = AdviceInvocationContext {
            origin: &origin,
            context: self.context,
            config: self.config,
            dialect: self.dialect,
            policy: self.policy,
            query: None,
        };
        let mut branch = graph.clone();
        branch.logical_namespace = Some(namespace);
        branch.handles.clear();
        branch.class_handles.clear();
        let mut tape = OriginalSourceTransitionAdviceTape::default();
        for command in plan.commands {
            let step = graph
                .decoded_phase_steps
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if step >= 2048 {
                return None;
            }
            context.retain_original_invocation(
                &mut branch,
                &mut tape,
                &command,
                logical_navigation::LogicalCallbackScope::Unavailable,
            )?;
        }
        let offsets: BTreeSet<_> = tape
            .advice
            .keys()
            .chain(tape.body_catalogue.keys())
            .copied()
            .collect();
        let advice = offsets
            .into_iter()
            .filter_map(|offset| {
                let mut advice =
                    OriginalSourceTransitionAdviceTape::prefix_catalogue_advice(&tape, offset)?
                        .clone();
                advice
                    .obligations
                    .push(SourceCommandTransitionObligation::DeferredLogicalBodyApplicability);
                Some((offset, advice))
            })
            .collect();
        Some(OriginalConditionalLambdaFormattingBody {
            input: input.clone(),
            parent,
            word,
            origin,
            fields,
            argument_count: call.count,
            advice,
            nested: tape.decoded_lambda_phases,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn phase(
        source: &str,
    ) -> (
        crate::analyser::ResolvedAnalysisInput,
        Option<Arc<OriginalConditionalLambdaFormattingBody>>,
    ) {
        let mut analyser = crate::analyser::Analyser::new();
        let input = analyser.prepare_analysis_input("tcl").unwrap();
        let analysis = analyser
            .with_resolved_input(input.clone())
            .analyse(source, "tcl");
        let commands = crate::segmenter::segment_commands_with_offset_and_config(
            source,
            0,
            input.lexer_config(),
        );
        let command = commands.last().unwrap();
        let words = crate::registry_invocation::source_structure::source_registry_words(
            source, &analysis, command,
        )
        .unwrap();
        let word = words.original_argument_word(0).unwrap();
        let result = OriginalConditionalLambdaFormattingBody::capture(
            analysis.retained_command_realm().unwrap(),
            &input,
            word,
        );
        (input, result)
    }
    #[test]
    fn logical_decoded_phase_keeps_original_parent_and_conditional_role_debt() {
        let source = r"apply {{} if\ 1\ \{puts\ hi\} ::}";
        let (input, phase) = phase(source);
        let phase = phase.expect("actual configured Logical lambda field");
        assert_eq!(phase.image().try_text().unwrap(), "if 1 {puts hi}");
        assert!(
            matches!(phase.origin.kind(), crate::command_binding::SourceOriginKind::ConditionalField { parent, argument: 0, field: 1, .. } if parent.offset == 0 && parent.source.source_image().bytes() == source.as_bytes())
        );
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            phase.image().try_text().unwrap(),
            0,
            input.lexer_config(),
        )
        .remove(0);
        let words = phase
            .words(&command, &input)
            .expect("actual conditional child roles");
        assert!(
            words
                .roles()
                .unwrap()
                .iter()
                .any(|(_, role)| *role == tcl_registry::ArgRole::Body)
        );
        assert!(!words.operands_preserve_source_lookup());
        let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
            advice,
        ) = words.source()
        else {
            panic!("source phase only");
        };
        assert!(advice.original_head().native_input().is_none());
        assert!(
            advice
                .obligations()
                .contains(&SourceCommandTransitionObligation::DeferredLogicalBodyApplicability)
        );
        assert!(advice.logical_source_scope().is_none());
        assert_eq!(
            advice.original_words().first().unwrap().image(),
            phase.image()
        );
        let foreign = crate::segmenter::segment_commands_with_offset_and_config(
            "if 1 {puts no}",
            0,
            input.lexer_config(),
        )
        .remove(0);
        assert!(phase.words(&foreign, &input).is_none());
        let mut analyser = crate::analyser::Analyser::new();
        let foreign_input = analyser.prepare_analysis_input("tcl8.4").unwrap();
        assert!(phase.words(&command, &foreign_input).is_none());
        let literal_config = tcl_lexer::LexerConfig {
            brace_backslash_newline: if input.lexer_config().brace_backslash_newline.folds() {
                tcl_dialect::BraceBackslashNewline::Literal
            } else {
                tcl_dialect::BraceBackslashNewline::Folds
            },
            ..input.lexer_config()
        };
        let configured = crate::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            input.context_registry(),
            literal_config,
        );
        assert!(phase.words(&command, &configured).is_none());
    }
    #[test]
    fn logical_field_phase_does_not_recover_a_shadowed_registry_command() {
        let (input, phase) = phase(r"proc if args {}; apply {{} if\ 1\ \{puts\ hi\}}");
        let phase = phase.expect("the lambda field remains authentic");
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            phase.image().try_text().unwrap(),
            0,
            input.lexer_config(),
        )
        .remove(0);
        assert!(phase.words(&command, &input).is_none());
        assert!(self::phase("apply {$body}").1.is_none());
    }
    #[test]
    fn decoded_logical_unicode_and_nested_bom_keep_the_field_grammar() {
        let (input, unicode) = phase(r"apply {{} if\ 1\ \{puts\ \u00e9\}}");
        let unicode = unicode.unwrap();
        assert_eq!(unicode.image().try_text().unwrap(), "if 1 {puts é}");
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            unicode.image().try_text().unwrap(),
            0,
            input.lexer_config(),
        )
        .remove(0);
        assert!(unicode.words(&command, &input).is_some());
        let (input, bom) = phase("apply {{} \u{feff}if\\ 1\\ \\{puts\\ hi\\}}");
        let bom = bom.unwrap();
        assert!(bom.image().try_text().unwrap().starts_with('\u{feff}'));
        let command = crate::segmenter::segment_commands_with_offset_and_config(
            bom.image().try_text().unwrap(),
            0,
            input.lexer_config().nested(),
        )
        .remove(0);
        assert_eq!(command.argv[0].span.start(), 0);
        assert_eq!(command.texts[0], "\u{feff}if");
        assert!(bom.words(&command, &input).is_none());
        assert!(phase("apply {{} body ::N extra}").1.is_none());
    }
}
