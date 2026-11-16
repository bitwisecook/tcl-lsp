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

//! Body assistance for the parameters of Logical procedures and lambdas.

use super::body_candidates::{body_assistance_context, invocation_body_assistance};
use super::effective_words::compose_original_effective_words;
use super::handler_invocation::resolved_declared_assistance;
use super::metadata_context::InvocationMetadataContext;
use super::source_scoped_body::OriginalSourceScriptBody;
use super::{
    ConditionalLogicalLambdaRoles, InvocationBodyAssistance, InvocationWordOrigin,
    LogicalLambdaSourceOwner, LogicalTraitInvocation, ParameterRoleAdvice, RegistryInvocationShape,
    ResolvedStatementInvocation, source_structure,
};
use crate::ir::CommandTokens;
use tcl_registry::InvocationFacts;

struct LogicalParameterSourceFacts {
    pub(super) facts: InvocationFacts,
    pub(super) source_roles: Option<(Vec<(u8, tcl_registry::ArgRole)>, bool)>,
    named_only_roles: bool,
}

fn logical_parameter_source_facts(
    words: &source_structure::OriginalRegistryWords,
    context: &tcl_registry::model::ContextRegistry,
    conditional_source: bool,
    named_only: bool,
) -> Option<LogicalParameterSourceFacts> {
    let (facts, source_roles) = words.with_source_schema(context, |schema| {
        (
            schema.facts().clone(),
            conditional_source.then(|| schema.authored_logical_source_argument_roles()),
        )
    })?;
    let named_only_roles = named_only && facts.arg_roles_complete;
    if facts.arity_accepts_frozen_arguments() != Some(true)
        || (!named_only_roles && !source_roles.as_ref().is_some_and(|(_, complete)| *complete))
    {
        return None;
    }
    Some(LogicalParameterSourceFacts {
        facts,
        source_roles,
        named_only_roles,
    })
}

fn logical_parameter_origins_are_unwritten(
    words: &source_structure::OriginalRegistryWords,
) -> bool {
    words
        .origins()
        .iter()
        .enumerate()
        .any(|(ordinal, origin)| *origin != InvocationWordOrigin::Written(ordinal))
}

