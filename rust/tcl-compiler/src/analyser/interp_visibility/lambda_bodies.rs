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

//! The source namespace of a literal lambda body under a child declaration.

use super::super::{Analyser, ResolvedAnalysisInput};
use super::{OriginalInterpreterSourceDomain, SourceInterpreterVisibilitySnapshot};
use crate::signature_scan::scope::SignatureNamespaceScope;
use std::sync::Arc;
use tcl_lexer::{NativeWord, SourceImage, Span, Token};

/// A literal lambda's original source namespace under its child declaration.
/// This retains source materialisation and future-entry assumptions; it is not
/// an entered lambda, native frame or namespace allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct OriginalInterpreterLambdaBody {
    pub(super) input: ResolvedAnalysisInput,
    pub(super) lambda: NativeWord,
    pub(super) producer: Vec<NativeWord>,
    content: Span,
    pub(super) namespace: SignatureNamespaceScope,
    domain: Option<Arc<OriginalInterpreterSourceDomain>>,
    pub(super) logical: Option<Arc<crate::command_binding::OriginalLogicalLambdaBody>>,
}

impl OriginalInterpreterLambdaBody {
    fn from_logical_source(
        input: &ResolvedAnalysisInput,
        words: crate::registry_invocation::source_structure::OriginalRegistryWords,
        producer: &[NativeWord],
        purpose: crate::signature_scan::original_name::OriginalSourceNamePurpose,
    ) -> Option<Self> {
        let logical = Arc::new(crate::command_binding::OriginalLogicalLambdaBody::capture(
            input, words, producer, purpose,
        )?);
        Some(Self {
            input: input.clone(),
            lambda: logical.lambda().clone(),
            producer: producer.to_vec(),
            content: logical.body().content_span(),
            namespace: logical.namespace().clone(),
            domain: None,
            logical: Some(logical),
        })
    }

    pub(in super::super) fn logical_source_owner(
        &self,
    ) -> Option<&Arc<crate::command_binding::OriginalLogicalLambdaBody>> {
        self.logical.as_ref()
    }

    pub(in super::super) fn has_logical_source_namespace(&self) -> bool {
        self.logical.is_some()
    }

    pub(in super::super) fn logical_formals_for_body_token(
        &self,
        source: &str,
        input: &ResolvedAnalysisInput,
        token: Token,
        parameter_token: Token,
    ) -> Option<&crate::signature_scan::formal_parameters::LogicalSourceFormalParameters> {
        self.for_body_token(source, input, token)?;
        let formals = self.logical.as_ref()?.formal_parameters()?;
        let field = formals.parameter_field();
        let content = field.source_span()?;
        if parameter_token
            .span
            .start()
            .checked_add(u32::from(parameter_token.content_offset))
            != Some(content.start())
            || parameter_token.span.end() != content.end()
        {
            return None;
        }
        Some(formals)
    }

    /// Require both token-derived list elements to retain the exact selected
    /// original parameter and body fields under this complete input.
    pub(in super::super) fn for_lambda_tokens(
        &self,
        source: &str,
        input: &ResolvedAnalysisInput,
        parameter_token: Token,
        body_token: Token,
    ) -> Option<(
        SignatureNamespaceScope,
        Option<Arc<OriginalInterpreterSourceDomain>>,
    )> {
        let selected = self.for_body_token(source, input, body_token)?;
        let parameter_span =
            crate::lambda_literal::split_original_lambda_literal(&self.lambda)?.params;
        if parameter_token
            .span
            .start()
            .checked_add(u32::from(parameter_token.content_offset))
            != Some(parameter_span.start())
            || parameter_token.span.end() != parameter_span.end()
        {
            return None;
        }
        Some(selected)
    }

    pub(in super::super) fn logical_namespace_field_for_body_token(
        &self,
        source: &str,
        input: &ResolvedAnalysisInput,
        token: Token,
    ) -> Option<&crate::signature_scan::name_value::LogicalSourceListNameInput> {
        self.for_body_token(source, input, token)?;
        self.logical.as_ref()?.namespace_field()
    }

