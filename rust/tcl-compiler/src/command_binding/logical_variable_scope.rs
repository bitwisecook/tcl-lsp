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

//! Original Logical source naming scope, independent of Native variable selection.

use super::{
    OriginalDeclaredLogicalBodyContext, OriginalDeclaredSourceBodyFrame,
    OriginalLogicalProcedureBody, SourceOriginId, SourceOriginalVariableFrame,
};
use crate::analyser::ResolvedAnalysisInput;
use crate::signature_scan::scope::{
    SignatureNamespaceScope, SignatureSourceNameInput, SignatureSourceNameKey,
};
use std::sync::Arc;
use tcl_lexer::{NativeWord, SourceImage, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
enum LogicalVariableFrameRecipe {
    Document(Arc<SourceOriginId>),
    Declared {
        scope: Arc<OriginalDeclaredLogicalBodyContext>,
        procedure: Option<Arc<OriginalLogicalProcedureBody>>,
    },
    Formal(Arc<OriginalLogicalProcedureBody>),
    LambdaFormal(Arc<OriginalDeclaredLogicalBodyContext>),
}

/// Actual source scope and complete original interpretation. This is a naming
/// recipe, never a selected variable cell, entered frame or rename certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalLogicalVariableScope {
    input: ResolvedAnalysisInput,
    original: Arc<[NativeWord]>,
    frame: LogicalVariableFrameRecipe,
    earlier_unknown_mutations: bool,
}

