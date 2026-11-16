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

//! What body assistance answers: caller templates, parameter roles, traits and lambda owners.

use super::invocation_assistance::mapped_written_argument_roles;
use super::source_scoped_body::OriginalSourceScriptBody;
use super::{
    BodyParameterSourceInvocation, CallerTemplateSource, ConditionalParameterSourceAliasLayout,
    ConditionalParameterSourceInvocation, InvocationBodyAssistance, InvocationWordOrigin,
    LogicalLambdaSourceOwner, LogicalTraitInvocation, ParameterRoleAdvice, RegistryInvocationShape,
    ResolvedStatementInvocation, original_literal_argument_data, source_structure,
};
use crate::ir::CommandTokens;

impl ParameterRoleAdvice {
    pub(crate) fn is_closed(&self) -> bool {
        self.closed
    }

    pub(crate) fn candidates(&self) -> &[RegistryInvocationShape] {
        &self.candidates
    }

    pub(crate) fn written_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        self.candidates
            .iter()
            .flat_map(RegistryInvocationShape::written_argument_roles)
            .collect()
    }

    pub(crate) fn may_execute_body(&self) -> bool {
        self.candidates
            .iter()
            .any(RegistryInvocationShape::may_execute_parameter_body)
    }
}

impl BodyParameterSourceInvocation {
    fn matches_parameters(&self, parameters: &[&str]) -> bool {
        self.entry.matches_parameters(parameters)
    }
}

impl<'a> ConditionalParameterSourceInvocation<'a> {
    pub(crate) fn written_argument_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        mapped_written_argument_roles(
            &self.invocation.effective,
            self.facts.argument_offset,
            &self.shape.roles,
        )
    }

    /// The original data-only invocation for the existing body-owner-checked
    /// alias/template readers. Its ordinary facts are unchanged.
    pub(crate) fn retained_invocation(&self) -> &'a ResolvedStatementInvocation {
        self.invocation
    }
}

impl ConditionalParameterSourceAliasLayout {
    pub(crate) fn into_option(self) -> Option<(tcl_registry::FrameLevel, usize)> {
        match self {
            Self::Unavailable => None,
            Self::Available(layout) => Some(layout),
        }
    }
}

impl InvocationBodyAssistance {
    /// Conditional roles from the authentic enclosing Logical lambda source.
    /// This projection never populates ordinary parameter advice.
    pub(crate) fn conditional_logical_lambda_roles(
        &self,
    ) -> Option<Vec<(usize, tcl_registry::ArgRole)>> {
        let owned = self.logical_lambda_roles.as_ref()?;
        let source_structure::OriginalRegistrySource::SourceTransitions(advice) =
            owned.words.source()
        else {
            return None;
        };
        (owned
            .lambda
            .matches_source(owned.lambda.lambda().image(), &owned.input)
            && owned.scope.matches_input(&owned.input)
            && advice.logical_source_input() == Some(&owned.input)
            && owned.invocation.effective == owned.shape.effective)
            .then(|| owned.shape.written_argument_roles())
    }

    pub(crate) fn conditional_logical_lambda_shape(&self) -> Option<&RegistryInvocationShape> {
        let owned = self.logical_lambda_roles.as_ref()?;
        let source_structure::OriginalRegistrySource::SourceTransitions(advice) =
            owned.words.source()
        else {
            return None;
        };
        (owned
            .lambda
            .matches_source(owned.lambda.lambda().image(), &owned.input)
            && owned.scope.matches_input(&owned.input)
            && advice.logical_source_input() == Some(&owned.input)
            && owned.invocation.effective == owned.shape.effective)
            .then_some(&owned.shape)
    }