/// Offset-free syntax assistance joins the retained lookup worlds. It has no
/// original invocation point and therefore never returns definite grammar,
/// entered scope or state-bearing alias/value-copy evidence.
pub(crate) fn logical_parameter_body_assistance(
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    realm: &crate::realm::CommandBindingRealm,
    body: &std::sync::Arc<crate::command_binding::OriginalLogicalProcedureBody>,
    source: &str,
    command: &crate::segmenter::SegmentedCommand,
    base: u32,
) -> Option<Box<InvocationBodyAssistance>> {
    let input = realm
        .source_bindings_ref()
        .original_logical_source_name_advice_input()?;
    let config = input.lexer_config();
    let image = realm.original_source_image()?;
    let context = input.borrowed_context_registry();
    if !realm.matches_resolved_analysis_input(input)
        || !body.matches_source(image, input)
        || body.has_earlier_unknown_mutations()
        || !body
            .scope()
            .scope_body()
            .matches_original_content(source, base, config)
        || context.commands().snapshot().semantic_key()
            != surface.commands().snapshot().semantic_key()
    {
        return None;
    }
    let segment = command.clone().shifted_by(base);
    let full_source = image.try_text().ok()?;
    let words =
        source_structure::logical_source_registry_words(full_source, realm, input, &segment)?;
    let source_structure::OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
        return None;
    };
    let scope = advice.logical_source_scope()?;
    let conditional_source = body
        .parameter_source_prefix(advice.original_words())
        .is_some_and(|prefix| {
            prefix.matches_body(body, input) && prefix.current() == Some(advice.as_ref())
        });
    let named_only = advice.has_named_only_logical_summary_descriptor();
    if (!named_only && !conditional_source)
        || scope.frame_recipe()
            != crate::command_binding::OriginalDeclaredSourceBodyFrame::Procedure
        || scope.scope_body() != body.scope().scope_body()
    {
        return None;
    }
    let LogicalParameterSourceFacts {
        facts,
        source_roles,
        named_only_roles,
    } = logical_parameter_source_facts(&words, context, conditional_source, named_only)?;
    let mut tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::from_image(image), config, &segment);
    realm.stamp_original_tokens(&mut tokens);
    // This named-only source descriptor has no alias or expanded prefix. Its
    // canonical Registry label selects facts; original Written operands retain
    // their own geometry and unknown values.
    if logical_parameter_origins_are_unwritten(&words) {
        return None;
    }
    let effective = compose_original_effective_words(&tokens, words.command(), &[])?;
    let evaluated_words = words.arguments().to_vec();
    let invocation = ResolvedStatementInvocation {
        dialect: words.dialect(),
        facts: Box::new(facts),
        arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
        evaluated_arguments: evaluated_words
            .iter()
            .map(|word| {
                word.literal_bytes()
                    .and_then(|value| String::from_utf8(value.to_vec()).ok())
            })
            .collect(),
        evaluated_words,
        effective,
        authored_case_body_selection: None,
    };
    let mut view = unpositioned_body_assistance(
        surface,
        realm.source_bindings_ref(),
        source,
        config,
        command,
        Some(context),
    )?;
    let mut shape =
        RegistryInvocationShape::from_facts(&invocation.facts, invocation.effective.clone());
    let source_parameter_shape = if named_only_roles {
        view.parameter_advice = ParameterRoleAdvice {
            candidates: vec![shape],
            closed: true,
        };
        None
    } else {
        // Only this separate shape consumes the source-role consensus. Keep
        // ordinary InvocationFacts roles/completeness and all generic closed
        // advice unchanged, including unavailable execution frame grammar.
        let (roles, complete) = source_roles?;
        shape.roles = roles;
        shape.roles_complete = complete;
        view.parameter_advice = ParameterRoleAdvice::default();
        Some(shape)
    };
    view.logical_trait_invocation = Some(LogicalTraitInvocation {
        source_parameter_shape,
        input: input.clone(),
        body: std::sync::Arc::clone(body),
        words,
        invocation,
    });
    view.logical_lambda_roles = None;
    // Other execution, Native conditional-entry and definite fields retain
    // their independent meaning; only this authenticated source template is added.
    Some(view)
}