impl OriginalLogicalVariableScope {
    pub(crate) fn shares_original_allocation(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.original, &other.original)
    }

    pub(crate) fn lambda_formal(
        input: &ResolvedAnalysisInput,
        scope: Arc<OriginalDeclaredLogicalBodyContext>,
        field: &crate::signature_scan::scope::OriginalLogicalListField,
    ) -> Option<Self> {
        let lambda = scope.lambda_source()?;
        if scope.frame_recipe() != OriginalDeclaredSourceBodyFrame::Lambda
            || !Arc::ptr_eq(scope.body_arc(), lambda.body())
            || !field.matches_lambda(lambda, input)
        {
            return None;
        }
        let advice = lambda.current_advice(input)?;
        let result = Self {
            input: input.clone(),
            original: Arc::from(lambda.original_words()),
            earlier_unknown_mutations: !advice.uncertain_operations().is_empty(),
            frame: LogicalVariableFrameRecipe::LambdaFormal(scope),
        };
        result
            .matches_name(
                field.source_span(),
                &SignatureSourceNameInput::OriginalLogicalListField(field.clone()),
            )
            .then_some(result)
    }

    pub(crate) fn formal(
        input: &ResolvedAnalysisInput,
        procedure: Arc<OriginalLogicalProcedureBody>,
        span: Span,
        name: &SignatureSourceNameInput,
    ) -> Option<Self> {
        let result = Self {
            input: input.clone(),
            original: Arc::from(std::slice::from_ref(procedure.formal_original())),
            earlier_unknown_mutations: procedure.has_earlier_unknown_mutations(),
            frame: LogicalVariableFrameRecipe::Formal(procedure),
        };
        result.matches_name(span, name).then_some(result)
    }
    pub(super) fn document(
        input: &ResolvedAnalysisInput,
        origin: Arc<SourceOriginId>,
        original: Arc<[NativeWord]>,
        earlier_unknown_mutations: bool,
        span: Span,
        name: &SignatureSourceNameInput,
    ) -> Option<Self> {
        let result = Self {
            input: input.clone(),
            original,
            frame: LogicalVariableFrameRecipe::Document(origin),
            earlier_unknown_mutations,
        };
        result.matches_name(span, name).then_some(result)
    }

    pub(super) fn declared(
        input: &ResolvedAnalysisInput,
        scope: Arc<OriginalDeclaredLogicalBodyContext>,
        procedure: Option<Arc<OriginalLogicalProcedureBody>>,
        original: Arc<[NativeWord]>,
        earlier_unknown_mutations: bool,
        span: Span,
        name: &SignatureSourceNameInput,
    ) -> Option<Self> {
        if scope.frame_recipe() == OriginalDeclaredSourceBodyFrame::Procedure {
            let card = procedure.as_ref()?;
            if !same_procedure_scope(card, &scope)
                || !card.matches_source(original.first()?.image(), input)
            {
                return None;
            }
        } else if procedure.is_some() {
            return None;
        }
        let result = Self {
            input: input.clone(),
            original,
            frame: LogicalVariableFrameRecipe::Declared { scope, procedure },
            earlier_unknown_mutations,
        };
        result.matches_name(span, name).then_some(result)
    }

    pub(crate) fn matches_source(
        &self,
        image: &SourceImage,
        input: &ResolvedAnalysisInput,
    ) -> bool {
        self.input == *input
            && input.has_logical_source_name_context()
            && super::source_analysis_entry::source_input_dialect(input)
                .authored_name_policy()
                .is_some_and(|policy| {
                    policy.authority()
                        == tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
                        && matches!(
                            policy.recipe(),
                            tcl_syntax::naming::NativeNameProtocol::C(_)
                        )
                })
            && !self.original.is_empty()
            && self
                .original
                .iter()
                .all(|word| word.image() == image && word.config() == input.lexer_config())
            && match &self.frame {
                LogicalVariableFrameRecipe::Document(origin) => origin.source_image() == image,
                LogicalVariableFrameRecipe::Formal(card) => {
                    card.matches_source(image, input)
                        && self.original.as_ref() == std::slice::from_ref(card.formal_original())
                }
                LogicalVariableFrameRecipe::LambdaFormal(scope) => {
                    scope.matches_input(input)
                        && scope.lambda_source().is_some_and(|lambda| {
                            Arc::ptr_eq(scope.body_arc(), lambda.body())
                                && lambda.matches_source(image, input)
                                && self.original.as_ref() == lambda.original_words()
                        })
                }
                LogicalVariableFrameRecipe::Declared { scope, procedure } => {
                    scope.matches_input(input)
                        && scope.owns_words(&self.original)
                        && procedure.as_ref().is_none_or(|card| {
                            same_procedure_scope(card, scope) && card.matches_source(image, input)
                        })
                }
            }
    }

    pub(crate) fn matches_name(&self, span: Span, name: &SignatureSourceNameInput) -> bool {
        let Some(image) = self.original.first().map(NativeWord::image) else {
            return false;
        };
        if !self.matches_source(image, &self.input)
            || super::source_analysis_entry::source_input_dialect(&self.input)
                .authored_name_policy()
                != Some(name.policy())
            || name.policy().authority()
                != tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
        {
            return false;
        }
        if let LogicalVariableFrameRecipe::Formal(card) = &self.frame {
            return card
                .formal_topology()
                .and_then(crate::command_binding::formal_topology::OriginalFormalTopology::original_name_fields)
                .is_some_and(|fields| {
                    fields
                        .iter()
                        .any(|field| field.span == span && &field.input == name)
                });
        }
        if let LogicalVariableFrameRecipe::LambdaFormal(scope) = &self.frame {
            return matches!(name, SignatureSourceNameInput::OriginalLogicalListField(field)
                if field.source_span() == span
                    && scope.lambda_source().is_some_and(|lambda| field.matches_lambda(lambda, &self.input)));
        }
        Self::original_words_match_name(&self.original, &self.input, span, name)
    }

    pub(super) fn original_words_match_name(
        original: &[NativeWord],
        input: &ResolvedAnalysisInput,
        span: Span,
        name: &SignatureSourceNameInput,
    ) -> bool {
        let Some(image) = original.first().map(NativeWord::image) else {
            return false;
        };
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&input.lexer_config());
        match name {
            SignatureSourceNameInput::OriginalLogicalListField(_) => false,
            SignatureSourceNameInput::OriginalWord(key) => original.iter().any(|word| {
                (word.span() == span
                    || word
                        .tokens()
                        .first()
                        .is_some_and(|token| token.span == span))
                    && SignatureSourceNameKey::from_original_logical_source_word(
                        word,
                        rules,
                        name.policy(),
                    )
                    .as_ref()
                        == Some(key)
            }),
            SignatureSourceNameInput::OriginalVariableRoot(root) => {
                root.source_span() == span
                    && root.source_image() == image
                    && root.lexer_config() == input.lexer_config()
                    && original.iter().any(|word| {
                        let arena = word.executable_parts();
                        let mut parts = arena.all_parts().filter(|part| {
                            matches!(part.part, tcl_lexer::ExecutablePart::Variable { .. })
                                && part.span == root.part_span()
                                && arena.source_span(part) == Some(span)
                        });
                        let Some(part) = parts.next() else {
                            return false;
                        };
                        let tcl_lexer::ExecutablePart::Variable { name, .. } = part.part else {
                            return false;
                        };
                        parts.next().is_none() && arena.bytes(name) == Some(root.bytes())
                    })
            }
            SignatureSourceNameInput::OriginalValue(value) => original.iter().any(|word| {
                (word.span() == span || word.tokens().first().is_some_and(|token| token.span == span))
                    && crate::signature_scan::scope::SignatureSourceNameValue::from_original_variable_operand_root(
                        word, rules, name.policy(),
                    ).as_ref() == Some(value)
            }),
        }
    }

    pub(crate) fn namespace(&self) -> SignatureNamespaceScope {
        SignatureNamespaceScope::C(match &self.frame {
            LogicalVariableFrameRecipe::Document(_) => tcl_core_types::ByteNamespacePath::root(),
            LogicalVariableFrameRecipe::Formal(card) => card.scope().namespace_recipe().clone(),
            LogicalVariableFrameRecipe::LambdaFormal(scope) => scope.namespace_recipe().clone(),
            LogicalVariableFrameRecipe::Declared { scope, .. } => scope.namespace_recipe().clone(),
        })
    }

    pub(crate) fn local_frame(&self) -> Option<SourceOriginalVariableFrame> {
        if let LogicalVariableFrameRecipe::LambdaFormal(scope) = &self.frame {
            return SourceOriginalVariableFrame::from_logical_lambda_source(
                Arc::clone(scope.lambda_source()?),
                &self.input,
            );
        }
        if let LogicalVariableFrameRecipe::Declared { scope, .. } = &self.frame
            && scope.frame_recipe() == OriginalDeclaredSourceBodyFrame::Lambda
        {
            return SourceOriginalVariableFrame::from_logical_lambda_source(
                Arc::clone(scope.lambda_source()?),
                &self.input,
            );
        }
        let card = match &self.frame {
            LogicalVariableFrameRecipe::Declared {
                procedure: Some(card),
                ..
            }
            | LogicalVariableFrameRecipe::Formal(card) => card,
            _ => return None,
        };
        SourceOriginalVariableFrame::from_logical_procedure_source(card, &self.input)
    }

    pub(crate) fn namespace_storage(&self) -> bool {
        match &self.frame {
            LogicalVariableFrameRecipe::Document(_) => true,
            LogicalVariableFrameRecipe::Formal(_) | LogicalVariableFrameRecipe::LambdaFormal(_) => {
                false
            }
            LogicalVariableFrameRecipe::Declared { scope, .. } => !matches!(
                scope.frame_recipe(),
                OriginalDeclaredSourceBodyFrame::Procedure
                    | OriginalDeclaredSourceBodyFrame::Lambda
            ),
        }
    }

    pub(crate) fn alias_frame(&self) -> Option<SourceOriginalVariableFrame> {
        if let Some(frame) = self.local_frame() {
            return Some(frame);
        }
        let LogicalVariableFrameRecipe::Declared { scope, .. } = &self.frame else {
            return None;
        };
        SourceOriginalVariableFrame::from_logical_namespace_source(scope, &self.input)
    }

    pub(crate) fn matches_alias_template(
        &self,
        span: Span,
        input: &SignatureSourceNameInput,
        receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
        alias: &crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt,
    ) -> bool {
        use crate::signature_scan::variable_symbol::{
            OriginalVariableAliasAdvice, SignatureSourceVariableOccurrence,
        };
        use tcl_syntax::naming::{
            NativeNameContext, NativeVariableInputForm, NativeVariableRootGeometry,
        };
        if !self.matches_name(span, input)
            || self.alias_frame().as_ref() != Some(alias.frame())
            || input.policy() != alias.target().policy()
            || span.start() < alias.invocation_offset()
        {
            return false;
        }
        let declaration = OriginalVariableAliasAdvice::from_receipt(alias);
        if span == declaration.span() {
            return input == declaration.original_name_input();
        }
        let Some(form) = receiver.input_form(input) else {
            return false;
        };
        let protocol = input.policy().recipe();
        let selected = match form {
            NativeVariableInputForm::Combined(bytes) => protocol.combined_variable_input(bytes),
            NativeVariableInputForm::Separate { root, element } => {
                protocol.separate_variable_input(root, element)
            }
        };
        if let NativeVariableRootGeometry::Local(simple) =
            protocol.variable_root_geometry(NativeNameContext::root(), selected.root().selected())
        {
            return &simple == alias.local()
                && (self.namespace_storage() || self.local_name_is_byte_unique(simple.as_bytes()));
        }
        SignatureSourceVariableOccurrence::from_logical_source(
            span,
            input.clone(),
            receiver,
            false,
            self.clone(),
            &[],
        )
        .is_some_and(|occurrence| occurrence.symbol() == alias.target())
    }

    pub(crate) fn local_name_is_byte_unique(&self, name: &[u8]) -> bool {
        let (parameters_text, original_parameters) = match &self.frame {
            LogicalVariableFrameRecipe::LambdaFormal(scope)
            | LogicalVariableFrameRecipe::Declared { scope, .. }
                if scope.frame_recipe() == OriginalDeclaredSourceBodyFrame::Lambda =>
            {
                let Some(formals) = scope
                    .lambda_source()
                    .and_then(|lambda| lambda.formal_parameters())
                else {
                    return false;
                };
                (formals.parameter_field().value(), formals.parameters())
            }
            LogicalVariableFrameRecipe::Declared {
                procedure: Some(card),
                ..
            }
            | LogicalVariableFrameRecipe::Formal(card) => {
                (card.parameters_text(), card.parameters())
            }
            _ => return false,
        };
        let Some(policy) =
            super::source_analysis_entry::source_input_dialect(&self.input).authored_name_policy()
        else {
            return false;
        };
        let Some(parameters) = super::logical_definition::static_formals(
            parameters_text,
            self.input.lexer_config().list_parse,
        ) else {
            return false;
        };
        if parameters != original_parameters {
            return false;
        }
        super::logical_definition::local_formals_are_unique(&parameters, policy)
            && !name.contains(&0)
    }
}

fn same_procedure_scope(
    card: &OriginalLogicalProcedureBody,
    scope: &OriginalDeclaredLogicalBodyContext,
) -> bool {
    std::ptr::eq(card.scope().scope_body(), scope.scope_body())
        && card.scope().scope_name_input().is_some()
        && card.scope().scope_name_input() == scope.scope_name_input()
        && card.scope().namespace_recipe() == scope.namespace_recipe()
        && card.scope().frame_recipe() == scope.frame_recipe()
}