    /// Retain the selected nested lambda under the same source owner and
    /// ancestry as the current logical body.
    pub(crate) fn logical_trait_caller_lambda_owner(
        &self,
        argument: usize,
    ) -> Option<LogicalLambdaSourceOwner> {
        let (input, words, parent_scope) = if let Some(owned) = &self.logical_trait_invocation {
            (
                &owned.input,
                &owned.words,
                std::sync::Arc::clone(owned.body.scope()),
            )
        } else {
            let owned = self.logical_lambda_roles.as_ref()?;
            (
                &owned.input,
                &owned.words,
                std::sync::Arc::clone(&owned.scope),
            )
        };
        let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
            advice,
        ) = words.source()
        else {
            return None;
        };
        let policy = crate::command_binding::source_input_dialect(input).authored_name_policy()?;
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                input, None, policy,
            );
        let lambda =
            std::sync::Arc::new(crate::command_binding::OriginalLogicalLambdaBody::capture(
                input,
                words.clone(),
                advice.original_words(),
                purpose,
            )?);
        let selected_word = words.original_argument_word(argument)?;
        if lambda.lambda() != selected_word {
            return None;
        }
        let scope = std::sync::Arc::new(
            crate::command_binding::OriginalDeclaredLogicalBodyContext::capture_lambda(
                lambda.clone(),
                input,
                Some(parent_scope),
            )?,
        );
        lambda.frame_source(input)?;
        Some(LogicalLambdaSourceOwner { lambda, scope })
    }

    pub(super) fn retain_caller_template_source(&mut self, tokens: &CommandTokens) {
        self.caller_template_source = self.caller_name_template_invocation().map(|invocation| {
            Box::new(CallerTemplateSource {
                tokens: tokens.clone(),
                invocation: invocation.clone(),
            })
        });
    }

    /// Literal data from this retained invocation's original Written operand
    /// or actual captured prefix value. No name, frame or execution follows.
    pub(crate) fn caller_template_literal_data(
        &self,
        selected: &ResolvedStatementInvocation,
        argument: usize,
    ) -> Option<String> {
        (self
            .caller_name_template_invocation()
            .is_some_and(|actual| std::ptr::eq(actual, selected))
            || self.conditional_parameter_source().is_some_and(|owned| {
                std::ptr::eq(std::ptr::from_ref(&owned.invocation), selected)
            })
            || self.body_parameter_source.as_ref().is_some_and(|owned| {
                std::ptr::eq(std::ptr::from_ref(&owned.invocation), selected)
            }))
        .then_some(())?;
        if let Some(owned) = &self.body_parameter_source
            && std::ptr::eq(std::ptr::from_ref(&owned.invocation), selected)
        {
            let InvocationWordOrigin::Written(written) =
                selected.effective.origins.get(argument.checked_add(1)?)?
            else {
                return None;
            };
            let word = owned.words.get(*written)?;
            let value = tcl_syntax::word_rules::original_static_word_unicode_value(word)?;
            return (!value.contains(&0))
                .then(|| String::from_utf8(value).ok())
                .flatten();
        }
        if let Some(owned) = &self.logical_trait_invocation
            && std::ptr::eq(std::ptr::from_ref(&owned.invocation), selected)
        {
            let word = owned.words.original_argument_word(argument)?;
            let value = tcl_syntax::word_rules::original_static_word_unicode_value(word)?;
            return (!value.contains(&0))
                .then(|| String::from_utf8(value).ok())
                .flatten();
        }
        let source = self.caller_template_source.as_deref()?;
        (&source.invocation == selected).then_some(())?;
        match selected.effective.origins.get(argument.checked_add(1)?)? {
            InvocationWordOrigin::Written(written) => {
                original_literal_argument_data(&source.tokens, written.checked_sub(1)?)
            }
            InvocationWordOrigin::BindingPrefix(_) => {
                let value = selected.evaluated_words.get(argument)?.literal_bytes()?;
                (!value.contains(&0)).then_some(())?;
                Some(std::str::from_utf8(value).ok()?.to_owned())
            }
            InvocationWordOrigin::ResolvedHead | InvocationWordOrigin::ExpandedElement { .. } => {
                None
            }
        }
    }

    pub(crate) fn parameter_role_advice(&self) -> &ParameterRoleAdvice {
        &self.parameter_advice
    }

    /// The own-body caller-name layout is a symbolic callee template. It
    /// identifies no caller cell and authorises no successful alias or store.
    pub(crate) fn caller_name_template_invocation(&self) -> Option<&ResolvedStatementInvocation> {
        self.definite_invocation
            .as_ref()
            .or_else(|| {
                self.conditional_invocation
                    .as_ref()
                    .map(|conditional| &conditional.invocation)
            })
            .or_else(|| {
                self.receiver_trait_invocation
                    .as_ref()
                    .map(|owned| &owned.invocation)
            })
            .or_else(|| {
                self.logical_trait_invocation
                    .as_ref()
                    .filter(|owned| owned.source_parameter_shape.is_none())
                    .map(|owned| &owned.invocation)
            })
    }

    fn conditional_parameter_source(&self) -> Option<&LogicalTraitInvocation> {
        let owned = self.logical_trait_invocation.as_ref()?;
        owned.source_parameter_shape.as_ref()?;
        let source_structure::OriginalRegistrySource::SourceTransitions(advice) =
            owned.words.source()
        else {
            return None;
        };
        let prefix = owned
            .body
            .parameter_source_prefix(advice.original_words())?;
        prefix
            .matches_body(&owned.body, &owned.input)
            .then_some(())?;
        (prefix.current() == Some(advice.as_ref())).then_some(owned)
    }

    /// Exact conditional own-formal source shape. Original observer/lookup
    /// applicability stays retained on the body prefix, independently of the
    /// closed execution candidate advice and successful caller operations.
    pub(crate) fn conditional_parameter_source_shape(
        &self,
        parameters: &[&str],
    ) -> Option<&RegistryInvocationShape> {
        if self.logical_trait_invocation.is_none() {
            let owned = self.body_parameter_source.as_ref()?;
            return owned.matches_parameters(parameters).then_some(&owned.shape);
        }
        let owned = self.conditional_parameter_source()?;
        owned
            .body
            .parameters()
            .iter()
            .map(|formal| formal.name.as_str())
            .eq(parameters.iter().copied())
            .then_some(())?;
        owned.source_parameter_shape.as_ref()
    }

    /// Source-only symbolic operand layout. This is never returned by the
    /// generic parameter/caller-template invocation or definite accessors.
    pub(crate) fn conditional_parameter_source_invocation(
        &self,
        parameters: &[&str],
    ) -> Option<ConditionalParameterSourceInvocation<'_>> {
        let shape = self.conditional_parameter_source_shape(parameters)?;
        let invocation = if self.logical_trait_invocation.is_some() {
            &self.conditional_parameter_source()?.invocation
        } else {
            &self.body_parameter_source.as_ref()?.invocation
        };
        Some(ConditionalParameterSourceInvocation {
            facts: &invocation.facts,
            invocation,
            shape,
        })
    }

    /// Conditional source alias layout from the same retained Registry source
    /// schema. None means this is not that purpose; Unavailable preserves a
    /// missing layout without falling through to Native success rules.
    pub(crate) fn conditional_parameter_source_alias_layout(
        &self,
        selected: &ResolvedStatementInvocation,
    ) -> Option<ConditionalParameterSourceAliasLayout> {
        let owned = self.conditional_parameter_source()?;
        std::ptr::eq(std::ptr::from_ref(&owned.invocation), selected).then_some(())?;
        Some(
            owned
                .words
                .with_source_schema(owned.input.borrowed_context_registry(), |schema| {
                    let aliases = schema.authored_logical_source_variable_aliases()?;
                    if let Some(first) = aliases
                        .iter()
                        .filter_map(|alias| {
                            matches!(
                                alias.target,
                                tcl_registry::VariableAliasTarget::Namespace { .. }
                            )
                            .then_some(alias.words.target)
                        })
                        .min()
                    {
                        return Some((tcl_registry::FrameLevel::Dynamic, first));
                    }
                    (schema.semantics.frame_effect?.layout
                        == tcl_registry::FrameArgLayout::AliasPairs)
                        .then_some(())?;
                    match schema.authored_logical_source_frame_argument_layout()? {
                        tcl_registry::frame_effect::FrameArgumentResolution::Valid {
                            level,
                            level_word_len,
                        } => Some((level, level_word_len)),
                        tcl_registry::frame_effect::FrameArgumentResolution::Unknown
                        | tcl_registry::frame_effect::FrameArgumentResolution::Invalid => None,
                    }
                })
                .flatten()
                .map_or(
                    ConditionalParameterSourceAliasLayout::Unavailable,
                    ConditionalParameterSourceAliasLayout::Available,
                ),
        )
    }

    /// The actual original descriptor decides whether this script operand
    /// inherits the caller source context. A deferred/fresh-frame body cannot
    /// donate its locals to the enclosing procedure's parameter template.
    pub(crate) fn logical_trait_caller_body(
        &self,
        argument: usize,
    ) -> Option<OriginalSourceScriptBody> {
        let (words, input) = if let Some(owned) = &self.logical_trait_invocation {
            (&owned.words, &owned.input)
        } else {
            let owned = self.logical_lambda_roles.as_ref()?;
            (&owned.words, &owned.input)
        };
        let word = words.original_argument_word(argument)?;
        let context = input.borrowed_context_registry();
        let mut bodies = words
            .source_script_bodies(context)
            .into_iter()
            .filter(|body| body.original_container() == word);
        let body = bodies.next()?;
        (bodies.next().is_none() && body.inherits_immediate_caller_source_context(context))
            .then_some(body)
    }

    /// Exact source lambda body selected by this same Logical invocation.
    /// This retains only authored list geometry; it does not establish an
    /// entered lambda frame or parameter binding.
    pub(crate) fn logical_trait_caller_lambda(
        &self,
        argument: usize,
    ) -> Option<OriginalSourceScriptBody> {
        let (words, input) = if let Some(owned) = &self.logical_trait_invocation {
            (&owned.words, &owned.input)
        } else {
            let owned = self.logical_lambda_roles.as_ref()?;
            (&owned.words, &owned.input)
        };
        let word = words.original_argument_word(argument)?;
        let context = input.borrowed_context_registry();
        let body = words.original_lambda_body_geometry(context)?;
        (body.original_container() == word
            && body.matches_source(word.image(), input.lexer_config()))
        .then_some(body)
    }

    /// Symbolic local-copy and alias traits use the declaration's original
    /// formal recipe. Symbolic caller-name roles may use it too; actual caller
    /// cells and effects require entered-frame proof and `definite_invocation`.
    pub(crate) fn parameter_invocation(
        &self,
        parameters: &[&str],
    ) -> Option<&ResolvedStatementInvocation> {
        self.definite_invocation
            .as_ref()
            .or_else(|| {
                let conditional = self.conditional_invocation.as_ref()?;
                conditional
                    .entry
                    .matches_parameters(parameters)
                    .then_some(&conditional.invocation)
            })
            .or_else(|| {
                let owned = self.receiver_trait_invocation.as_ref()?;
                owned
                    .entry
                    .matches_trait_parameters(parameters)
                    .then_some(&owned.invocation)
            })
            .or_else(|| {
                let owned = self.logical_trait_invocation.as_ref()?;
                if owned.source_parameter_shape.is_some() {
                    return None;
                }
                let formal_names = owned
                    .body
                    .parameters()
                    .iter()
                    .map(|formal| formal.name.as_str())
                    .collect::<Vec<_>>();
                (formal_names == parameters).then_some(&owned.invocation)
            })
    }
}
