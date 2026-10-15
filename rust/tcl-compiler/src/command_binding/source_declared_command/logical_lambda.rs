// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional Logical lambda source, independent of body entry or Native frames.

use crate::analyser::ResolvedAnalysisInput;
use crate::registry_invocation::source_structure::OriginalRegistryWords;
use crate::signature_scan::scope::SignatureNamespaceScope;
use std::sync::Arc;
use tcl_lexer::{NativeWord, SourceImage, Span};

/// The actual selected source call, unchanged list roster and body ancestry.
/// Namespace, formals and argument topology retain separate source purposes;
/// none establishes an entered lambda, cell, implementation or Normal result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalLogicalLambdaBody {
    input: ResolvedAnalysisInput,
    words: Arc<OriginalRegistryWords>,
    producer: Vec<NativeWord>,
    lambda: NativeWord,
    body: Arc<crate::registry_invocation::OriginalSourceScriptBody>,
    content: Span,
    namespace: SignatureNamespaceScope,
    fields: Vec<crate::signature_scan::name_value::LogicalSourceListNameInput>,
    formals: Option<Arc<crate::signature_scan::formal_parameters::LogicalSourceFormalParameters>>,
}

impl OriginalLogicalLambdaBody {
    pub(crate) fn capture(
        input: &ResolvedAnalysisInput,
        words: OriginalRegistryWords,
        producer: &[NativeWord],
        purpose: crate::signature_scan::original_name::OriginalSourceNamePurpose,
    ) -> Option<Self> {
        let context = input.context_registry();
        let image = producer.first()?.image();
        if !input.has_logical_source_name_context()
            || producer
                .iter()
                .any(|word| word.image() != image || word.config() != input.lexer_config())
            || !words.matches_source(image, input.lexer_config())
            || !words.matches_registry(context.commands())
            || words.context() != Some(context.context())
            || words.origins().len() != producer.len()
            || words.origins().iter().enumerate().any(|(ordinal, origin)| {
                *origin != crate::registry_invocation::InvocationWordOrigin::Written(ordinal)
            })
        {
            return None;
        }
        let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
            advice,
        ) = words.source()
        else {
            return None;
        };
        if advice.logical_source_input() != Some(input) || advice.original_words() != producer {
            return None;
        }
        let selected = words.with_source_schema(&context, |schema| {
            (schema.semantics.analyser_hook == Some(tcl_registry::hooks::AnalyserHookId::Apply))
                .then(|| schema.authored_source_lambda_call())
                .flatten()
        })??;
        let lambda = words
            .original_argument_word(selected.lambda_argument)?
            .clone();
        let body = words.original_lambda_body_geometry(&context)?;
        if body.original_container() != &lambda {
            return None;
        }
        let fields = purpose.logical_list_inputs(
            &lambda,
            tcl_syntax::word_rules::WordValueRules::from_config(&input.lexer_config()),
        )?;
        let namespace = SignatureNamespaceScope::from_logical_lambda_list(&fields)?;
        let formals = crate::signature_scan::formal_parameters::LogicalSourceFormalParameters::from_lambda_fields(
            &fields, crate::command_binding::source_input_dialect(input),
        ).map(Arc::new);
        Some(Self {
            input: input.clone(),
            words: Arc::new(words),
            producer: producer.to_vec(),
            lambda,
            content: body.content_span(),
            body: Arc::new(body),
            namespace,
            fields,
            formals,
        })
    }

    fn current_advice(
        &self,
        input: &ResolvedAnalysisInput,
    ) -> Option<&crate::command_binding::OriginalSourceCommandTransitionAdvice> {
        if self.input != *input
            || !input.has_logical_source_name_context()
            || self.lambda.config() != input.lexer_config()
            || self.producer.is_empty()
            || self.producer.iter().any(|word| {
                word.image() != self.lambda.image() || word.config() != input.lexer_config()
            })
        {
            return None;
        }
        let context = input.context_registry();
        if !self
            .source_words()
            .matches_source(self.lambda.image(), input.lexer_config())
            || !self.words.matches_registry(context.commands())
            || self.words.context() != Some(context.context())
        {
            return None;
        }
        let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
            advice,
        ) = self.words.source()
        else {
            return None;
        };
        if advice.logical_source_input() != Some(input) || advice.original_words() != self.producer
        {
            return None;
        }
        let body = self.words.original_lambda_body_geometry(&context)?;
        if body != *self.body
            || body.original_container() != &self.lambda
            || body.content_span() != self.content
            || SignatureNamespaceScope::from_logical_lambda_list(&self.fields).as_ref()
                != Some(&self.namespace)
        {
            return None;
        }
        Some(advice)
    }

    pub(crate) fn matches_source(
        &self,
        image: &SourceImage,
        input: &ResolvedAnalysisInput,
    ) -> bool {
        self.lambda.image() == image && self.current_advice(input).is_some()
    }

    pub(crate) fn frame_source(
        &self,
        input: &ResolvedAnalysisInput,
    ) -> Option<(
        Arc<crate::command_binding::SourceOriginId>,
        u32,
        Span,
        SignatureNamespaceScope,
    )> {
        let advice = self.current_advice(input)?;
        let context = input.context_registry();
        let call = self
            .words
            .with_source_schema(&context, |schema| schema.authored_source_lambda_call())??;
        let formals = self.formals.as_deref()?;
        if call.count.indeterminate {
            return None;
        }
        crate::signature_scan::formal_count::logical_formal_argument_bindings(
            formals.parameter_field().value(),
            formals.parameters(),
            usize::from(call.count.minimum),
        )?;
        Some((
            Arc::clone(&advice.site().source),
            advice.site().offset,
            self.content,
            self.namespace.clone(),
        ))
    }

    pub(crate) fn original_words(&self) -> &[NativeWord] {
        &self.producer
    }

    pub(crate) fn lambda(&self) -> &NativeWord {
        &self.lambda
    }
    pub(crate) fn body(&self) -> &Arc<crate::registry_invocation::OriginalSourceScriptBody> {
        &self.body
    }
    pub(crate) fn namespace(&self) -> &SignatureNamespaceScope {
        &self.namespace
    }
    pub(crate) fn source_words(&self) -> &OriginalRegistryWords {
        &self.words
    }
    pub(crate) fn formal_parameters(
        &self,
    ) -> Option<&crate::signature_scan::formal_parameters::LogicalSourceFormalParameters> {
        self.formals.as_deref()
    }
    pub(crate) fn namespace_field(
        &self,
    ) -> Option<&crate::signature_scan::name_value::LogicalSourceListNameInput> {
        self.fields.get(2)
    }
}