    pub(in super::super) fn for_body_token(
        &self,
        source: &str,
        input: &ResolvedAnalysisInput,
        token: Token,
    ) -> Option<(
        SignatureNamespaceScope,
        Option<Arc<OriginalInterpreterSourceDomain>>,
    )> {
        if self.input != *input
            || self.lambda.image() != &SourceImage::document(source)
            || self.lambda.config() != input.lexer_config()
            || self.producer.is_empty()
            || token.kind != tcl_lexer::TokenType::Str
            || token
                .span
                .start()
                .checked_add(u32::from(token.content_offset))
                != Some(self.content.start())
            || token.span.end() != self.content.end()
        {
            return None;
        }
        if let Some(logical) = &self.logical
            && (!logical.matches_source(&SourceImage::document(source), input)
                || logical.lambda() != &self.lambda
                || logical.body().content_span() != self.content
                || logical.namespace() != &self.namespace)
        {
            return None;
        }
        Some((self.namespace.clone(), self.domain.clone()))
    }
}

impl SourceInterpreterVisibilitySnapshot {
    pub(super) fn literal_lambda_body(
        &self,
        invocation: &[NativeWord],
        producer: &[NativeWord],
        namespace: &SignatureNamespaceScope,
        domain: Option<Arc<OriginalInterpreterSourceDomain>>,
    ) -> Option<OriginalInterpreterLambdaBody> {
        let schema = self.possible_source_schema(invocation, namespace)?;
        if schema.hook != Some(tcl_registry::hooks::AnalyserHookId::Apply) {
            return None;
        }
        let ordinal = schema
            .roles
            .iter()
            .find(|(_, role)| *role == tcl_registry::ArgRole::LambdaLiteral)?
            .0;
        let lambda = invocation.get(ordinal.checked_add(1)?)?;
        let input = self.original_input(lambda)?;
        let fields = crate::lambda_literal::split_original_lambda_literal(lambda)?;
        let content = fields.braced_body()?;
        input.original_list_element(0)?;
        input.original_list_element(1)?;
        if input.original_list_element(3).is_some()
            || producer.is_empty()
            || producer.iter().any(|word| {
                word.image() != self.image() || word.config() != self.input.lexer_config()
            })
        {
            return None;
        }
        let path = match input.original_list_element(2) {
            Some(original) => self
                .policy
                .recipe()
                .lambda_namespace_path(original.bytes())
                .ok()?,
            None => tcl_core_types::ByteNamespacePath::root(),
        };
        Some(OriginalInterpreterLambdaBody {
            input: self.input.clone(),
            lambda: lambda.clone(),
            producer: producer.to_vec(),
            content,
            namespace: SignatureNamespaceScope::C(path),
            domain,
            logical: None,
        })
    }
}

impl Analyser {
    pub(in super::super) fn original_interp_apply_body(
        &self,
        head: Token,
        arguments: &[Token],
        scope_path: &[usize],
    ) -> Option<OriginalInterpreterLambdaBody> {
        let Some(snapshot) = self.safe_interp_stack.last() else {
            return self.original_logical_apply_namespace(head, arguments, scope_path);
        };
        snapshot
            .matches(
                &SourceImage::document(&self.source),
                &self.resolved_analysis_input(),
            )
            .then_some(())?;
        let mut argv = vec![head];
        argv.extend_from_slice(arguments);
        let native = self.original_visibility_words(&argv)?;
        let namespace = self.declaration_namespace_scope(scope_path)?;
        let domain = super::super::scope::scope_at(&self.result.global_scope, scope_path)?
            .original_interpreter_source_domain
            .clone();
        snapshot.literal_lambda_body(&native, &native, &namespace, domain)
    }

    fn original_logical_apply_namespace(
        &self,
        head: Token,
        arguments: &[Token],
        scope_path: &[usize],
    ) -> Option<OriginalInterpreterLambdaBody> {
        let input = self.resolved_analysis_input();
        input.has_logical_source_name_context().then_some(())?;
        let scope = super::super::scope::scope_at(&self.result.global_scope, scope_path)?;
        if scope.original_interpreter_source_domain.is_some() || !self.safe_interp_stack.is_empty()
        {
            return None;
        }
        let mut argv = vec![head];
        argv.extend_from_slice(arguments);
        let native = self.original_visibility_words(&argv)?;
        let tokens = self.retained_invocation_tokens(head.span.start(), &argv)?;
        let context = input.context_registry();
        let advice = self
            .head_identities
            .original_source_transition_advice(&context, &tokens)?;
        let words =
            crate::registry_invocation::source_structure::source_transition_words_from_advice(
                &self.source,
                input.lexer_config(),
                &context,
                advice,
            )?;
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                &input,
                self.source_analysis_entry.as_deref(),
                self.declaration_name_policy()?,
            );
        OriginalInterpreterLambdaBody::from_logical_source(&input, words, &native, purpose)
    }
}
