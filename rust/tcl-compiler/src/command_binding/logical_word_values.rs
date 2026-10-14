// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original Logical word values, independently of static words and Native reads.

use super::{
    Arc, ModuleCommandBindings, SourceExecutionContext, SourceNamespaceKey, SourceOriginId,
};
use crate::analyser::ResolvedAnalysisInput;
use crate::ir::SourceSite;
use crate::var_resolve::{ContentsOrigin, ResolveContext};
use tcl_lexer::{ExecutablePart, NativeWord};

/// A current source value retains its original word and stored read provenance.
/// It cannot become a static word, Native argv, read or completed invocation.
pub(super) struct LogicalOriginalWordValue {
    input: ResolvedAnalysisInput,
    source: Arc<SourceOriginId>,
    frame: crate::var_resolve::VariableExecutionFrame,
    namespace: SourceNamespaceKey,
    variables: Arc<ResolveContext>,
    word: NativeWord,
    value: String,
}

impl LogicalOriginalWordValue {
    pub(super) fn capture_invocation(
        words: &[crate::ir::WordExpr],
        offset: u32,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Vec<Self>> {
        super::logical_definition::original_invocation_words(words, offset, state, context)?
            .into_iter()
            .map(|word| Self::capture(word, state, context))
            .collect()
    }

    fn capture(
        word: NativeWord,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        let input = super::logical_definition::logical_entry(state, context)?;
        let source = state.current_source_origin.as_ref()?;
        if word.image() != source.source_image()
            || word.config() != context.config
            || state.variable_frame != *context.frame
            || state.source_variables.namespace != context.namespace
        {
            return None;
        }
        let bytes = tcl_syntax::word_rules::original_word_unicode_value_with_variables(
            &word,
            |arena, component| {
                let ExecutablePart::Variable { name, index: None } = component.part else {
                    return None;
                };
                if tcl_syntax::naming::split_element_ref_bytes(arena.bytes(name)?).is_some() {
                    return None;
                }
                let spelling = std::str::from_utf8(arena.bytes(component.span)?).ok()?;
                let original = (spelling, SourceSite::source(component.span));
                let access =
                    super::original_variable_compilation::logical_original_substitution_access(
                        Some(&original),
                        Some(arena),
                        state,
                        context,
                    )?;
                if access.dynamic
                    || access.observed
                    || access.kind != crate::place::PlaceKind::Scalar
                    || access.index.is_some()
                    || !super::logical_operation::logical_cell_is_local(&access, state, context)
                    || !state.source_variables.contents_have_source(&access, source)
                    || !matches!(
                        state.source_variables.contents_origin(&access),
                        ContentsOrigin::WrittenAt(_)
                    )
                {
                    return None;
                }
                Some(
                    state
                        .source_variables
                        .literal_contents_at(&access, context.registry)?
                        .as_bytes()
                        .to_vec(),
                )
            },
        )?;
        Some(Self {
            input: input.clone(),
            source: Arc::clone(source),
            frame: state.variable_frame.clone(),
            namespace: context.namespace_identity(),
            variables: Arc::clone(&state.source_variables),
            word,
            value: String::from_utf8(bytes).ok()?,
        })
    }

    pub(super) fn is_current(
        &self,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> bool {
        super::logical_definition::logical_entry(state, context) == Some(&self.input)
            && state.current_source_origin.as_ref() == Some(&self.source)
            && state.variable_frame == self.frame
            && context.namespace_identity() == self.namespace
            && state.source_variables == self.variables
            && self.word.image() == self.source.source_image()
            && self.word.config() == context.config
    }

    pub(super) fn word(&self) -> &NativeWord {
        &self.word
    }

    pub(super) fn value(&self) -> &str {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};

    fn inspect_original_call(
        source: &str,
        inspect: impl FnOnce(
            &ModuleCommandBindings,
            SourceExecutionContext<'_>,
            &crate::ir::CommandTokens,
        ),
    ) -> bool {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        assert!(input.has_logical_source_name_context());
        let current = input.context_registry();
        let registry = current.commands();
        let options = SourceAnalysisOptions::for_logical_source(&input).unwrap();
        let image = tcl_lexer::SourceImage::document(source);
        let frame = crate::var_resolve::VariableExecutionFrame::Global;
        let bindings = SourceCommandBindings::analyse_image_in_frame_with_options(
            &image,
            &frame,
            input.lexer_config(),
            registry,
            options,
        )
        .unwrap();
        let segments = crate::segmenter::segment_commands_image_with_offset_and_config(
            &image,
            0,
            input.lexer_config(),
        )
        .unwrap();
        let segment = segments.last().unwrap();
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &image.source_map(),
            input.lexer_config(),
            segment,
        );
        bindings.stamp_original_tokens(&mut tokens);
        let Some(binding) = tokens.source_binding.as_ref() else {
            return false;
        };
        let Some(snapshot) = binding.lookup_state.as_ref() else {
            return false;
        };
        let state = &snapshot.state;
        let mut context = super::super::root_source_execution_context(
            &state.variable_frame,
            &state.source_variables.namespace,
            &binding.lookup_namespace_key,
            input.lexer_config(),
            registry,
            options,
            tcl_lexer::SourceSegmentationPurpose::WholeInput,
        );
        context.invocation_offset = segment.span.start();
        inspect(state, context, &tokens);
        true
    }

    #[test]
    fn original_logical_declaration_values_retain_reads_without_rewriting_static_words() {
        assert!(inspect_original_call(
            "set name {child}; set body {return original}; proc $name {} $body",
            |state, context, tokens| {
                let values = LogicalOriginalWordValue::capture_invocation(
                    tokens.words(),
                    tokens.words()[0].source().span.start(),
                    state,
                    context,
                )
                .expect("exact quiet Logical scalar reads");
                assert_eq!(
                    values
                        .iter()
                        .map(LogicalOriginalWordValue::value)
                        .collect::<Vec<_>>(),
                    ["proc", "child", "", "return original"]
                );
                assert!(values.iter().all(|value| value.is_current(state, context)));
                assert!(super::super::logical_definition::static_value(values[1].word()).is_none());
                assert!(super::super::logical_definition::static_value(values[3].word()).is_none());
                assert!(state.baseline.native_entry.is_none());
                assert!(state.source_variables.interpreter.is_none());
                assert!(state.source_variables.execution.is_none());
            },
        ));
    }

    #[test]
    fn original_logical_declaration_values_refuse_changed_trace_contents_and_source_owners() {
        assert!(inspect_original_call(
            "set name {child}; set body {return original}; proc $name {} $body",
            |state, context, tokens| {
                let offset = tokens.words()[0].source().span.start();
                let values = LogicalOriginalWordValue::capture_invocation(
                    tokens.words(),
                    offset,
                    state,
                    context,
                )
                .expect("actual original Logical values");
                let mut changed = state.clone();
                Arc::make_mut(&mut changed.source_variables).dynamic_traces = true;
                assert!(
                    LogicalOriginalWordValue::capture_invocation(
                        tokens.words(),
                        offset,
                        &changed,
                        context
                    )
                    .is_none()
                );
                assert!(!values[1].is_current(&changed, context));
                let mut changed = state.clone();
                Arc::make_mut(&mut changed.source_variables).define_literal(
                    "name",
                    "changed",
                    context.registry,
                );
                assert!(!values[1].is_current(&changed, context));
                let mut changed = state.clone();
                changed.current_source_origin = Some(Arc::new(SourceOriginId::authored_image(
                    tcl_lexer::SourceImage::document("different original source"),
                )));
                assert!(
                    LogicalOriginalWordValue::capture_invocation(
                        tokens.words(),
                        offset,
                        &changed,
                        context
                    )
                    .is_none()
                );
                assert!(!values[1].is_current(&changed, context));
                assert!(
                    LogicalOriginalWordValue::capture_invocation(
                        tokens.words(),
                        offset + 1,
                        state,
                        context
                    )
                    .is_none()
                );
            },
        ));
    }

    #[test]
    fn original_logical_declaration_values_refuse_unknown_and_array_operands() {
        for source in [
            "set body {return original}; proc $missing {} $body",
            "set name {child}; set body {return original}; proc $name {} $body(index)",
            "set name {child}; set body {return original}; proc $name {} ${body(index)}",
            "set name {child}; set body {return original}; proc $name {} [opaque]",
        ] {
            inspect_original_call(source, |state, context, tokens| {
                assert!(
                    LogicalOriginalWordValue::capture_invocation(
                        tokens.words(),
                        tokens.words()[0].source().span.start(),
                        state,
                        context,
                    )
                    .is_none(),
                    "{source}"
                );
            });
        }
    }
}