/// Source-only argument roles for one actual command inside a selected
/// Logical lambda. The separate role facet never enters ordinary invocation
/// advice or establishes lambda entry.
// naming.compiler.logical-lambda-parameter-trait-source-roles
// docs/design/analysis/name-resolution-proofs/logical-lambda-parameter-trait-source-roles.md
pub(crate) fn logical_lambda_parameter_body_assistance(
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    realm: &crate::realm::CommandBindingRealm,
    owner: &LogicalLambdaSourceOwner,
    selected_child_body: Option<&OriginalSourceScriptBody>,
    source: &str,
    command: &crate::segmenter::SegmentedCommand,
    base: u32,
) -> Option<Box<InvocationBodyAssistance>> {
    let input = realm
        .source_bindings_ref()
        .original_logical_source_name_advice_input()?;
    let image = realm.original_source_image()?;
    let context = input.borrowed_context_registry();
    let lambda = &owner.lambda;
    let scope = &owner.scope;
    let body = lambda.body();
    let source_scope = selected_child_body.unwrap_or(body);
    let segment = command.clone().shifted_by(base);
    if !realm.matches_resolved_analysis_input(input)
        || !lambda.matches_source(image, input)
        || !scope.matches_input(input)
        || !scope.lambda_source().is_some_and(|owner| owner == lambda)
        || !source_scope.matches_original_content(source, base, input.lexer_config())
        || !source_scope.matches_source(image, input.lexer_config())
        || !source_scope.matches_context(context)
        || source_scope.content_span().start() < body.content_span().start()
        || source_scope.content_span().end() > body.content_span().end()
        || segment.span.start() < source_scope.content_span().start()
        || segment.span.end() > source_scope.content_span().end()
        || context.commands().snapshot().semantic_key()
            != surface.commands().snapshot().semantic_key()
    {
        return None;
    }
    let full_source = image.try_text().ok()?;
    let words =
        source_structure::logical_source_registry_words(full_source, realm, input, &segment)?;
    let (facts, roles) = words.with_source_schema(context, |schema| {
        (
            schema.facts().clone(),
            schema.authored_logical_source_argument_roles(),
        )
    })?;
    let (roles, complete) = roles;
    if !complete
        || facts.arity_accepts_frozen_arguments() != Some(true)
        || words
            .origins()
            .iter()
            .enumerate()
            .any(|(ordinal, origin)| *origin != InvocationWordOrigin::Written(ordinal))
    {
        return None;
    }
    let mut tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(image),
        input.lexer_config(),
        &segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let effective = compose_original_effective_words(&tokens, words.command(), &[])?;
    let evaluated_words = words.arguments().to_vec();
    let invocation = ResolvedStatementInvocation {
        dialect: words.dialect(),
        facts: Box::new(facts),
        arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
        evaluated_arguments: evaluated_words
            .iter()
            .map(|word| {
                word.literal_bytes()
                    .and_then(|value| String::from_utf8(value.to_vec()).ok())
            })
            .collect(),
        evaluated_words,
        effective,
        authored_case_body_selection: None,
    };
    let mut shape =
        RegistryInvocationShape::from_facts(&invocation.facts, invocation.effective.clone());
    shape.roles = roles;
    shape.roles_complete = true;
    let mut view = unpositioned_body_assistance(
        surface,
        realm.source_bindings_ref(),
        source,
        input.lexer_config(),
        command,
        Some(context),
    )?;
    view.logical_lambda_roles = Some(ConditionalLogicalLambdaRoles {
        lambda: std::sync::Arc::clone(lambda),
        scope: std::sync::Arc::clone(scope),
        input: input.clone(),
        words,
        invocation,
        shape,
    });
    Some(view)
}

pub(crate) fn unpositioned_body_assistance(
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    bindings: &crate::command_binding::SourceCommandBindings,
    source: &str,
    config: tcl_lexer::LexerConfig,
    command: &crate::segmenter::SegmentedCommand,
    metadata: Option<&tcl_registry::model::ContextRegistry>,
) -> Option<Box<InvocationBodyAssistance>> {
    let registry = surface.commands();
    let mut tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, command);
    tokens.source_binding = Some(bindings.invocation_unpositioned(command.name()));
    bindings.attach_declared_body_assistance(&mut tokens, surface);
    let context = match metadata {
        Some(context) => Some(InvocationMetadataContext::from(context)),
        None => body_assistance_context(registry, &tokens).map(InvocationMetadataContext::from),
    };
    if context.is_some_and(|context| !context.matches_registry(registry)) {
        return None;
    }
    let Some(context) = context else {
        let declared = resolved_declared_assistance(surface, &tokens)?;
        return Some(Box::new(InvocationBodyAssistance {
            caller_template_source: None,
            possible_roles: declared.roles.clone(),
            declared_roles: declared.roles,
            definite_roles: Vec::new(),
            possible_traits: tcl_registry::Traits::empty(),
            definite_traits: tcl_registry::Traits::empty(),
            definite_scope: None,
            possible_definition_grammars: Vec::new(),
            possible_case_lists: Vec::new(),
            possible_operations: Vec::new(),
            definite_invocation: None,
            conditional_invocation: None,
            receiver_trait_invocation: None,
            logical_trait_invocation: None,
            logical_lambda_roles: None,
            body_parameter_source: None,
            unknown_residual: true,
            may_be_absent: true,
            parameter_advice: ParameterRoleAdvice::default(),
        }));
    };
    let mut view = invocation_body_assistance(registry, context, surface, &tokens);
    view.definite_roles.clear();
    view.definite_traits = tcl_registry::Traits::empty();
    view.definite_scope = None;
    view.definite_invocation = None;
    view.unknown_residual = true;
    view.may_be_absent = true;
    Some(view)
}
